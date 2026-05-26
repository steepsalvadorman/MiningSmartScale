#!/usr/bin/env bash
# WeighFlow IoT — Instalador para Linux (systemd)
# Uso: sudo ./install/install.sh
set -euo pipefail

# ── Constantes ─────────────────────────────────────────────────────────────────
BINARY="weighflow"
INSTALL_BIN="/usr/local/bin/${BINARY}"
INSTALL_CFG="/etc/${BINARY}"
INSTALL_DATA="/var/lib/${BINARY}"
SYSTEMD_UNIT="/etc/systemd/system/${BINARY}.service"
SERVICE_USER="${BINARY}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "${SCRIPT_DIR}")"

# ── Colores ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
ok()   { echo -e "  ${GREEN}✓${NC} $*"; }
warn() { echo -e "  ${YELLOW}!${NC} $*"; }
err()  { echo -e "  ${RED}✗${NC} $*"; }

# ── Verificaciones previas ─────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then
    err "Este instalador debe ejecutarse como root."
    echo "    Usa: sudo ${BASH_SOURCE[0]}"
    exit 1
fi

if ! command -v cargo &>/dev/null; then
    err "Rust/Cargo no encontrado. Instala Rust primero: https://rustup.rs"
    exit 1
fi

echo ""
echo "══════════════════════════════════════════════════"
echo "  WeighFlow IoT — Instalación"
echo "══════════════════════════════════════════════════"
echo ""

# ── Paso 1: Compilar binario release ──────────────────────────────────────────
echo "[1/6] Compilando WeighFlow IoT (release)..."
cd "${PROJECT_DIR}"
cargo build --release --bin weighflow 2>&1
ok "Binario compilado: target/release/${BINARY}"

# ── Paso 2: Usuario del servicio ──────────────────────────────────────────────
echo ""
echo "[2/6] Configurando usuario del servicio..."
if ! id "${SERVICE_USER}" &>/dev/null; then
    useradd --system \
            --no-create-home \
            --shell /usr/sbin/nologin \
            --comment "WeighFlow IoT service" \
            "${SERVICE_USER}"
    ok "Usuario creado: ${SERVICE_USER}"
else
    ok "Usuario ya existe: ${SERVICE_USER}"
fi

# Acceso al puerto serial (dialout en Debian/Ubuntu, uucp en Arch/Fedora)
SERIAL_GROUP=""
for grp in dialout uucp lock; do
    if getent group "${grp}" &>/dev/null; then
        SERIAL_GROUP="${grp}"
        break
    fi
done

if [[ -n "${SERIAL_GROUP}" ]]; then
    usermod -aG "${SERIAL_GROUP}" "${SERVICE_USER}"
    ok "Añadido al grupo serial: ${SERIAL_GROUP}"
else
    warn "Grupo serial no encontrado. Verifica acceso a /dev/ttyS* manualmente."
fi

# ── Paso 3: Directorios ────────────────────────────────────────────────────────
echo ""
echo "[3/6] Creando directorios..."
mkdir -p "${INSTALL_CFG}" "${INSTALL_DATA}"
chown "${SERVICE_USER}:${SERVICE_USER}" "${INSTALL_DATA}"
chmod 755 "${INSTALL_DATA}"
ok "Datos del servicio: ${INSTALL_DATA}"
ok "Configuración:      ${INSTALL_CFG}"

# ── Paso 4: Binario ──────────────────────────────────────────────────────────
echo ""
echo "[4/6] Instalando binario..."
install -m 755 "${PROJECT_DIR}/target/release/${BINARY}" "${INSTALL_BIN}"
ok "Instalado: ${INSTALL_BIN}"

# ── Paso 5: Configuración ────────────────────────────────────────────────────
echo ""
echo "[5/6] Instalando configuración..."
CFG_TARGET="${INSTALL_CFG}/weighflow.toml"

if [[ ! -f "${CFG_TARGET}" ]]; then
    install -m 640 "${PROJECT_DIR}/weighflow.toml" "${CFG_TARGET}"
    chown "root:${SERVICE_USER}" "${CFG_TARGET}"
    ok "Configuración instalada: ${CFG_TARGET}"
    warn "IMPORTANTE: edita la clave HMAC antes de iniciar el servicio."
    warn "  Archivo: ${CFG_TARGET}"
    warn "  Campo:   hmac_key = \"<clave-secreta-unica>\""
else
    ok "Configuración existente conservada: ${CFG_TARGET}"
fi

# ── Paso 6: Servicio systemd ──────────────────────────────────────────────────
echo ""
echo "[6/6] Instalando servicio systemd..."
install -m 644 "${SCRIPT_DIR}/weighflow.service" "${SYSTEMD_UNIT}"
systemctl daemon-reload
systemctl enable "${BINARY}"

# Advertir si el puerto serial no está configurado
if grep -q '^# port' "${CFG_TARGET}" 2>/dev/null || ! grep -q '^port' "${CFG_TARGET}" 2>/dev/null; then
    warn "Puerto serial sin configurar — se usará auto-detección al iniciar."
fi

systemctl start "${BINARY}" || true
sleep 1

STATUS=$(systemctl is-active "${BINARY}" 2>/dev/null || echo "unknown")
if [[ "${STATUS}" == "active" ]]; then
    ok "Servicio activo"
else
    warn "Servicio no activo (status: ${STATUS}). Revisa la configuración."
    echo "      journalctl -u ${BINARY} -n 20 --no-pager"
fi

# ── Resumen ────────────────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════"
echo -e "  ${GREEN}WeighFlow IoT instalado correctamente${NC}"
echo "══════════════════════════════════════════════════"
echo "  API HTTP:  http://localhost:8080"
echo "  Eventos:   http://localhost:8080/events"
echo "  CSV:       http://localhost:8080/export/csv"
echo "  WS live:   ws://localhost:8080/live"
echo ""
echo "  Config:    ${CFG_TARGET}"
echo "  Datos:     ${INSTALL_DATA}"
echo ""
echo "  Logs:      journalctl -u ${BINARY} -f"
echo "  Estado:    systemctl status ${BINARY}"
echo "  Reiniciar: systemctl restart ${BINARY}"
echo "══════════════════════════════════════════════════"
echo ""

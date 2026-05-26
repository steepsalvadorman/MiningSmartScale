#!/usr/bin/env bash
# WeighFlow IoT — Desinstalador para Linux (systemd)
# Uso: sudo ./install/uninstall.sh
set -euo pipefail

BINARY="weighflow"
INSTALL_BIN="/usr/local/bin/${BINARY}"
INSTALL_CFG="/etc/${BINARY}"
INSTALL_DATA="/var/lib/${BINARY}"
SYSTEMD_UNIT="/etc/systemd/system/${BINARY}.service"
SERVICE_USER="${BINARY}"

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
ok()   { echo -e "  ${GREEN}✓${NC} $*"; }
warn() { echo -e "  ${YELLOW}!${NC} $*"; }

if [[ $EUID -ne 0 ]]; then
    echo -e "  ${RED}✗${NC} Este script debe ejecutarse como root (sudo ${BASH_SOURCE[0]})"
    exit 1
fi

echo ""
echo "══════════════════════════════════════════════════"
echo "  WeighFlow IoT — Desinstalación"
echo "══════════════════════════════════════════════════"
echo ""

# Detener y deshabilitar el servicio
if systemctl is-active "${BINARY}" &>/dev/null; then
    systemctl stop "${BINARY}"
    ok "Servicio detenido"
fi

if systemctl is-enabled "${BINARY}" &>/dev/null 2>&1; then
    systemctl disable "${BINARY}"
    ok "Servicio deshabilitado"
fi

# Eliminar la unidad systemd
if [[ -f "${SYSTEMD_UNIT}" ]]; then
    rm -f "${SYSTEMD_UNIT}"
    systemctl daemon-reload
    ok "Unidad systemd eliminada"
fi

# Eliminar binario
if [[ -f "${INSTALL_BIN}" ]]; then
    rm -f "${INSTALL_BIN}"
    ok "Binario eliminado: ${INSTALL_BIN}"
fi

# Eliminar configuración (preguntar)
if [[ -d "${INSTALL_CFG}" ]]; then
    read -r -p "  ¿Eliminar configuración en ${INSTALL_CFG}? [s/N] " answer
    if [[ "${answer,,}" == "s" ]]; then
        rm -rf "${INSTALL_CFG}"
        ok "Configuración eliminada: ${INSTALL_CFG}"
    else
        warn "Configuración conservada: ${INSTALL_CFG}"
    fi
fi

# Eliminar datos del servicio (preguntar)
if [[ -d "${INSTALL_DATA}" ]]; then
    read -r -p "  ¿Eliminar datos y CSV exportados en ${INSTALL_DATA}? [s/N] " answer
    if [[ "${answer,,}" == "s" ]]; then
        rm -rf "${INSTALL_DATA}"
        ok "Datos eliminados: ${INSTALL_DATA}"
    else
        warn "Datos conservados: ${INSTALL_DATA}"
    fi
fi

# Eliminar usuario del servicio (preguntar)
if id "${SERVICE_USER}" &>/dev/null; then
    read -r -p "  ¿Eliminar usuario del sistema '${SERVICE_USER}'? [s/N] " answer
    if [[ "${answer,,}" == "s" ]]; then
        userdel "${SERVICE_USER}"
        ok "Usuario eliminado: ${SERVICE_USER}"
    else
        warn "Usuario conservado: ${SERVICE_USER}"
    fi
fi

echo ""
echo "══════════════════════════════════════════════════"
echo -e "  ${GREEN}WeighFlow IoT desinstalado${NC}"
echo "══════════════════════════════════════════════════"
echo ""

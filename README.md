# WeighFlow IoT

**Lector inteligente de balanzas industriales via RS-232**
Conecta cualquier balanza al computador y captura los pesos automáticamente — sin configurar nada, sin instalar drivers especiales.

---

## ¿Qué hace?

WeighFlow lee los datos que envía la balanza por el puerto serial (RS-232) y los pone disponibles de tres maneras al mismo tiempo:

| Modo | ¿Para qué sirve? |
|---|---|
| 🖥️ **Dashboard web** | Ver el peso en tiempo real desde el navegador (`http://localhost:8080`) |
| ⌨️ **Modo teclado** | Escribe el peso directamente en Excel, SAP, o cualquier programa abierto — como un lector de código de barras |
| 📄 **Exportar CSV** | Descargar el registro completo del turno con un clic |

Cada lectura queda firmada digitalmente (HMAC-SHA256) para que no pueda ser alterada.

---

## Lo que necesitas

- Una balanza con salida RS-232 (cualquier marca: Mettler Toledo, Rinstrum, CAS, Ohaus…)
- Un cable RS-232 al puerto COM de la PC (o adaptador USB-Serial)
- Windows 10/11 **o** Linux
- Rust instalado (solo para compilar — ver abajo)

> **No necesitas saber programar.** El instalador lo hace todo.

---

## Instalación

### En Windows

1. Abre **PowerShell como Administrador**
   *(clic derecho en el menú inicio → "Windows PowerShell (Administrador)")*

2. Navega a la carpeta del proyecto:
   ```powershell
   cd C:\ruta\al\proyecto\weighflow
   ```

3. Permite ejecutar el script:
   ```powershell
   Set-ExecutionPolicy Bypass -Scope Process -Force
   ```

4. Ejecuta el instalador:
   ```powershell
   .\install\install.ps1
   ```

Al terminar verás esto:

```
==================================================
  WeighFlow IoT instalado correctamente
==================================================
  Dashboard:  http://localhost:8080
  Eventos:    http://localhost:8080/events
  CSV:        http://localhost:8080/export/csv
  WS live:    ws://localhost:8080/live

  Config:     C:\ProgramData\WeighFlow\weighflow.toml
  Datos:      C:\ProgramData\WeighFlow\exports

  Logs:       Visor de eventos > Aplicacion > WeighFlow
  Estado:     Get-Service WeighFlow
  Reiniciar:  Restart-Service WeighFlow
==================================================
```

WeighFlow ya está corriendo como servicio de Windows y se inicia automáticamente con el equipo.

---

### En Linux

1. Abre una terminal y ve a la carpeta del proyecto:
   ```bash
   cd /ruta/al/proyecto/weighflow
   ```

2. Ejecuta el instalador como root:
   ```bash
   sudo ./install/install.sh
   ```

Al terminar:

```
══════════════════════════════════════════════════
  WeighFlow IoT instalado correctamente
══════════════════════════════════════════════════
  API:     http://localhost:8080
  Config:  /etc/weighflow/weighflow.toml
  Logs:    journalctl -u weighflow -f
  Estado:  systemctl status weighflow
══════════════════════════════════════════════════
```

---

## Configurar el puerto de la balanza

Antes de usar, abre el archivo de configuración y ajusta el puerto:

**Windows** → `C:\ProgramData\WeighFlow\weighflow.toml`
**Linux** → `/etc/weighflow/weighflow.toml`

```toml
[serial]
port = "COM3"        # Windows: COM1, COM2, COM3…
# port = "/dev/ttyS0"  # Linux: ttyS0, ttyUSB0…
baud_rate = 9600

[security]
hmac_key = "pon-aqui-una-clave-secreta-unica"   # ← cambia esto

[server]
http_port = 8080
```

> **Consejo:** Si no sabes qué puerto es, deja `port` comentado (con `#`). WeighFlow buscará la balanza automáticamente.

Después de editar, reinicia el servicio:
- **Windows:** `Restart-Service WeighFlow`
- **Linux:** `sudo systemctl restart weighflow`

---

## El Dashboard — pantalla principal

Abre el navegador y ve a **`http://localhost:8080`**

```
╔══════════════════════════════════════════════════════════════════════╗
║  WeighFlow IoT                                        ● Conectado   ║
╠══════════════╦═══════════════════════════════════════════════════════╣
║              ║   Total: 147    Estables: 132   Movimiento: 15       ║
║   24.550     ║   Último: 24.550 kg                                  ║
║      kg      ╠═══════════════════════════════════════════════════════╣
║              ║  #    Hora      Peso        Unidad  Estado           ║
║  ✓ ESTABLE   ║  147  14:32:05  24.550      kg      ✓ Estable        ║
║              ║  146  14:31:58   0.000      kg      ✓ Estable        ║
║  ID: 147     ║  145  14:31:44  24.550      kg      ✓ Estable        ║
║  14:32:05    ║  144  14:31:29  23.800      kg      ~ Movimiento     ║
║  🔐 a3f9b2…  ║  143  14:31:21  24.550      kg      ✓ Estable        ║
║              ║                                                       ║
║              ║  [ ⬇ Descargar CSV del turno ]                       ║
╚══════════════╩═══════════════════════════════════════════════════════╝
```

- El **número grande** es el peso actual de la balanza en tiempo real.
- **ESTABLE** (verde) = el peso dejó de moverse y la lectura es confiable.
- **EN MOVIMIENTO** (amarillo) = la balanza sigue midiendo, espera que se estabilice.
- El **historial** muestra las últimas 100 lecturas de la sesión.
- El indicador **●** en la esquina cambia a rojo si pierde conexión con la balanza.

---

## Exportar el registro del turno a Excel

Desde el dashboard, haz clic en **"⬇ Descargar CSV del turno"**.

O abre directamente en el navegador:
```
http://localhost:8080/export/csv
```

El archivo descargado (`pesajes_20250526.csv`) se abre directo en Excel:

```
id  | fecha_hora          | timestamp_ms  | valor    | unidad | estable | hmac
----|---------------------|---------------|----------|--------|---------|--------
1   | 2025-05-26 08:15:03 | 1748247303000 | 24.55000 | kg     | true    | a3f9b2…
2   | 2025-05-26 08:15:11 | 1748247311000 |  0.00000 | kg     | true    | 8d21c4…
3   | 2025-05-26 08:15:44 | 1748247344000 | 24.55000 | kg     | true    | f17e90…
```

> El campo **hmac** es la firma digital de cada pesaje. Cualquier alteración al valor rompería esta firma — sirve como registro de auditoría.

---

## Modo teclado — escribir el peso en Excel sin tocar el teclado

WeighFlow puede escribir el peso directamente en la celda activa de Excel o cualquier otro programa, igual que un lector de código de barras.

Para activarlo, edita el archivo de configuración:

```toml
[wedge]
enabled    = true       # activar
separator  = "tab"      # "tab" = avanza a la siguiente celda
                        # "enter" = baja a la siguiente fila
                        # "tab_enter" = avanza celda y confirma
stable_only = true      # solo escribe cuando el peso está estable (recomendado)
clipboard   = false     # también copiar al portapapeles
min_interval_ms = 500   # mínimo 500ms entre escrituras
```

**Cómo usarlo:**
1. Activa `enabled = true` y reinicia el servicio.
2. Abre Excel y haz clic en la celda donde quieres el dato.
3. Pon la carga en la balanza.
4. Cuando se estabilice, WeighFlow escribe el peso automáticamente.
5. El cursor avanza a la siguiente celda (si usas `tab`).

```
Excel antes:           Excel después de poner 24.55 kg:
┌────────────┐         ┌────────────┬──────────┐
│    A    │  B  │      │    A    │    B    │
│ Muestra │ Peso│      │ Muestra │  24.55  │
│    1    │  ▌  │  →   │    1    │         │ ← cursor salta a B2
│    2    │     │      │    2    │   ▌     │
└─────────┴─────┘      └─────────┴─────────┘
```

---

## Consola — lo que ves al iniciar manualmente

Si quieres correr WeighFlow directamente en la terminal (sin servicio):

```bash
./weighflow
```

```
2025-05-26T14:30:00 INFO WeighFlow IoT v0.5 — iniciando
2025-05-26T14:30:00 INFO ─────────────────────────────────────────────────
2025-05-26T14:30:00 INFO Configuración cargada desde /etc/weighflow/weighflow.toml
2025-05-26T14:30:00 INFO Puerto detectado: /dev/ttyUSB0
2025-05-26T14:30:00 INFO [PARSER] Aprendiendo protocolo...
2025-05-26T14:30:01 INFO [PARSER] Protocolo detectado: CRLF
2025-05-26T14:30:01 INFO [SEALER] Iniciado — firmando eventos con HMAC-SHA256
2025-05-26T14:30:01 INFO   Dashboard:  http://localhost:8080/
2025-05-26T14:30:01 INFO   GET  http://localhost:8080/events
2025-05-26T14:30:01 INFO   GET  http://localhost:8080/export/csv
2025-05-26T14:30:01 INFO   WS   ws://localhost:8080/live
2025-05-26T14:30:01 INFO ─────────────────────────────────────────────────
2025-05-26T14:30:05 INFO PESO #1    │     24.550 kg │ ESTABLE
2025-05-26T14:30:11 INFO PESO #2    │      0.000 kg │ ESTABLE
2025-05-26T14:30:15 WARN PESO #3    │     23.800 kg │ EN MOVIMIENTO
2025-05-26T14:30:18 INFO PESO #4    │     24.550 kg │ ESTABLE
```

- Las líneas **INFO** (blancas) son lecturas estables.
- Las líneas **WARN** (amarillas) son lecturas en movimiento.

Para especificar el puerto manualmente:
```bash
./weighflow --port COM3              # Windows
./weighflow --port /dev/ttyUSB0     # Linux
```

Para usar un archivo de configuración en otra ubicación:
```bash
./weighflow --config /ruta/a/mi-config.toml
```

---

## Verificar que está funcionando

**Desde el navegador:**
```
http://localhost:8080/status
```

Respuesta si está bien:
```json
{
  "connected": true,
  "event_count": 47,
  "last_event": { "value": 24.55, "unit": "kg", "stable": true, ... }
}
```

Respuesta si la balanza no está enviando datos:
```json
{
  "connected": false,
  "event_count": 0,
  "last_event": null
}
```

---

## Problemas frecuentes

### "No se detectó ningún puerto serial"
- Verifica que el cable RS-232 está conectado.
- En Windows: abre el Administrador de dispositivos y busca "Puertos (COM y LPT)". Anota el número (ej. COM4) y ponlo en la configuración.
- En Linux: ejecuta `ls /dev/ttyS* /dev/ttyUSB*` para ver los puertos disponibles.

### "El peso en pantalla no cambia"
- Verifica que la balanza está encendida y enviando datos.
- Prueba cambiar `baud_rate` en la configuración (valores comunes: 1200, 2400, 4800, 9600, 19200).
- Reinicia el servicio después de cambiar la configuración.

### "El modo teclado no escribe nada"
- Verifica que `enabled = true` está en la sección `[wedge]`.
- Comprueba que `stable_only = true` — el peso debe estabilizarse antes de escribir.
- Aumenta `min_interval_ms` si escribe demasiado rápido.

### "No puedo acceder a http://localhost:8080"
- Verifica que el servicio está corriendo:
  - Windows: `Get-Service WeighFlow`
  - Linux: `systemctl status weighflow`
- Si el puerto 8080 está ocupado, cambia `http_port` en la configuración.

---

## Comandos rápidos

### Windows (PowerShell)
```powershell
Get-Service WeighFlow              # Ver estado
Start-Service WeighFlow            # Iniciar
Stop-Service WeighFlow             # Detener
Restart-Service WeighFlow          # Reiniciar
```

### Linux
```bash
systemctl status weighflow         # Ver estado
sudo systemctl start weighflow     # Iniciar
sudo systemctl stop weighflow      # Detener
sudo systemctl restart weighflow   # Reiniciar
journalctl -u weighflow -f         # Ver logs en tiempo real
journalctl -u weighflow --since today  # Logs de hoy
```

---

## Desinstalar

**Windows:**
```powershell
.\install\uninstall.ps1
```

**Linux:**
```bash
sudo ./install/uninstall.sh
```

El desinstalador preguntará antes de borrar los datos y el historial de pesajes.

---

## Estructura del proyecto

```
weighflow/
├── src/                    Código fuente
│   ├── main.rs             Punto de entrada
│   ├── parser/             Parser RS-232 (auto-detecta protocolo)
│   ├── sealer.rs           Firmado HMAC-SHA256
│   ├── api/                Servidor HTTP + WebSocket
│   ├── export.rs           Exportación CSV
│   ├── wedge.rs            Modo teclado virtual
│   └── config.rs           Carga de configuración
├── static/
│   └── dashboard.html      Dashboard web (embebido en el binario)
├── install/
│   ├── install.sh          Instalador Linux
│   ├── install.ps1         Instalador Windows
│   ├── uninstall.sh        Desinstalador Linux
│   ├── uninstall.ps1       Desinstalador Windows
│   └── weighflow.service   Unidad systemd
├── tests/
│   ├── integration_test.rs Pruebas del pipeline completo
│   └── robustness_test.rs  Pruebas de condiciones de campo
└── weighflow.toml          Configuración por defecto
```

---

*WeighFlow IoT — CodeCraft Perú*

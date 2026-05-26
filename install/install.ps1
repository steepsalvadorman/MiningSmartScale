#Requires -RunAsAdministrator
# WeighFlow IoT — Instalador para Windows (PowerShell)
# Ejecutar en PowerShell como Administrador:
#   Set-ExecutionPolicy Bypass -Scope Process -Force
#   .\install\install.ps1
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ServiceName = "WeighFlow"
$InstallDir  = "$env:ProgramFiles\WeighFlow"
$InstallBin  = "$InstallDir\weighflow.exe"
$InstallCfg  = "$env:ProgramData\WeighFlow"
$InstallData = "$env:ProgramData\WeighFlow\exports"
$CfgTarget   = "$InstallCfg\weighflow.toml"

$ProjectDir  = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

function Write-Ok   { param($msg) Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "  [!]  $msg" -ForegroundColor Yellow }
function Write-Err  { param($msg) Write-Host "  [x]  $msg" -ForegroundColor Red; exit 1 }

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  WeighFlow IoT -- Instalacion (Windows)"          -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# ── Verificar Cargo/Rust ───────────────────────────────────────────────────────
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Err "Cargo/Rust no encontrado. Instala Rust desde https://rustup.rs"
}

# ── Paso 1: Compilar release ───────────────────────────────────────────────────
Write-Host "[1/5] Compilando WeighFlow IoT (release)..."
Push-Location $ProjectDir
cargo build --release --bin weighflow
if ($LASTEXITCODE -ne 0) { Write-Err "Fallo al compilar. Revisa los errores anteriores." }
Pop-Location
Write-Ok "Binario compilado: target\release\weighflow.exe"

# ── Paso 2: Directorios y binario ─────────────────────────────────────────────
Write-Host ""
Write-Host "[2/5] Instalando binario..."
New-Item -ItemType Directory -Force -Path $InstallDir  | Out-Null
New-Item -ItemType Directory -Force -Path $InstallData | Out-Null
Copy-Item "$ProjectDir\target\release\weighflow.exe" -Destination $InstallBin -Force
Write-Ok "Instalado: $InstallBin"

# ── Paso 3: Configuracion ─────────────────────────────────────────────────────
Write-Host ""
Write-Host "[3/5] Instalando configuracion..."
if (-not (Test-Path $CfgTarget)) {
    Copy-Item "$ProjectDir\weighflow.toml" -Destination $CfgTarget -Force

    # Ajustar output_dir al directorio de datos absoluto para el servicio
    $exportPath = $InstallData.Replace('\', '/')
    $content    = Get-Content $CfgTarget -Raw
    $content    = $content -replace 'output_dir\s*=\s*"\."', "output_dir = `"$exportPath`""
    Set-Content $CfgTarget $content -Encoding UTF8

    Write-Ok "Configuracion instalada: $CfgTarget"
    Write-Warn "IMPORTANTE: edita la clave HMAC antes de iniciar el servicio."
    Write-Warn "  Archivo: $CfgTarget"
    Write-Warn "  Campo:   hmac_key = `"<clave-secreta-unica>`""
} else {
    Write-Ok "Configuracion existente conservada: $CfgTarget"
}

# ── Paso 4: Servicio Windows ──────────────────────────────────────────────────
Write-Host ""
Write-Host "[4/5] Registrando servicio Windows..."

$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Warn "Servicio ya existe — actualizando..."
    if ($existing.Status -eq "Running") {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 2
}

$binPath = "`"$InstallBin`" --config `"$CfgTarget`""
sc.exe create $ServiceName `
    binpath= $binPath `
    start=   auto `
    obj=     LocalSystem `
    DisplayName= "WeighFlow IoT" | Out-Null

sc.exe description $ServiceName `
    "WeighFlow IoT -- Adquisicion RS-232 para balanzas industriales" | Out-Null

# Recuperacion automatica: reiniciar 3 veces si falla (cada 5 segundos)
sc.exe failure $ServiceName `
    reset=   60 `
    actions= restart/5000/restart/5000/restart/5000 | Out-Null

Write-Ok "Servicio registrado: $ServiceName (inicio automatico)"

# ── Paso 5: Iniciar servicio ──────────────────────────────────────────────────
Write-Host ""
Write-Host "[5/5] Iniciando servicio..."
Start-Service -Name $ServiceName -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2

$status = (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue)?.Status
if ($status -eq "Running") {
    Write-Ok "Servicio activo"
} else {
    Write-Warn "Servicio no activo (status: $status)"
    Write-Warn "Revisa: Visor de eventos > Registros de Windows > Aplicacion"
    Write-Warn "  o ejecuta manualmente: & `"$InstallBin`" --config `"$CfgTarget`""
}

# ── Resumen ───────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  WeighFlow IoT instalado correctamente"           -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  Dashboard:  http://localhost:8080"
Write-Host "  Eventos:    http://localhost:8080/events"
Write-Host "  CSV:        http://localhost:8080/export/csv"
Write-Host "  WS live:    ws://localhost:8080/live"
Write-Host ""
Write-Host "  Config:     $CfgTarget"
Write-Host "  Exportados: $InstallData"
Write-Host ""
Write-Host "  Logs:       Visor de eventos > Aplicacion > WeighFlow"
Write-Host "  Estado:     Get-Service WeighFlow"
Write-Host "  Reiniciar:  Restart-Service WeighFlow"
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

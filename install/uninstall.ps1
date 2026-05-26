#Requires -RunAsAdministrator
# WeighFlow IoT — Desinstalador para Windows (PowerShell)
# Ejecutar en PowerShell como Administrador:
#   Set-ExecutionPolicy Bypass -Scope Process -Force
#   .\install\uninstall.ps1
Set-StrictMode -Version Latest
$ErrorActionPreference = "SilentlyContinue"

$ServiceName = "WeighFlow"
$InstallDir  = "$env:ProgramFiles\WeighFlow"
$InstallCfg  = "$env:ProgramData\WeighFlow"

function Write-Ok   { param($msg) Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn { param($msg) Write-Host "  [!]  $msg" -ForegroundColor Yellow }

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  WeighFlow IoT -- Desinstalacion (Windows)"       -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# Detener y eliminar el servicio
$svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($svc) {
    if ($svc.Status -eq "Running") {
        Stop-Service -Name $ServiceName -Force
        Start-Sleep -Seconds 2
        Write-Ok "Servicio detenido"
    }
    sc.exe delete $ServiceName | Out-Null
    Write-Ok "Servicio eliminado: $ServiceName"
} else {
    Write-Warn "El servicio '$ServiceName' no estaba instalado"
}

# Eliminar binario
if (Test-Path $InstallDir) {
    Remove-Item $InstallDir -Recurse -Force
    Write-Ok "Binario eliminado: $InstallDir"
}

# Eliminar configuracion y datos (preguntar)
if (Test-Path $InstallCfg) {
    $answer = Read-Host "  Eliminar configuracion y datos en '$InstallCfg'? [s/N]"
    if ($answer -match '^[sS]$') {
        Remove-Item $InstallCfg -Recurse -Force
        Write-Ok "Configuracion y datos eliminados: $InstallCfg"
    } else {
        Write-Warn "Conservados: $InstallCfg"
    }
}

Write-Host ""
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host "  WeighFlow IoT desinstalado"                      -ForegroundColor Green
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

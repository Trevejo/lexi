$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

$exePath = Join-Path $scriptDir "target\release\lexi-win.exe"

$gnuExePath = Join-Path $scriptDir "target\x86_64-pc-windows-gnu\release\lexi-win.exe"
if (-not (Test-Path $exePath) -and (Test-Path $gnuExePath)) {
    New-Item -ItemType Directory -Force -Path (Join-Path $scriptDir "target\release") | Out-Null
    Copy-Item $gnuExePath $exePath
}

if (-not (Test-Path $exePath)) {
    Write-Host "[AVISO] Executavel ainda nao compilado. Executando build..." -ForegroundColor Yellow
    & (Join-Path $scriptDir "build.ps1")
}

if (-not (Test-Path $exePath)) {
    Write-Host "[ERRO] Executavel target\release\lexi-win.exe nao foi encontrado." -ForegroundColor Red
    Read-Host "Pressione Enter para fechar..."
    exit 1
}

$cfgPath = Join-Path $scriptDir "config.toml"
if (-not (Test-Path $cfgPath)) {
    Copy-Item (Join-Path $scriptDir "config.example.toml") $cfgPath
}

Write-Host "Iniciando Lexi em background..." -ForegroundColor Green
Start-Process $exePath
Write-Host "Lexi esta ativo na bandeja do sistema (proximo ao relogio)!" -ForegroundColor Cyan

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "  Compilando Lexi - Tradutor Gamer Nativo para Windows" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host ""

$cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoCmd) {
    Write-Host "[ERRO] O comando 'cargo' nao foi encontrado no Windows." -ForegroundColor Red
    Write-Host "Se acabou de instalar o Rust, feche este PowerShell e abra novamente."
    Write-Host "Caso nao tenha instalado, baixe em: https://rustup.rs/"
    Read-Host "Pressione Enter para fechar..."
    exit 1
}

Write-Host "Compilando em modo Release otimizado..." -ForegroundColor Yellow
cargo build --release --bin lexi-win

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "[SUCESSO] Compilacao concluida!" -ForegroundColor Green
    Write-Host "Executavel gerado em: target\release\lexi-win.exe" -ForegroundColor Green
    if (-not (Test-Path "config.toml")) {
        Copy-Item "config.example.toml" "config.toml"
        Write-Host "Arquivo config.toml criado a partir do modelo."
    }
} else {
    Write-Host "[ERRO] Falha durante a compilacao." -ForegroundColor Red
}

Read-Host "Pressione Enter para continuar..."

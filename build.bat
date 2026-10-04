@echo off
pushd "%~dp0"
echo ========================================================
echo   Compilando Lexi - Tradutor Gamer Nativo para Windows
echo ========================================================
echo.

where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERRO] O comando 'cargo' nao foi encontrado no PATH do Windows!
    echo.
    echo Se voce acabou de instalar o Rust:
    echo 1. Feche TODAS as janelas de terminal abertas.
    echo 2. Abra novamente este arquivo build.bat para que o novo PATH seja carregado.
    echo.
    echo Se ainda nao instalou o Rust:
    echo Baixe e execute o instalador oficial em: https://rustup.rs/
    echo.
    popd
    pause
    exit /b 1
)

echo Executando: cargo build --release --bin lexi-win
cargo build --release --bin lexi-win

if %ERRORLEVEL% equ 0 (
    echo.
    echo ========================================================
    echo   [SUCESSO] Compilacao concluida com exito!
    echo   Executavel gerado em: target\release\lexi-win.exe
    echo ========================================================
    if not exist "%~dp0config.toml" (
        echo Criando config.toml...
        copy "%~dp0config.example.toml" "%~dp0config.toml" >nul
    )
    echo.
    echo Agora voce pode dar dois cliques no 'run.bat' para iniciar!
) else (
    echo.
    echo ========================================================
    echo   [ERRO] Ocorreu uma falha durante a compilacao.
    echo   Leia as mensagens de erro acima para mais detalhes.
    echo ========================================================
)

popd
pause

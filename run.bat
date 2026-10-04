@echo off
pushd "%~dp0"

if not exist "%~dp0target\release\lexi-win.exe" (
    if exist "%~dp0target\x86_64-pc-windows-gnu\release\lexi-win.exe" (
        if not exist "%~dp0target\release" mkdir "%~dp0target\release"
        copy "%~dp0target\x86_64-pc-windows-gnu\release\lexi-win.exe" "%~dp0target\release\lexi-win.exe" >nul
    )
)

if not exist "%~dp0target\release\lexi-win.exe" (
    echo ========================================================
    echo   [AVISO] O executavel target\release\lexi-win.exe
    echo   ainda nao foi compilado!
    echo.
    echo   Executando a compilacao com build.bat...
    echo ========================================================
    echo.
    call "%~dp0build.bat"
)

if not exist "%~dp0target\release\lexi-win.exe" (
    echo.
    echo ========================================================
    echo   [ERRO] A compilacao nao gerou o executavel.
    echo   Verifique as mensagens de erro exibidas pelo compilador.
    echo ========================================================
    popd
    pause
    exit /b 1
)

if not exist "%~dp0config.toml" (
    echo Criando config.toml a partir do config.example.toml...
    copy "%~dp0config.example.toml" "%~dp0config.toml" >nul
)

echo Iniciando Lexi em segundo plano...
start "" "%~dp0target\release\lexi-win.exe"
echo.
echo ========================================================
echo   [OK] Lexi iniciado com sucesso!
echo   Ele esta rodando na bandeja do sistema
echo   (perto do relogio, na setinha de icones ocultos).
echo.
echo   Clique com o botao direito no icone para testar
echo   o overlay com a palavra 'flank' ou abrir as configuracoes.
echo ========================================================
popd
timeout /t 5 >nul

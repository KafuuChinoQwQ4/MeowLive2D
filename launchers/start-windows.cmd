@echo off
setlocal
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0start-windows.ps1" %*
set "meow_exit=%errorlevel%"
if not "%meow_exit%"=="0" pause
exit /b %meow_exit%

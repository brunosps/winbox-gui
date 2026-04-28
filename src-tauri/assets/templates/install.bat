@echo off
setlocal
rem winbox OEM bootstrap — roda no fim do unattend do Windows
mkdir C:\winbox 2>nul
copy /Y C:\OEM\firstlogon.ps1 C:\winbox\firstlogon.ps1
if exist C:\OEM\authorized_keys copy /Y C:\OEM\authorized_keys C:\winbox\authorized_keys

rem Agenda firstlogon.ps1 para rodar no primeiro login via RunOnce
reg add "HKLM\Software\Microsoft\Windows\CurrentVersion\RunOnce" /v winbox /t REG_SZ /d "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\winbox\firstlogon.ps1" /f
endlocal

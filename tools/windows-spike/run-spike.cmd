@echo off
REM Double-click this file to launch winbox-spike.ps1 elevated as Administrator.
REM A UAC prompt will appear; accept it.

powershell.exe -Command "Start-Process powershell.exe -Verb RunAs -ArgumentList '-NoExit','-ExecutionPolicy','Bypass','-File','%~dp0winbox-spike.ps1'"

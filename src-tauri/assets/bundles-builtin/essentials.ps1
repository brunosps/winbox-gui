# winbox bundle: essentials — utilitários mínimos (sempre aplicado)
Write-Host "[essentials] 7-Zip"
winget install -e --id 7zip.7zip --silent --accept-package-agreements --accept-source-agreements

Write-Host "[essentials] Notepad++"
winget install -e --id Notepad++.Notepad++ --silent --accept-package-agreements --accept-source-agreements

Write-Host "[essentials] VC++ Redist 2015+"
winget install -e --id Microsoft.VCRedist.2015+.x64 --silent --accept-package-agreements --accept-source-agreements

Write-Host "[essentials] PowerShell 7"
winget install -e --id Microsoft.PowerShell --silent --accept-package-agreements --accept-source-agreements

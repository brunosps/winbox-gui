# winbox bundle: dev — ferramentas de desenvolvimento
Write-Host "[dev] Git"
winget install -e --id Git.Git --silent --accept-package-agreements --accept-source-agreements

Write-Host "[dev] Visual Studio Code"
winget install -e --id Microsoft.VisualStudioCode --silent --accept-package-agreements --accept-source-agreements

Write-Host "[dev] Node.js LTS"
winget install -e --id OpenJS.NodeJS.LTS --silent --accept-package-agreements --accept-source-agreements

Write-Host "[dev] Python 3.12"
winget install -e --id Python.Python.3.12 --silent --accept-package-agreements --accept-source-agreements

Write-Host "[dev] Windows Terminal"
winget install -e --id Microsoft.WindowsTerminal --silent --accept-package-agreements --accept-source-agreements

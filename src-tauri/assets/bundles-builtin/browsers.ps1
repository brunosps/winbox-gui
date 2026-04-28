# winbox bundle: browsers — Chrome + Firefox
Write-Host "[browsers] Google Chrome"
winget install -e --id Google.Chrome --silent --accept-package-agreements --accept-source-agreements

Write-Host "[browsers] Mozilla Firefox"
winget install -e --id Mozilla.Firefox --silent --accept-package-agreements --accept-source-agreements

# winbox bundle: office — LibreOffice (alternativa grátis ao MS Office)
Write-Host "[office] LibreOffice"
winget install -e --id TheDocumentFoundation.LibreOffice --silent --accept-package-agreements --accept-source-agreements

Write-Host "[office] Adobe Acrobat Reader"
winget install -e --id Adobe.Acrobat.Reader.64-bit --silent --accept-package-agreements --accept-source-agreements

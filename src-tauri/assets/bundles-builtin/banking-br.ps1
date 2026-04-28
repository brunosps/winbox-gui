# winbox bundle: banking-br — internet banking brasileiro
# (muitos bancos BR requerem Java e funcionam melhor no Chrome)
Write-Host "[banking-br] Google Chrome (melhor compat com IB)"
winget install -e --id Google.Chrome --silent --accept-package-agreements --accept-source-agreements

Write-Host "[banking-br] Temurin 21 JRE (Java para módulos de segurança)"
winget install -e --id EclipseAdoptium.Temurin.21.JRE --silent --accept-package-agreements --accept-source-agreements

Write-Host "[banking-br] Adobe Acrobat Reader (para extratos/boletos)"
winget install -e --id Adobe.Acrobat.Reader.64-bit --silent --accept-package-agreements --accept-source-agreements

# NOTE: Módulos específicos (Warsaw do Itaú, Guardião Santander) não estão no winget.
# Instale manualmente pelos sites oficiais após o primeiro login.

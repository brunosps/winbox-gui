# winbox bundle: media — VLC, OBS, Spotify
Write-Host "[media] VLC"
winget install -e --id VideoLAN.VLC --silent --accept-package-agreements --accept-source-agreements

Write-Host "[media] OBS Studio"
winget install -e --id OBSProject.OBSStudio --silent --accept-package-agreements --accept-source-agreements

Write-Host "[media] Spotify"
winget install -e --id Spotify.Spotify --silent --accept-package-agreements --accept-source-agreements

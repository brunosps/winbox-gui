# winbox firstlogon — gerado automaticamente no install
# Este script roda no primeiro login via HKLM\...\RunOnce

$logPath = "C:\winbox\install.log"
New-Item -ItemType Directory -Force -Path "C:\winbox" | Out-Null
Start-Transcript -Path $logPath -Append -ErrorAction SilentlyContinue | Out-Null

Write-Host "================================================================"
Write-Host " winbox firstlogon"
Write-Host " $(Get-Date)"
Write-Host "================================================================"

# ─── OpenSSH Server ──────────────────────────────────────────────────────
try {
  $cap = Get-WindowsCapability -Online -Name OpenSSH.Server* -ErrorAction Stop
  if ($cap.State -ne 'Installed') {
    Write-Host "[ssh] Instalando OpenSSH Server..."
    Add-WindowsCapability -Online -Name $cap.Name | Out-Null
  }
  Start-Service sshd -ErrorAction SilentlyContinue
  Set-Service -Name sshd -StartupType Automatic -ErrorAction SilentlyContinue
  New-NetFirewallRule -Name 'OpenSSH-Server-In-TCP' -DisplayName 'OpenSSH Server (sshd)' `
    -Enabled True -Direction Inbound -Protocol TCP -Action Allow -LocalPort 22 `
    -ErrorAction SilentlyContinue | Out-Null

  if (Test-Path "C:\winbox\authorized_keys") {
    $key = Get-Content "C:\winbox\authorized_keys" -Raw
    if ($key) {
      $sshDir = Join-Path $env:USERPROFILE ".ssh"
      New-Item -ItemType Directory -Force -Path $sshDir | Out-Null
      Set-Content -Path (Join-Path $sshDir "authorized_keys") -Value $key -NoNewline
      $admFile = "C:\ProgramData\ssh\administrators_authorized_keys"
      Set-Content -Path $admFile -Value $key -NoNewline
      icacls.exe $admFile /inheritance:r /grant "Administrators:F" /grant "SYSTEM:F" | Out-Null
      Write-Host "[ssh] Chave pública importada."
    }
  }
  Write-Host "[ssh] OpenSSH configurado."
} catch {
  Write-Warning "[ssh] Falhou: $_"
}

# ─── winget sanity check ─────────────────────────────────────────────────
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
  Write-Warning "[winget] Ausente. Em Win10 LTSC pode ser necessário instalar App Installer manualmente."
  Write-Warning "[winget] Bundles dependentes de winget serão pulados."
  $global:WINBOX_NO_WINGET = $true
} else {
  Write-Host "[winget] $(winget --version)"
}

# ─── Bundles ─────────────────────────────────────────────────────────────

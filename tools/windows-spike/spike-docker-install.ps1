# Windows Port spike phase 2: enable WSL2 + install Docker Desktop +
# attempt to run dockurr/windows inside.
#
# Run as Administrator. Will REBOOT in the middle (after wsl --install).
# After reboot, log back in and re-run this script: it detects what's
# already done and continues.
#
# Output appended to spike-results.txt next to the script.

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$out = Join-Path $here "spike-results.txt"

function Log($msg) {
    $stamp = Get-Date -Format 'u'
    $line = "[$stamp] $msg"
    Write-Host $line
    Add-Content -Path $out -Value $line
}

Log "=== Phase 2 starting ==="

# Step 1: WSL features
$wsl = Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Windows-Subsystem-Linux -ErrorAction SilentlyContinue
$vmp = Get-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -ErrorAction SilentlyContinue
if ($wsl.State -ne "Enabled" -or $vmp.State -ne "Enabled") {
    Log "Enabling WSL + VirtualMachinePlatform via 'wsl --install --no-distribution'..."
    wsl --install --no-distribution
    Log "WSL install kicked off. REBOOT NOW manually, then re-run this script."
    Log "After reboot you should see: wsl --status -> Default Version: 2"
    exit 0
}
Log "WSL features already enabled."

# Step 2: WSL2 default distro (Ubuntu) if not present
$distros = wsl --list --quiet 2>&1 | Where-Object { $_ -and -not $_.StartsWith("Distribui") -and -not $_.Contains("nenhuma distribui") }
if (-not $distros) {
    Log "Installing Ubuntu WSL distro..."
    wsl --install -d Ubuntu --no-launch
    Log "Ubuntu install kicked off. Wait for it to finish, then re-run."
    exit 0
}
Log ("WSL distros present: " + ($distros -join ", "))

# Step 3: Docker Desktop
$docker = Get-Command docker -ErrorAction SilentlyContinue
if (-not $docker) {
    Log "Installing Docker Desktop via winget..."
    winget install --id Docker.DockerDesktop -e --accept-package-agreements --accept-source-agreements
    Log "Docker Desktop installed. Start it manually from the Start menu, accept the terms, and re-run this script."
    exit 0
}
Log "docker binary present."

# Step 4: docker info
Log "Running docker info (will hang if Desktop engine is not started)..."
$info = docker info 2>&1
$info | ForEach-Object { Log ("  info: " + $_) }
if ($info -match "ERROR" -or $info -match "cannot connect") {
    Log "Docker engine not reachable. Start Docker Desktop and re-run."
    exit 0
}

# Step 5: pull + run dockurr/windows
Log "Pulling dockurr/windows:5.14..."
$pull = docker pull dockurr/windows:5.14 2>&1
$pull | ForEach-Object { Log ("  pull: " + $_) }

Log "Attempting docker run dockurr/windows for ~30s then stopping..."
$cid = (docker run -d --name spike-dockur dockurr/windows:5.14 2>&1).Trim()
Log ("  container id: " + $cid)
Start-Sleep -Seconds 30
Log "First 100 log lines from the inner container:"
docker logs spike-dockur 2>&1 | Select-Object -First 100 | ForEach-Object { Log ("    " + $_) }
docker rm -f spike-dockur 2>&1 | Out-Null

Log "=== Phase 2 finished ==="
Log "Verdict heuristics:"
Log "  - If inner logs show 'Starting Windows for Docker' and 'Booting Windows using QEMU' WITHOUT errors about /dev/kvm or 'no acceleration', the port is VIABLE."
Log "  - If logs show 'KVM not available' or 'tcg acceleration' fallback, dockurr/windows runs but in TCG (no hardware virt) -- usable but extremely slow."
Log "  - If logs show 'Operation not permitted' on /dev/kvm or 'KVM is required', the WSL2 kernel does not expose KVM and port is blocked."

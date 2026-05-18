# Windows Port spike: nested virtualization + Docker Desktop check.
#
# How to run inside the bancos VM (already RDP'd in):
#   1. Open PowerShell as Administrator.
#   2. cd Y:\        (or wherever dockur mounts the shared folder)
#   3. Set-ExecutionPolicy -Scope Process Bypass
#   4. .\spike-docker.ps1
#
# Output lands in Y:\spike-results.txt. No automatic install: the
# script reports what's possible; you decide whether to proceed.

$ErrorActionPreference = "Continue"
$report = @()

function Add-Line($line) {
    $script:report += $line
    Write-Host $line
}

Add-Line "=== winbox-gui Windows port spike: $(Get-Date -Format 'u') ==="
Add-Line ""

# 1. CPU virtualization extensions exposed to the guest?
Add-Line "## 1. CPU virtualization"
$cpu = Get-CimInstance Win32_Processor
Add-Line ("  Name: " + $cpu.Name)
Add-Line ("  VirtualizationFirmwareEnabled: " + $cpu.VirtualizationFirmwareEnabled)
Add-Line ("  VMMonitorModeExtensions: " + $cpu.VMMonitorModeExtensions)
Add-Line ("  SecondLevelAddressTranslationExtensions: " + $cpu.SecondLevelAddressTranslationExtensions)
$sysinfo = systeminfo | Select-String "Hyper-V"
Add-Line "  systeminfo Hyper-V block:"
$sysinfo | ForEach-Object { Add-Line ("    " + $_) }
Add-Line ""

# 2. Hyper-V feature present (Pro/Enterprise SKUs only)
Add-Line "## 2. Hyper-V availability"
$hv = Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V-All -ErrorAction SilentlyContinue
if ($hv) {
    Add-Line ("  Microsoft-Hyper-V-All: " + $hv.State)
} else {
    Add-Line "  Microsoft-Hyper-V-All: not available (Home SKU?)"
}
$wsl = Get-WindowsOptionalFeature -Online -FeatureName Microsoft-Windows-Subsystem-Linux -ErrorAction SilentlyContinue
if ($wsl) {
    Add-Line ("  WSL feature: " + $wsl.State)
} else {
    Add-Line "  WSL feature: not available"
}
$vmp = Get-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform -ErrorAction SilentlyContinue
if ($vmp) {
    Add-Line ("  VirtualMachinePlatform: " + $vmp.State)
} else {
    Add-Line "  VirtualMachinePlatform: not available"
}
Add-Line ""

# 3. Existing tooling
Add-Line "## 3. Existing tooling"
$winget = Get-Command winget -ErrorAction SilentlyContinue
Add-Line ("  winget: " + $(if ($winget) { "present" } else { "absent" }))
$docker = Get-Command docker -ErrorAction SilentlyContinue
Add-Line ("  docker: " + $(if ($docker) { "present" } else { "absent" }))
$wslExe = Get-Command wsl -ErrorAction SilentlyContinue
Add-Line ("  wsl.exe: " + $(if ($wslExe) { "present" } else { "absent" }))
if ($wslExe) {
    Add-Line "  wsl --status:"
    try {
        (wsl --status) | ForEach-Object { Add-Line ("    " + $_) }
    } catch {
        Add-Line ("    error: " + $_)
    }
}
Add-Line ""

# 4. Verdict heuristic
Add-Line "## 4. Verdict"
$nestedReady = $cpu.VirtualizationFirmwareEnabled -and $cpu.VMMonitorModeExtensions
if ($nestedReady) {
    Add-Line "  Nested virt looks ENABLED. Guest sees VT-x/AMD-V."
    Add-Line "  Next steps: install Docker Desktop (with WSL2 backend), then:"
    Add-Line "    docker pull dockurr/windows"
    Add-Line "    docker run --rm -it dockurr/windows"
    Add-Line "  If the inner container even attempts to boot QEMU, the port"
    Add-Line "  is viable. If it dies with EPERM on /dev/kvm or 'no acceleration',"
    Add-Line "  Docker Desktop's WSL2 is not exposing nested virt: port is blocked."
} else {
    Add-Line "  Nested virt is DISABLED in this VM."
    Add-Line "  The host QEMU launched bancos without nested=on. To enable:"
    Add-Line "    on the Linux host, edit the bancos compose template's QEMU"
    Add-Line "    ARGUMENTS to include '+vmx' (Intel) or '+svm' (AMD), e.g.:"
    Add-Line "      -cpu host,+vmx,kvm=off,hv_vendor_id=whatever"
    Add-Line "    Then restart bancos. Re-run this spike."
    Add-Line "  Without nested virt, Docker Desktop's WSL2 backend will refuse to start."
}

# Write report next to the script (works regardless of how dockur mounts /shared)
$outPath = Join-Path $PSScriptRoot "spike-results.txt"
$report | Out-File $outPath -Encoding UTF8
Add-Line ""
Add-Line ("Report saved to " + $outPath)
Add-Line "On the Linux host the file is ~/Windows/bancos/spike-results.txt"

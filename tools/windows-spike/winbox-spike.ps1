# winbox-gui Windows spike: install + configure + build + test.
#
# All-in-one, idempotent. Run once as Administrator from the shared
# folder. The script:
#   * Installs Git, Node LTS, Rust, WebView2, VS 2022 Build Tools
#     (only what is missing).
#   * Schedules itself in RunOnce + reboots when a feature requires it,
#     so re-running the script is automatic after login.
#   * Syncs the repo from the shared folder into C:\winbox-src.
#   * Patches tauri.conf.json so Windows produces an NSIS/MSI bundle.
#   * Runs `npm install`, `cargo test`, `npm run tauri build`.
#   * Writes logs + binaries back to the shared folder.
#
# Usage from RDP, PowerShell as Administrator:
#   cd C:\Users\bruno\Desktop\Shared
#   Set-ExecutionPolicy -Scope Process Bypass
#   .\winbox-spike.ps1               # full run; reboots when needed
#   .\winbox-spike.ps1 -SkipReboot   # don't reboot automatically; just log
#   .\winbox-spike.ps1 -Status       # report what's done, do nothing
#
# Outputs (shared folder):
#   spike.log              full timestamped log
#   test-results.txt       cargo test output
#   winbox-windows\        binaries + bundle/

[CmdletBinding()]
param(
    [switch]$SkipReboot,
    [switch]$Status
)

$ErrorActionPreference = "Continue"
$here = $PSScriptRoot
$log = Join-Path $here "spike.log"
$state = Join-Path $here "spike.state.json"
$srcShared = Join-Path $here "winbox-src"
$srcLocal = "C:\winbox-src"
$artifactDir = Join-Path $here "winbox-windows"

# ---------------- helpers ----------------

function Log($msg) {
    $stamp = Get-Date -Format 'u'
    $line = "[$stamp] $msg"
    Write-Host $line
    Add-Content -Path $log -Value $line
}
function Has-Cmd($name) { [bool](Get-Command $name -ErrorAction SilentlyContinue) }
function Refresh-Path {
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
                [System.Environment]::GetEnvironmentVariable("Path","User")
}
function Load-State {
    if (Test-Path $state) {
        try { return Get-Content $state -Raw | ConvertFrom-Json }
        catch { return @{} }
    }
    return @{ phase = "start"; reboot_pending = $false }
}
function Save-State($s) {
    $s | ConvertTo-Json -Depth 5 | Out-File $state -Encoding UTF8
}
function Schedule-AutoResume {
    $runOnce = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\RunOnce"
    if (-not (Test-Path $runOnce)) { New-Item -Path $runOnce -Force | Out-Null }
    $cmd = "powershell -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    Set-ItemProperty -Path $runOnce -Name "winbox-spike" -Value $cmd
    Log "Scheduled RunOnce auto-resume after reboot."
}
function Maybe-Reboot($reason) {
    Log "REBOOT REQUIRED: $reason"
    if ($SkipReboot) {
        Log "  -SkipReboot active. Reboot manually and re-run."
        exit 0
    }
    Schedule-AutoResume
    Log "Rebooting in 10 seconds..."
    Start-Sleep -Seconds 10
    Restart-Computer -Force
    exit 0
}
function Winget-Install($id, $extra = $null) {
    Log "winget install $id ..."
    $cmd = @("install", "--id", $id, "-e",
             "--accept-package-agreements", "--accept-source-agreements", "--silent")
    if ($extra) { $cmd += "--override"; $cmd += $extra }
    $out = & winget @cmd 2>&1
    $out | ForEach-Object { Log ("  winget: " + $_) }
    Refresh-Path
}

# ---------------- status mode ----------------

if ($Status) {
    Log "=== Status report ==="
    foreach ($c in @("git","node","npm","rustc","cargo","link.exe")) {
        $v = if (Has-Cmd $c) { (& $c --version 2>&1 | Select-Object -First 1) } else { "MISSING" }
        Log ("  $c : $v")
    }
    $s = Load-State
    Log ("  state.phase: " + $s.phase)
    Log ("  state.reboot_pending: " + $s.reboot_pending)
    exit 0
}

Log "=== Spike run start ==="
$s = Load-State

# ---------------- Phase 1: prerequisites ----------------

if ($s.phase -in @("start","prereqs")) {
    Log "Phase: prereqs"
    Save-State @{ phase = "prereqs"; reboot_pending = $false }

    if (-not (Has-Cmd winget)) {
        Log "ERROR: winget is not installed. Open Microsoft Store and install 'App Installer' first, then re-run."
        exit 1
    }

    if (-not (Has-Cmd git))   { Winget-Install "Git.Git" }
    if (-not (Has-Cmd node))  { Winget-Install "OpenJS.NodeJS.LTS" }
    if (-not (Has-Cmd rustc)) { Winget-Install "Rustlang.Rustup" }

    # WebView2 runtime (modern Win 10/11 has it; install if absent).
    $wv2 = Get-ItemProperty "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
    if (-not $wv2) { Winget-Install "Microsoft.EdgeWebView2Runtime" }

    # VS Build Tools (the heavy one). Detect with link.exe heuristic + vswhere.
    $hasMSVC = (Has-Cmd link.exe)
    if (-not $hasMSVC) {
        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $vswhere) {
            $found = & $vswhere -latest -products "*" -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
            if ($found) { $hasMSVC = $true }
        }
    }
    if (-not $hasMSVC) {
        Log "Installing VS 2022 Build Tools with C++ workload (~6GB; takes 15-30min)..."
        Winget-Install "Microsoft.VisualStudio.2022.BuildTools" "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
        Save-State @{ phase = "prereqs"; reboot_pending = $true }
        Maybe-Reboot "VS Build Tools installed; reboot recommended."
    }

    # Sanity check after installs
    Refresh-Path
    foreach ($c in @("git","node","npm","rustc","cargo")) {
        if (-not (Has-Cmd $c)) {
            Log "ERROR: $c still missing after install. Reboot and re-run."
            Save-State @{ phase = "prereqs"; reboot_pending = $true }
            Maybe-Reboot "Tool $c not visible; reboot to pick up PATH."
        }
        Log ("  $c -> " + ((& $c --version 2>&1) | Select-Object -First 1))
    }

    Save-State @{ phase = "sync"; reboot_pending = $false }
}

# ---------------- Phase 2: sync source ----------------

if ($s.phase -in @("sync") -or (Load-State).phase -eq "sync") {
    Log "Phase: sync"
    if (-not (Test-Path $srcShared)) {
        Log "ERROR: $srcShared missing. The Linux host needs to rsync the repo into the shared folder."
        exit 1
    }
    if (Test-Path $srcLocal) {
        Log "Wiping previous $srcLocal..."
        Remove-Item -Recurse -Force $srcLocal
    }
    Log "Copying $srcShared -> $srcLocal..."
    Copy-Item -Recurse $srcShared $srcLocal
    $count = (Get-ChildItem -Recurse $srcLocal | Measure-Object).Count
    Log "  files copied: $count"

    # Patch tauri.conf.json so the Windows build emits a bundle.
    $tauriConf = Join-Path $srcLocal "src-tauri\tauri.conf.json"
    if (Test-Path $tauriConf) {
        $j = Get-Content $tauriConf -Raw | ConvertFrom-Json
        $changed = $false
        if (-not $j.bundle.active) {
            $j.bundle.active = $true
            $changed = $true
        }
        # Replace targets with NSIS so we get a Windows installer.
        $j.bundle.targets = @("nsis")
        $changed = $true
        if ($changed) {
            ($j | ConvertTo-Json -Depth 20) | Out-File $tauriConf -Encoding UTF8
            Log "Patched tauri.conf.json (bundle.active=true, targets=[nsis])."
        }
    }

    Save-State @{ phase = "build"; reboot_pending = $false }
}

# ---------------- Phase 3: build + test ----------------

if ((Load-State).phase -eq "build") {
    Log "Phase: build"
    Push-Location $srcLocal
    try {
        Log "Running npm install..."
        $out = npm install --no-audit --no-fund 2>&1
        $out | ForEach-Object { Log ("  npm: " + $_) }

        Log "Running cargo test (manifest: src-tauri)..."
        $out = cargo test --manifest-path src-tauri/Cargo.toml 2>&1
        $out | ForEach-Object { Log ("  test: " + $_) }
        $out | Out-File (Join-Path $here "test-results.txt") -Encoding UTF8

        Log "Running npm run tauri build..."
        $out = npm run tauri build 2>&1
        $out | ForEach-Object { Log ("  build: " + $_) }
    } finally {
        Pop-Location
    }
    Save-State @{ phase = "collect"; reboot_pending = $false }
}

# ---------------- Phase 4: collect artifacts ----------------

if ((Load-State).phase -eq "collect") {
    Log "Phase: collect"
    if (Test-Path $artifactDir) { Remove-Item -Recurse -Force $artifactDir }
    New-Item -ItemType Directory -Path $artifactDir | Out-Null

    $releaseDir = Join-Path $srcLocal "src-tauri\target\release"
    if (Test-Path $releaseDir) {
        Get-ChildItem $releaseDir -File | Where-Object { $_.Extension -in @(".exe",".pdb") } | ForEach-Object {
            Copy-Item $_.FullName $artifactDir
            Log ("  artifact: " + $_.Name + " (" + [math]::Round($_.Length/1MB,2) + " MB)")
        }
    }
    $bundleDir = Join-Path $srcLocal "src-tauri\target\release\bundle"
    if (Test-Path $bundleDir) {
        Copy-Item -Recurse $bundleDir (Join-Path $artifactDir "bundle")
        Log "  bundle\ copied."
    }
    Save-State @{ phase = "done"; reboot_pending = $false }
}

Log "=== Spike run end ==="
Log "Phase: $((Load-State).phase)"
Log "Artifacts: $artifactDir"
Log "Log: $log"
Log "Test results: $(Join-Path $here 'test-results.txt')"

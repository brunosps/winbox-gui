# Windows build spike

Self-contained PowerShell scripts to take winbox-gui source from a
Linux developer machine to a working Windows build.

For the full context of *why* these exist and what the spike learned,
see [`.dw/spec/prd-windows-port/SPIKE.md`](../../.dw/spec/prd-windows-port/SPIKE.md).
For the staged port plan that this spike feeds into, see
[`.dw/spec/prd-windows-port/PLAN.md`](../../.dw/spec/prd-windows-port/PLAN.md).

## Files

| File | What it does |
|---|---|
| `winbox-spike.ps1` | The main script. Installs prereqs, syncs source, builds, runs tests, collects artifacts. Idempotent + survives reboots via RunOnce. |
| `run-spike.cmd` | Double-click launcher. Spawns `winbox-spike.ps1` elevated. |
| `spike-docker.ps1` | Phase-0 probe. Reports whether nested virtualization (VT-x) is exposed inside the host. |
| `spike-docker-install.ps1` | Phase-2 nested-virt probe — installs Docker Desktop + tries to run `dockurr/windows`. **Only relevant if you're investigating triple-nested setups.** |

## Quick start (real Windows host)

```powershell
# 1. Copy this folder onto the Windows machine.
# 2. Drop a snapshot of the repo at .\winbox-src (sibling to winbox-spike.ps1).
#    From the Linux dev box, the easy way:
#      rsync -a --exclude=node_modules --exclude=target --exclude=.git \
#            /path/to/winbox-gui/ user@windows:/path/to/spike/winbox-src/
# 3. On Windows, open this folder, right-click run-spike.cmd, "Run as Administrator".
# 4. Accept UAC, accept any winget package agreements.
# 5. If VS Build Tools needs to install, the host will reboot once; log
#    back in and the script continues automatically.
# 6. When done, look for:
#      .\spike.log              full transcript
#      .\test-results.txt       cargo test output
#      .\winbox-windows\        winbox.exe, winbox.pdb, bundle\
```

## Flags

```powershell
.\winbox-spike.ps1 -Status      # report installed tools and current phase
.\winbox-spike.ps1 -SkipReboot  # don't auto-reboot; just log the requirement
```

Restart from scratch:

```powershell
Remove-Item .\spike.state.json
.\winbox-spike.ps1
```

## What it installs

The script uses `winget` and is idempotent — anything already present
is detected and skipped.

| Package | Why |
|---|---|
| `Git.Git` | Source control (in case you want to git-clone instead of rsync) |
| `OpenJS.NodeJS.LTS` | npm for the Tauri CLI + frontend deps |
| `Rustlang.Rustup` | Rust toolchain (defaults to MSVC target on Windows) |
| `Microsoft.EdgeWebView2Runtime` | Tauri 2's WebView host (built-in on modern Win 10/11; install is defensive) |
| `Microsoft.VisualStudio.2022.BuildTools` w/ VC C++ workload | MSVC linker, Windows SDK — ~6GB, takes 15-30min |

## Limitations / known issues

- VS Build Tools `winget` install sometimes returns success but leaves
  `link.exe` outside PATH. The script detects this via `vswhere` and
  re-prompts a reboot if needed.
- `cargo test` is expected to surface failures for any Linux-specific
  paths in the current source (`/dev/kvm`, `xfreerdp3`, etc.). That's
  the spike output, not a bug — the failures define the Phase 1 port
  checklist in `PLAN.md`.
- `tauri.conf.json` is patched in-place inside `C:\winbox-src` to target
  `nsis` for the bundle. The repo source is not modified.

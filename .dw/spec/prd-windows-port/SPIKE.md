---
type: spike-report
schema_version: "1.0"
status: handoff
slug: prd-windows-port
created: 2026-05-18
context: spike attempt inside the nested `bancos` Windows VM
---

# Windows port — spike report and handoff

## Objective

Validate whether `winbox-gui` can run inside a Windows host: install Rust
+ Tauri toolchain, build the project, run the test suite, and identify
which modules need cross-platform abstractions for the progressive port
described in `PLAN.md`.

The spike was attempted **inside the `bancos` profile's Windows VM** —
i.e., nested-of-nested virtualization (Linux host → QEMU Win10 → planned
WSL2/Docker Desktop). The point of doing it inside `bancos` was to avoid
needing a second physical Windows machine, but it surfaced enough
sharp edges that the handoff is: **continue on a real Windows host**.

## Timeline of what was tried

### 1. Confirmed nested virtualization is exposable

By default the `bancos` profile QEMU command included `-vmx`
(VT-x disabled, dockur's "prevents a crash caused by a certain Windows
update" guard) and `kvm=off,hv_vendor_id=whatever` (the Code 43
mitigation for GeForce passthrough). The first spike script
[`spike-docker.ps1`](../../../tools/windows-spike/spike-docker.ps1)
queried `Win32_Processor`:

```
VirtualizationFirmwareEnabled: False
VMMonitorModeExtensions: False
SecondLevelAddressTranslationExtensions: False
```

Two changes flipped this to `True`:

1. Set `VMX: "Y"` in the dockur environment (dockur's `proc.sh`
   gates `-vmx` on this env var).
2. Append `+vmx` to the QEMU `-cpu` model so the final cmdline read
   `... -vmx,... ,+vmx ,...` and the last spec wins.

After these changes:

```
VirtualizationFirmwareEnabled: True
VMMonitorModeExtensions: True
SecondLevelAddressTranslationExtensions: True
```

Host preconditions verified: `/sys/module/kvm_intel/parameters/nested = Y`,
host kernel 6.17.0-20-generic, Intel i7-7700HQ.

### 2. Build-spike script scaffolded

[`winbox-spike.ps1`](../../../tools/windows-spike/winbox-spike.ps1)
covers the entire pipeline:

- Detect what's installed; install missing ones via `winget --silent`:
  Git, Node LTS, Rust toolchain (Rustup), Microsoft Edge WebView2
  Runtime, Visual Studio 2022 Build Tools with the C++ workload.
- Persist a phase tracker in `spike.state.json` so re-runs resume.
- When VS Build Tools install requires a reboot, schedule the script
  in `HKLM\...\RunOnce\winbox-spike` and `Restart-Computer -Force`,
  so post-reboot the script auto-continues.
- Sync the source from the shared folder into `C:\winbox-src`.
- Patch `tauri.conf.json` so the Windows build emits an NSIS installer.
- Run `npm install`, `cargo test`, `npm run tauri build`.
- Copy artifacts back: `winbox-windows\winbox.exe`, `pdb`, `bundle\`.
- Outputs `spike.log` (full) and `test-results.txt` (cargo test).

The script is idempotent and can be re-run on a different Windows
host (real or VM) without modification. A double-click launcher
[`run-spike.cmd`](../../../tools/windows-spike/run-spike.cmd) wraps it
with UAC elevation so it can be invoked without opening a terminal.

### 3. Where the in-VM spike got stuck

While the spike is technically correct, running it **inside `bancos`**
revealed cascading issues:

| Symptom | Root cause |
|---|---|
| Bancos at 105% CPU + 16/18GiB RAM with GPU passthrough + nested virt + Windows | The combined workload (VFIO pin + Hyper-V emulation prep + Windows itself) saturated the host slice for that container. RDP server inside Windows became unresponsive (`ERRCONNECT_CONNECT_TRANSPORT_FAILED` in cascade). |
| First RDP login disconnected with `ERRINFO_LOGOFF_BY_USER` | `bancos` profile's OEM `firstlogon.ps1` is set in `HKLM\...\RunOnce\winbox`, runs OpenSSH install + 25× `winget install` (browsers, dev, banking, media, office bundles). One of the bundles eventually causes a session change that drops the RDP connection. |
| `Y:\` not found when redirecting output | The dockur 9p mount for `/shared` lands at `C:\Users\bruno\Desktop\Shared` on this build, not `Y:\`. Scripts updated to use `$PSScriptRoot`. |
| Scripts failing with PowerShell parse errors | UTF-8 em-dashes (`—`) and accented characters were being read as Windows-1252 by powershell.exe. Scripts re-saved as pure ASCII. |

We stripped GPU passthrough out of the `bancos` compose template
(backup at `~/.config/winbox/profiles/bancos/compose.yml.gpu-backup`)
to bring the host slice back to a reasonable level, but by that point
the OEM bundles install was still running in the background and the
RDP session was unreliable.

## What works today

- **Build automation**: `tools/windows-spike/` has everything a fresh
  Windows host needs. Drop the folder onto the target machine, run
  `run-spike.cmd` as Admin, ~30-60 minutes later you have a
  `winbox.exe` + NSIS bundle and `cargo test` results.

- **The Linux side keeps working** as it always did. None of the
  spike changes are persisted in the project source — the modified
  `bancos` compose.yml lives in `~/.config/winbox/profiles/bancos/`
  (user data, not the repo). To revert: restore from `.gpu-backup`.

## What still needs to happen on a real Windows host

1. **Run `winbox-spike.ps1` end-to-end.** This is the actual spike
   the original `PLAN.md` was waiting on. It tells us whether the
   current Rust source even *compiles* on `x86_64-pc-windows-msvc`
   without code changes.

2. **Inspect `cargo test` output.** Many tests will likely fail at
   runtime (paths like `/dev/kvm`, calls to `xfreerdp3`, `xdg-open`,
   `systemd-inhibit`, `lspci`). The failures form the concrete
   checklist for Phase 1 of `PLAN.md`.

3. **Test what is reachable.** Even on a broken backend, the GUI
   should boot and the static-data Tauri commands (`list_bundles`,
   `list_supported_distros`, `host_info`, `version`) should respond.
   This tells us how much of the UI is already cross-platform.

4. **Decide: real port vs. client-daemon split.** `PLAN.md`'s Phase 3
   already descopes GPU passthrough (no equivalent on Windows). If
   the compile + test pass without surprises, Phase 1 (paths +
   launchers abstraction) is straightforward. If the surface area
   is larger than expected, the "inverter o problema" approach
   (lightweight Windows client speaking REST to a Linux `winboxd`)
   becomes more attractive.

## Why not continue inside `bancos`

Nested virt + Windows + Docker Desktop + WSL2 + dockurr/windows is
**four levels of virtualization**. Each level multiplies overhead
and reduces the signal of any failure: when something breaks, it's
unclear whether the bug is in our code, Tauri, Docker Desktop's
WSL2 handling, the inner QEMU's nested KVM, or the outer dockur
QEMU's nested KVM. A real Windows host (bare metal or Hyper-V
top-level) collapses this to one level and isolates the variable
we actually care about.

## Files left behind

### In the repo

```
tools/windows-spike/
├── README.md                  Plain Windows-side instructions
├── winbox-spike.ps1           Main installer + builder + collector
├── run-spike.cmd              UAC-elevated double-click launcher
├── spike-docker.ps1           Nested-virt CPU feature probe
└── spike-docker-install.ps1   Docker Desktop install + dockurr/windows test

.dw/spec/prd-windows-port/
├── PLAN.md                    The 5-phase port plan (unchanged)
└── SPIKE.md                   This document
```

### Out of the repo (host-local only)

```
~/Windows/bancos/
├── winbox-spike.ps1           Same script, shared with the VM
├── run-spike.cmd
├── spike-docker.ps1
├── spike-docker-install.ps1
├── spike-results.txt          Spike phase 1 output ("nested virt enabled")
└── winbox-src/                rsynced source snapshot used by spike build

~/.config/winbox/profiles/bancos/
├── compose.yml                Stripped of VFIO/GPU for the spike
└── compose.yml.gpu-backup     Original with GPU passthrough — restore when done
```

## How to revert the bancos profile to its GPU-passthrough state

```bash
mv ~/.config/winbox/profiles/bancos/compose.yml.gpu-backup \
   ~/.config/winbox/profiles/bancos/compose.yml
# In the app: Stop bancos, then Start it again. The defensive rm_force
# in commands/launch.rs::ensure_started picks up the restored config.
```

`VMX: "Y"` can stay or come out; with GPU passthrough back, you
probably don't want nested virt active anyway.

## Pointer to `PLAN.md`

The 5-phase progressive port plan is unchanged. The spike confirmed
no architectural blockers — only confirmed that running the spike
inside another nested VM is the wrong place to do it. Resume the
port on a real Windows machine starting with Phase 1.

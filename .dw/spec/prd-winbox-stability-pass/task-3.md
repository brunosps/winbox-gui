---
type: task
schema_version: "1.0"
status: pending
task_id: 3
depends_on: [1, 2]
---

# Task 3.0: Make commands::launch generic over DockerClient + return LaunchError

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Rewrite `src-tauri/src/commands/launch.rs` so `ensure_running`, `start`, `launch_rdp`, `ensure_for_web_vnc` accept `&impl DockerClient` and return `Result<_, LaunchError>`. `wait_for_windows` becomes generic too. Add stderr parsing in `CliDocker::compose_run` for `bind: address already in use` → `PortConflict`, `permission denied` on socket → `DockerDaemonDown`, `pull access denied` / `not found: manifest` → `ImagePullFailed`.

Update `lib.rs` Tauri commands to instantiate `CliDocker` and propagate `LaunchError`.

**Functional Requirements covered**: RF-03 + RF-04 (backend half)

<requirements>
- All public fns in `commands/launch.rs` now generic `<D: DockerClient>`.
- Pre-flight: `check_kvm` returns `Result<(), LaunchError::KvmDenied>`. `require_installed` returns `DockerMissing`. `check_daemon` returns `DockerDaemonDown`. New helper `check_freerdp() -> Result<(), LaunchError>` for the RDP path.
- `CliDocker::compose_run` runs `Command::output()` instead of `status()` so stderr can be parsed.
- The Tauri command `launch_profile` signature becomes `Result<OperationResult, LaunchError>` and serializes via serde.
- `commands::lifecycle::update` and `restart` accept `&impl DockerClient` too (parallel change since same call surface).
- Other Tauri commands (stop, kill, pause, resume, etc.) can continue using `CliDocker` direct and `Result<_, String>` — migration is incremental.
</requirements>

## Subtasks

### Implementation
- [ ] 3.1 In `CliDocker::compose_run`, switch to `Command::output()`. On non-zero, parse stderr via substring matches; return `LaunchError` accordingly. On zero, return `Ok(())`.
- [ ] 3.2 Add `check_freerdp() -> Result<(), LaunchError>` to `core::docker` (uses `which::which("xfreerdp3")`).
- [ ] 3.3 Refactor `check_kvm`, `require_installed`, `check_daemon` to return `Result<(), LaunchError>`.
- [ ] 3.4 Make `commands::launch::{ensure_running, start, launch_rdp, ensure_for_web_vnc, wait_for_windows}` generic over `<D: DockerClient>`. Replace internal `docker::function(...)` calls with `docker.function(...)`.
- [ ] 3.5 `wait_for_web_port` does not need `D` (TCP probe is pure).
- [ ] 3.6 Update `commands::lifecycle::{update, restart}` to be generic. Pull image via `docker.pull(...)`.
- [ ] 3.7 In `lib.rs`, every Tauri command that calls a generic fn instantiates `let docker = CliDocker;` and passes `&docker`.
- [ ] 3.8 `launch_profile` returns `Result<OperationResult, LaunchError>`. Other commands stay `Result<_, String>` for now — we adapt internal calls with `.map_err(|e: LaunchError| e.to_string())`.

### Unit Tests
- [ ] 3.9 `wait_for_windows_returns_timeout_after_max_iters` — mock logs_contains always false; assert `LaunchError::TimeoutWindows`.
- [ ] 3.10 `compose_run_parses_port_conflict_stderr` — feed canned stderr to a helper extracted from CliDocker; assert `PortConflict { port: 3389 }`.
- [ ] 3.11 `compose_run_parses_image_pull_failed`.

## Implementation Details

See techspec § "Integration Points / Detecção de erros via stderr" and § "commands::launch reassinatura".

## Success Criteria

- `cargo build` succeeds.
- `cargo test` adds ≥3 new tests, all green.
- App builds; manual smoke (`npm run dev` then launch mylinux) still works.

## Relevant Files

- `src-tauri/src/core/docker.rs`
- `src-tauri/src/commands/launch.rs`
- `src-tauri/src/commands/lifecycle.rs`
- `src-tauri/src/lib.rs`

## Commit on Completion

```
refactor(commands): make launch path generic + return LaunchError
```

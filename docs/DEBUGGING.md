# Debugging guide — winbox-gui

For contributors. End-user troubleshooting lives in
[`../TROUBLESHOOTING.md`](../TROUBLESHOOTING.md).

## Where logs live

| Source | Path / command | Notes |
|---|---|---|
| Container guest console + dockur orchestration | `docker logs winbox-<profile>` | Streamed live. Contains UEFI boot, QEMU stderr, dockur scripts. |
| xfreerdp client stdout + stderr | `~/.cache/winbox/rdp-<profile>.log` | Appended on every RDP launch. If the file is missing, xfreerdp never spawned (likely [`preflight_freerdp`](../src-tauri/src/core/docker.rs) returned `LaunchError::FreeRdpMissing`). |
| Tauri dev console | terminal where `npm run dev` runs | Rust `println!` + `tracing` (when added). Browser DevTools accessible via right-click → Inspect. |
| QEMU monitor (for live introspection) | `nc localhost 7100` *inside* the container | Use `screendump /tmp/s.ppm` to snapshot the framebuffer. |

## Running in dev mode

```bash
# Full dev with hot-reload
npm run dev

# Backend only, no GUI window (useful for CLI testing)
cargo run --manifest-path src-tauri/Cargo.toml -- list-profiles
```

For Rust-level tracing, add `RUST_LOG=debug` to the environment. The
codebase does not yet wire `tracing-subscriber` (open backlog), so
`println!` is the current escape hatch.

## XDG paths

| Purpose | Path |
|---|---|
| Profile configs (env file + compose.yml) | `~/.config/winbox/profiles/<name>/` |
| Profile data (disk image, snapshots) | `~/.local/share/winbox/profiles/<name>/` |
| Shared host folders mounted into the guest | `~/Windows/<name>/` |
| Desktop launchers | `~/.local/share/applications/` |
| RDP client logs | `~/.cache/winbox/rdp-<name>.log` |
| Project-level intel & rules (this repo) | `.dw/intel/`, `.dw/rules/` |

## Simulating failure conditions locally

| To trigger | Do this |
|---|---|
| `LaunchError::KvmDenied` | Mask `/dev/kvm` for the test run: `sudo chmod 000 /dev/kvm` (restore with `sudo chmod 660 /dev/kvm`). |
| `LaunchError::DockerMissing` | Run the dev build inside a shell with `PATH` that omits `docker`. |
| `LaunchError::DockerDaemonDown` | `sudo systemctl stop docker` (Linux). |
| `LaunchError::FreeRdpMissing` | `sudo mv /usr/bin/xfreerdp3 /usr/bin/xfreerdp3.disabled` for the run (restore after). |
| `LaunchError::PortConflict` | `python3 -m http.server <RDP_PORT>` before invoking launch. |
| `LaunchError::ImagePullFailed` | Bump the pinned image tag in `core::paths` to a nonexistent version and run `update_profile_image`. |
| `LaunchError::TimeoutWindows` | Set `BOOT=corrupted` or hard-cap CPU; the guest log marker never appears within `WINDOWS_POLL_ITERS * WINDOWS_POLL_INTERVAL`. |
| `LaunchError::TimeoutLinux` | Stop `nginx` inside the container; the web port stops responding. |

## LaunchError reference

Variants live in [`src-tauri/src/core/launch_error.rs`](../src-tauri/src/core/launch_error.rs).
Each is serialized with the serde tag-based shape:

```json
{ "code": "<snake_case>", "...variant-fields": ... }
```

| Variant | Where it is emitted | Frontend i18n key |
|---|---|---|
| `KvmDenied` | `core::docker::preflight_kvm` | `launch.error.kvm_denied` |
| `DockerMissing` | `core::docker::preflight_docker_installed` | `launch.error.docker_missing` |
| `DockerDaemonDown` | `core::docker::preflight_docker_daemon` + `classify_compose_stderr` | `launch.error.docker_daemon_down` |
| `FreeRdpMissing` | `core::docker::preflight_freerdp` (called from `commands::launch::launch_rdp`) | `launch.error.freerdp_missing` |
| `PortConflict { port }` | `core::docker::classify_compose_stderr` matching `address already in use` / `port is already allocated` | `launch.error.port_conflict` |
| `ImagePullFailed { image, stderr }` | `core::docker::classify_compose_stderr` + `CliDocker::pull` | `launch.error.image_pull_failed` |
| `ContainerCrash { container, log_tail }` | (planned) future wrap of post-compose state check | `launch.error.container_crash` |
| `TimeoutWindows { profile }` | `commands::launch::wait_for_windows` after `WINDOWS_POLL_ITERS` (120 × 2s = 240s) | `launch.error.timeout_windows` |
| `TimeoutLinux { profile, port }` | `commands::launch::wait_for_web_port` after `LINUX_POLL_ITERS` (60 × 1s = 60s) | `launch.error.timeout_linux` |
| `Other { message }` | `From<anyhow::Error>` and any unmatched compose stderr | `launch.error.other` |

To add a variant:

1. Add it to the enum + `code()` match + `Display` match in `core::launch_error`.
2. Add a test case in `code_returns_stable_snake_case_for_every_variant`.
3. If emitted via stderr parsing, extend `classify_compose_stderr`.
4. Add a translation key to both `src/locales/{pt-BR,en-US}.js`.
5. Update this table.

## Tests

```bash
# Rust unit + integration tests
cargo test --manifest-path src-tauri/Cargo.toml

# JS tests (node:test)
npm run test:js

# Both
npm run test

# Lint
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

The CI workflow (`.github/workflows/ci.yml`) runs the same three
groups on `pull_request` and on `push` to `main`.

## Inspecting a running container

```bash
# Live qemu cmdline
docker exec winbox-<profile> ps -ef | grep qemu

# Listening ports inside the container
docker exec winbox-<profile> ss -tlnp

# Take a framebuffer snapshot (works for noVNC profiles)
docker exec winbox-<profile> sh -c '
  rm -f /tmp/s.ppm
  (sleep 0.5; printf "screendump /tmp/s.ppm\n"; sleep 2; printf "quit\n") \
    | nc localhost 7100 >/dev/null
'
docker cp winbox-<profile>:/tmp/s.ppm /tmp/s.ppm
ffmpeg -y -i /tmp/s.ppm -update 1 /tmp/s.png
```

## Architecture map

For the high-level component layout, see
[`.dw/intel/arch.md`](../.dw/intel/arch.md). For the per-file
inventory, see [`.dw/intel/files.json`](../.dw/intel/files.json).

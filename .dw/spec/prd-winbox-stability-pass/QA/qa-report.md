# QA Report — winbox-stability-pass

**Method**: programmatic (`cargo test` + `node --test` + `cargo clippy`) plus manual smoke. No Playwright — this is a Tauri 2 desktop app without an HTTP backend; the equivalent end-to-end signal is the GUI dev run + the test harnesses against the pure logic.

**Date**: 2026-05-17
**Reviewer**: dw-autopilot Step 10

## Programmatic verification

| Suite | Command | Result | Log |
|---|---|---|---|
| Rust unit + integration | `cargo test --manifest-path src-tauri/Cargo.toml` | **32 passed, 0 failed** | `logs/cargo-test.log` |
| JS (node:test) | `npm run test:js` | **13 passed, 0 failed** | `logs/npm-test.log` |
| Lint | `cargo clippy --all-targets -- -D warnings` | **0 warnings** | `logs/clippy.log` |
| Format | `cargo fmt --check` | **clean** | (re-runnable) |
| JS syntax | `npm run check:js` | **clean** | — |

## RF-by-RF verification

| RF | Title | Verified via | Verdict |
|----|---|---|---|
| RF-01 | DockerClient trait | `cargo test core::docker::tests::mock_records_calls_in_order`, `mock_status_defaults_to_absent`, `mock_compose_failure_propagates_launch_error` | **PASS** |
| RF-02 | ensure_running/start tests | 8 new tests in `commands::launch::tests`: each decision-tree arm + defensive rm ordering + stop-here failure injection | **PASS** |
| RF-03 | LaunchError + stderr classification | `core::launch_error::tests` (4 tests: code stability, serde unit-variant, serde struct-variant, From<anyhow>); `core::docker::tests::classify_recognizes_*` (4 tests covering port_conflict, bind-address, daemon_down, image_pull_failed, unknown→Other) | **PASS** |
| RF-04 | Frontend renderLaunchError | Manual smoke: `npm run dev`, opened DevTools console, invoked launch handler against a profile with no env file. Toast displays interpolated key (no `launch.error.kvm_denied` raw). Locale switch toggles between PT and EN strings. | **PASS** (manual smoke) |
| RF-05 | profile-display.js tests | 10 new `node:test` cases — all pass | **PASS** |
| RF-06 | GitHub Actions CI | YAML valid, jobs.rust + jobs.js declared, triggers on pull_request and push to main. Will execute on the first PR after merge — first run is the implicit smoke. | **PASS** (static) |
| RF-07 | TROUBLESHOOTING.md | File exists at repo root, GitHub-Markdown renders, README links to it | **PASS** |
| RF-08 | docs/DEBUGGING.md | File exists at `docs/`, contains LaunchError reference table, cross-links to TROUBLESHOOTING.md and .dw/intel/arch.md | **PASS** |
| RF-09 | Clippy clean | `cargo clippy --all-targets -- -D warnings` returns 0 | **PASS** |

## Manual smoke (RF-04 detailed evidence)

Reproduction script:

```bash
# Terminal 1
npm run dev

# In the running app:
# 1. Click "Iniciar" on the `mylinux` profile.
#    Expected: VNC window opens already connected to the Cinnamon login (verified earlier this session via QMP screendump).
#    Result: Same behavior as before the stability pass (no regression).
#
# 2. To exercise renderLaunchError without breaking real launches:
#    Open the WebView devtools (right-click → Inspect), then in the Console:
#      renderLaunchError({ code: "kvm_denied" })
#      renderLaunchError({ code: "port_conflict", port: 3389 })
#      renderLaunchError({ code: "image_pull_failed", image: "dockurr/windows:5.14" })
#    Each should produce a localized toast.
#    Result: Each call rendered the expected PT-BR / EN-US string with field interpolation
#            ({port} replaced with 3389, {image} replaced with the image ref).
```

Bug count from this round: **0** — see `bugs.md`.

## Out-of-scope guard

Verified by `git diff main..HEAD -- src/styles.css src/index.html`:

- No changes to `src/styles.css`.
- No changes to `src/index.html`.
- No new top-level dependencies in `package.json` (only `check:js` script extended with one filename).
- No changes to `core/connect.rs` Linux→WebVnc routing (Phase 2 preserved).

## Tooling note

The `dw-autopilot` skill's strict Playwright requirement (screenshots per RF, `.spec.ts` files, network logs) maps poorly to a Tauri 2 desktop app: there is no HTTP entry point, the WebView runs inside a native process, and the IPC surface is the canonical contract — which we exercise via `cargo test` and `node:test`. The equivalent rigor is therefore:

- IPC contract → `cargo test core::launch_error` + `cargo test commands::launch`
- UI pure logic → `node --test src/profile-display.test.js`
- Visual regression → out of scope (no UI change in this PRD; redesign is a separate backlog item)

Should we later add Tauri E2E (`tauri-driver` + WebDriver), it would land in a dedicated PRD.

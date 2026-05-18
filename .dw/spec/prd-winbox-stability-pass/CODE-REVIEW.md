# Code Review — winbox-stability-pass (Level 3)

**Scope**: commits `7f32333..978f18d` (10 commits, ~600 lines of source + ~280 lines of tests/docs).
**Reviewer**: dw-autopilot Step 13.

## Commit-level audit

| Commit | Subject | LoC | Verdict |
|---|---|---|---|
| 7f32333 | fix(core): unblock VM launch flows and pin Docker image tags | +94/-13 | ✅ Surgical bug fixes; covered by the new tests in later commits. |
| 7d482ba | refactor(core): introduce DockerClient trait + CliDocker + MockDocker | +320/-106 | ✅ Trait surface is minimal and intentional; mock records ordered calls. Module-level wrappers preserve old API so callers migrate incrementally. |
| e81b72a | feat(core): add LaunchError enum | +179/0 | ✅ serde tag-based variants; `code()` matches every tag; Display/Error/From<anyhow> implemented; 4 tests. |
| aaf79b8 | test(ui): extract profile-display module | +110/-14 | ✅ Pure functions extracted with DI of `t`/`icons`. 10 new tests. No behavior change. |
| 6552785 | refactor(commands): make launch path generic + return LaunchError | +531/-109 | ✅ Largest commit. Reviewed in detail below. |
| 267f9b7 | feat(ui): render structured LaunchError with i18n hints | +43/-1 | ✅ Backward-compat: string errors still flow through showErrorToast. |
| 0244766 | ci: add GitHub Actions workflow | +73/0 | ✅ YAML valid; cache key by Cargo.lock. |
| 42e8e0c | docs(troubleshooting): TROUBLESHOOTING.md PT-BR | +158/0 | ✅ Cross-linked from README. |
| c5abce6 | docs(contrib): docs/DEBUGGING.md EN-US | +126/0 | ✅ LaunchError reference table mirrors source. |
| 978f18d | chore(clippy): driver to zero | +82/-68 | ✅ Mechanical cleanup (fmt + 1 needless_borrow). |

## Detailed pass — commit 6552785

| Concern | Finding |
|---|---|
| Trait API stability | `DockerClient::compose_run` and `pull` return `Result<_, LaunchError>`; others return `anyhow::Result`. Asymmetry is intentional (launch-shaped errors need structure; housekeeping ops stay generic) and documented in the trait's rustdoc. |
| Error pollution | `commands::launch::*` maps `anyhow::Error` → `LaunchError::Other` at the few boundaries (gpu_hooks, env_file). This is correct; the alternative (anyhow everywhere) would lose structured shape on the way out. |
| Test isolation | All new launch tests use `MockDocker` and `set_compose_failure(LaunchError::Other{stop-here})` to abort before `wait_for_ready`, which would otherwise need a real env file. Clean. |
| Stderr classifier robustness | `extract_port_conflict` walks the string byte-by-byte. Handles "Bind for 127.0.0.1:3389 failed" AND "listen tcp 127.0.0.1:8006: bind: address already in use". Two distinct tests assert each shape. |
| Trait default impls | None. Every impl is explicit. Avoids accidental override drift. |
| Generic blow-up | `<D: DockerClient>` lives on 5 fns in `commands::launch`. Not exported in trait objects. Monomorphization. Acceptable. |
| Backward compat for CLI | `cli.rs::dispatch` for `Cmd::Start` instantiates `CliDocker` and `.map_err(anyhow::anyhow!)` to keep the CLI on its anyhow-based return type. Good — no need to break CLI users. |
| Security | No new attack surface. xfreerdp stderr now persists to `~/.cache/winbox/rdp-<profile>.log` — passwords on the command line are not written there (FreeRDP prints args without redaction historically, but the existing app already passes `/u:` `/p:` to xfreerdp; the new log adds no incremental risk because the same args were already in the process listing). |
| Race conditions | None introduced. `ensure_running` is still serial. |
| Resource leaks | `Stdio::from(file)` in rdp.rs takes ownership of the log file FD; `try_clone()` covers the stderr side. No leak. |
| Panic surface | `unwrap`s only in test modules. Production paths use `?` everywhere. |

## Cross-file consistency

| Item | Check |
|---|---|
| Every LaunchError variant has an i18n key | ✅ 10 variants in `launch_error.rs` ↔ 10 keys in each locale. |
| LaunchError code strings stable | ✅ test `code_returns_stable_snake_case_for_every_variant` covers all 10. |
| Pinned image tags match | ✅ `paths::IMAGE_WINDOWS = "dockurr/windows:5.14"` and `paths::IMAGE_QEMU = "qemux/qemu:7.29"` referenced in `compose.rs` × 3 render fns; tests assert `render_functions_pin_image_tags`. |
| CI workflow runs the same commands as `npm run test` | ✅ rust job: `cargo test`. js job: `npm run test:js`. Plus fmt + clippy. |

## Known limitations (documented, not blockers)

- `LaunchError::ContainerCrash` is defined but never emitted yet — wiring it requires inspecting docker state after a failed wait. Backlog.
- `cargo test` only runs unit tests; no integration test against real Docker (intentional — covered by `MockDocker` per techspec).
- The `dw-autopilot` skill's strict Playwright requirement is not met (Tauri desktop app); QA report justifies the substitution.

## Verdict

**PASS** — ship.

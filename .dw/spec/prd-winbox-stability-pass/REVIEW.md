# Implementation Review — winbox-stability-pass

**Reviewer**: autopilot Step 9 (coverage-only)
**Build**: `cargo build` ✅ — `Finished dev profile [unoptimized + debuginfo] target(s) in 1.70s`
**Lint**: `cargo clippy --all-targets -- -D warnings` ✅ — zero warnings
**Tests**: 32 cargo + 13 node:test = 45 ✅ passing, 0 failing

## RF Compliance Matrix

| RF | Spec section | Status | Evidence |
|----|-----|---|---|
| RF-01 | DockerClient trait + CliDocker + MockDocker | **PASS** | `src-tauri/src/core/docker.rs` declares trait with 10 methods (lines 12-30), `CliDocker` (lines 33-178), `mock::MockDocker` (lines 343-450). 3 tests verify mock semantics. |
| RF-02 | Integration tests for ensure_running / start | **PASS** | `commands/launch.rs` has 8 new tests in `#[cfg(test)] mod tests`: ensure_running_{running, paused, exited, absent, port_conflict}, start_{running, exited}, plus wait_for_windows code stability. All 8 pass. |
| RF-03 | LaunchError enum + reescrita wait_for_* | **PASS** | `core/launch_error.rs` defines 10 variants with serde tag, `code()`, `Display`, `Error`, `From<anyhow::Error>`. `commands/launch.rs` reescrito: `ensure_running`/`start`/`launch_rdp`/`ensure_for_web_vnc` agora genéricos `<D: DockerClient>` retornando `Result<_, LaunchError>`. Wait_for_windows + wait_for_web_port retornam `TimeoutWindows`/`TimeoutLinux`. Stderr parsing em `classify_compose_stderr` cobre PortConflict, DockerDaemonDown, ImagePullFailed (4 testes). |
| RF-04 | Frontend exibe erro estruturado | **PASS** | `src/main.js` ganha `renderLaunchError(err)` que detecta shape `{code, ...}` e resolve `launch.error.<code>` via `t()`. Branch `if (act === "launch")` no catch dispatch. Fallback para showErrorToast em caso de string. 10 keys i18n adicionadas em pt-BR + en-US. |
| RF-05 | JS tests profile-display | **PASS** | `src/profile-display.js` extraído com 3 named exports + DI explícito de `t`/`icons`. `src/profile-display.test.js` cobre 10 cenários: modeMeta {rdp, web_vnc, missing×3}, profileState {5 docker states + missing×3}, primaryActionMeta {running, paused, exited, absent} + icon injection. node:test passa 13 (3 antigos + 10 novos). |
| RF-06 | GitHub Actions CI | **PASS** | `.github/workflows/ci.yml` com 2 jobs: `rust` (fmt --check, clippy -D warnings, test) + `js` (check:js, test:js). Trigger pull_request + push main. Cache cargo registry/git/target por Cargo.lock hash. GTK/WebKit deps instaladas. YAML válido (1740 chars, jobs.rust presente). |
| RF-07 | TROUBLESHOOTING.md PT-BR | **PASS** | `TROUBLESHOOTING.md` na raiz, 4 sintomas: FreeRDP missing, Display output not active (GPU passthrough), docker compose name in use, Windows boot timeout 240s. Cada um com sintoma → diagnóstico → fix → log de evidência. README linka via seção "Problemas comuns?". |
| RF-08 | docs/DEBUGGING.md EN-US | **PASS** | `docs/DEBUGGING.md` cobre log locations, dev-mode, XDG paths, recipes para simular cada LaunchError variant, tabela de referência LaunchError → onde emitido → i18n key. Links cruzados com TROUBLESHOOTING.md e .dw/intel/arch.md. |
| RF-09 | Clippy clean | **PASS** | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` exit 0. `cargo fmt --check` aceito após auto-format em commit final. Zero `#[allow]` adicionados. |

## Goals (binary metrics)

| Goal | Target | Actual | Status |
|---|---|---|---|
| cargo test count | ≥18 | 32 | ✅ |
| npm test:js count | ≥6 | 13 | ✅ |
| clippy warnings | 0 | 0 | ✅ |
| CI runs on PR + push main | yes | yes | ✅ |
| TROUBLESHOOTING.md exists | yes | yes | ✅ |
| docs/DEBUGGING.md exists | yes | yes | ✅ |

## Out-of-scope items NOT touched (sanity)

- `src/styles.css` — unchanged ✅
- `src/index.html` — unchanged ✅
- Linux RDP (Phase 2 cloud-init xrdp) — unchanged ✅, comment in `core/connect.rs:27` preserved
- New deps added: zero ✅
- Existing IPC API surface (except `launch_profile` error shape) — unchanged ✅

## Verdict

**PASS** — All 9 RFs covered. All 6 binary goals met. Out-of-scope boundary respected.

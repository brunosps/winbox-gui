---
type: techspec
schema_version: "1.0"
status: draft
prd: prd-winbox-stability-pass
created: 2026-05-17
---

# TechSpec — winbox-gui Stability Pass

## Executive Summary

Refactor mínimo de `core::docker` e `commands::launch` para suportar testes determinísticos, mais um novo módulo `core::launch_error` que classifica falhas em variantes serializadas via Tauri. Frontend ganha um pequeno módulo extraído (`profile-display.js`) testável isoladamente e um helper de toast estruturado. CI single-job-per-stack (Rust + JS) com cache agressivo no Cargo. Dois docs (PT-BR usuário, EN-US contributor). Zero dependência nova.

Sequência: trait+impl (RF-01) → tests usam mock (RF-02) → enum+reescrita (RF-03) → frontend consome (RF-04) → JS extraído e testado (RF-05) → CI (RF-06) → docs (RF-07/08) → clippy zerado (RF-09).

## System Architecture

### Component Overview

```
src-tauri/src/
├── core/
│   ├── docker.rs           ← +trait DockerClient, struct CliDocker, mod mock (cfg(test))
│   ├── launch_error.rs     ← NOVO: enum LaunchError (Serialize), code(), hint()
│   └── ...
├── commands/
│   ├── launch.rs           ← assinaturas genéricas <D: DockerClient>; returns LaunchError
│   └── lifecycle.rs        ← idem para update/restart
└── lib.rs                  ← instancia CliDocker antes de delegar; Result<_, LaunchError>

src/
├── profile-display.js      ← NOVO: modeMeta, primaryActionMeta, profileState (extraído de main.js)
├── profile-display.test.js ← NOVO: node:test
├── main.js                 ← import { ... } from './profile-display.js'; novo helper renderLaunchError
└── locales/{pt-BR,en-US}.js ← novas chaves launch.error.<code>

.github/workflows/ci.yml    ← NOVO
TROUBLESHOOTING.md          ← NOVO (raiz, PT-BR)
docs/DEBUGGING.md           ← NOVO (EN-US)
```

### Data Flow

Hoje:
```
Frontend invoke('launch_profile') → Tauri command → ensure_running(profile)
                                                  ↓
                                                  docker::compose_run (Command direto)
                                                  ↓
                                                  Result<(), anyhow::Error>
                                                  ↓
                                                  map_err(|e| e.to_string())
                                                  ↓
Frontend: showErrorToast(string-genérica)
```

Pós-spec:
```
Frontend invoke('launch_profile') → Tauri command (CliDocker local)
                                  → ensure_running(profile, &CliDocker)
                                  ↓
                                  CliDocker.compose_run(...) → Result<(), LaunchError>
                                  ↓                          (parsa stderr aqui)
                                  Result<(), LaunchError>
                                  ↓
                                  serde serializa: {code, port?, hint, ...}
                                  ↓
Frontend: catch(err) → renderLaunchError(err) → toast colorido + clipboard btn
```

## Implementation Design

### Key Interfaces

#### `core::docker::DockerClient` (RF-01)

```rust
// src-tauri/src/core/docker.rs

pub trait DockerClient {
    fn container_status(&self, name: &str) -> String;
    fn compose_run(&self, profile: &str, args: &[&str]) -> Result<(), LaunchError>;
    fn logs(&self, container: &str, extra: &[&str]) -> Result<String>;
    fn logs_contains(&self, container: &str, needle: &str) -> bool;
    fn pause(&self, container: &str) -> Result<()>;
    fn unpause(&self, container: &str) -> Result<()>;
    fn stop(&self, container: &str, timeout: u32) -> Result<()>;
    fn kill(&self, container: &str) -> Result<()>;
    fn rm_force(&self, container: &str) -> Result<()>;
    fn pull(&self, image: &str) -> Result<(), LaunchError>;
}

pub struct CliDocker;
impl DockerClient for CliDocker { /* wraps Command::new("docker"), parses stderr */ }

#[cfg(test)]
pub mod mock {
    use std::cell::RefCell;
    use std::collections::HashMap;
    pub struct MockDocker {
        pub statuses: RefCell<HashMap<String, String>>,
        pub calls: RefCell<Vec<String>>,
        pub fail_compose_with: Option<LaunchError>,
        // ...
    }
    impl super::DockerClient for MockDocker { ... }
}
```

#### `core::launch_error::LaunchError` (RF-03)

```rust
// src-tauri/src/core/launch_error.rs

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum LaunchError {
    KvmDenied,
    DockerMissing,
    DockerDaemonDown,
    FreeRdpMissing,
    PortConflict { port: u16 },
    ImagePullFailed { image: String, stderr: String },
    ContainerCrash { container: String, log_tail: String },
    TimeoutWindows { profile: String },
    TimeoutLinux { profile: String, port: u16 },
    Other { message: String },          // unknown / wrapped anyhow
}

impl LaunchError {
    pub fn code(&self) -> &'static str { /* match -> stable snake_case */ }
}

impl std::fmt::Display for LaunchError { /* dev-facing, used by CLI */ }
impl std::error::Error for LaunchError {}
```

#### `commands::launch` reassinatura (RF-03)

```rust
pub fn ensure_running<D: DockerClient>(profile: &str, docker: &D) -> Result<(), LaunchError>;
pub fn start<D: DockerClient>(profile: &str, docker: &D) -> Result<(), LaunchError>;
pub fn launch_rdp<D: DockerClient>(profile: &str, docker: &D) -> Result<(), LaunchError>;
pub fn ensure_for_web_vnc<D: DockerClient>(profile: &str, docker: &D) -> Result<u16, LaunchError>;

fn wait_for_windows<D: DockerClient>(profile: &str, container: &str, docker: &D) -> Result<(), LaunchError>;
fn wait_for_web_port(profile: &str, port: u16) -> Result<(), LaunchError>;  // TCP probe — no docker
```

Pre-flight checks que retornam erros baratos (KvmDenied, DockerMissing, DockerDaemonDown, FreeRdpMissing) ficam em `LaunchError::from_preflight()` ou helpers explícitos como `check_kvm() -> Result<(), LaunchError>`.

#### Tauri command signatures (RF-04)

```rust
#[tauri::command]
async fn launch_profile(app: AppHandle, name: String) -> Result<OperationResult, LaunchError>
```

Note: outros commands (stop, kill, etc.) continuam com `Result<_, String>` por enquanto. Migração progressiva fora deste PRD.

#### `profile-display.js` (RF-05)

```js
// src/profile-display.js
export function modeMeta(p) {
  const mode = p?.connect_mode || "rdp";
  return { label: mode === "rdp" ? "RDP" : "VNC", className: `is-${mode === "rdp" ? "rdp" : "web-vnc"}` };
}

export function profileState(p) {
  const s = (p?.container_state || "absent").toLowerCase();
  if (s.startsWith("up") || s === "running") return "running";
  if (s === "paused") return "paused";
  if (s === "exited" || s === "created" || s === "dead") return "exited";
  return "absent";
}

export function primaryActionMeta(p, t) {
  const state = profileState(p);
  if (state === "running") return { act: "launch", label: t("card.connect") };
  if (state === "paused")  return { act: "resume", label: t("card.resume") };
  return { act: "launch", label: t("card.launch") };
}
```

`src/main.js` substitui o corpo das três funções (linhas ~122-131) por `import { modeMeta, primaryActionMeta, profileState } from './profile-display.js'`. As funções existentes saem; nada mais quebra porque elas são pures (sem `this`, sem closure sobre globais).

#### Frontend toast estruturado (RF-04)

```js
// src/main.js

import { invoke } from '@tauri-apps/api/core';

async function launchProfile(name) {
  try {
    await invoke('launch_profile', { name });
  } catch (err) {
    renderLaunchError(err);
  }
}

function renderLaunchError(err) {
  // err é { code: "kvm_denied", port?: 3389, ... } ou string fallback
  if (typeof err === "string") return showErrorToast(err);
  const key = `launch.error.${err.code}`;
  const message = t(key, err);  // i18n interpolation
  showErrorToast(message, "error");
  // optional: button to copy hint command (i18n strings carry the command line embedded)
}
```

#### `.github/workflows/ci.yml` (RF-06)

```yaml
name: CI
on:
  pull_request:
  push:
    branches: [main]

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: install GTK/WebKit (Tauri build deps)
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev pkg-config
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy, rustfmt }
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            src-tauri/target
          key: ${{ runner.os }}-cargo-${{ hashFiles('src-tauri/Cargo.lock') }}
          restore-keys: ${{ runner.os }}-cargo-
      - run: cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
      - run: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
      - run: cargo test --manifest-path src-tauri/Cargo.toml

  js:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: npm ci
      - run: npm run check:js
      - run: npm run test:js
```

### Data Models

Sem migrations de DB (não há DB). Novos tipos serializáveis:

| Tipo | Onde | Shape exposto |
|---|---|---|
| `LaunchError` | `core::launch_error` | `{ "code": "...", ...variant-fields }` |

### API Endpoints

Não aplica — IPC Tauri. Comandos afetados:

| Command | Mudança |
|---|---|
| `launch_profile` | `Result<OperationResult, LaunchError>` em vez de `Result<OperationResult, String>` |

Tudo o resto inalterado.

## Integration Points

### Detecção de erros via stderr (RF-03)

O parsing fica em `CliDocker::compose_run` quando o exit code é non-zero. Substrings detectadas:

| Substring no stderr | LaunchError |
|---|---|
| `bind: address already in use` | `PortConflict { port: <extracted via regex \d+\.\d+\.\d+\.\d+:(\d+)> }` |
| `permission denied` + `/var/run/docker.sock` | `DockerDaemonDown` |
| `pull access denied` ou `not found: manifest` | `ImagePullFailed { image, stderr }` |
| `container name "<x>" is already in use` | `Other` (não deveria ocorrer — defensive rm_force cobre) |
| Outros | `Other { message: stderr_tail }` |

`check_kvm` retorna `KvmDenied` se `/dev/kvm` não existe ou não é acessível.

`require_installed` retorna `DockerMissing` se `docker` não está no PATH.

`check_daemon` retorna `DockerDaemonDown` se `docker info` falha.

`FreeRdpMissing` é checado em `commands::launch::launch_rdp` antes de chamar `rdp::launch`.

### CliDocker no Tauri lifecycle

Cada Tauri command instancia `CliDocker` no início (struct é unit, custo zero) e passa por referência:

```rust
#[tauri::command]
async fn launch_profile(app: AppHandle, name: String) -> Result<OperationResult, LaunchError> {
    let docker = CliDocker;
    // ...
    run_blocking(move || {
        match mode {
            ConnectMode::Rdp => commands::launch::launch_rdp(&profile_name, &docker),
            ConnectMode::WebVnc => commands::launch::ensure_for_web_vnc(&profile_name, &docker)
                .and_then(|port| open_web_vnc_window(&app_clone, &profile_name, port)),
        }
    })
    .await
}
```

CLI continua usando `CliDocker` direto.

## Testing Approach

### Unit Tests (Rust)

**`core::docker::tests`** (já existem 0; add ~3):

- `mock_records_calls` — instancia MockDocker, chama compose_run + rm_force, valida `mock.calls`.
- `cli_docker_propagates_unknown_error` — usa um stub que sempre falha; valida `LaunchError::Other`.

**`core::launch_error::tests`** (NOVO):

- `code_returns_stable_snake_case` — todas variantes têm `code()` único, snake_case, estável.
- `serializes_with_tag` — round-trip serde verifica `{"code":"port_conflict","port":3389}`.

**`commands::launch::tests`** (NOVO — RF-02):

- `ensure_running_paused_calls_unpause_only`
- `ensure_running_running_is_noop`
- `ensure_running_absent_calls_compose_up_only`
- `ensure_running_exited_rms_then_recreates` (ordem: rm_force ANTES de compose_run)
- `ensure_running_compose_failure_propagates_launch_error`
- `start_exited_rms_first` (mesma defesa pra start)
- `wait_for_windows_returns_timeout_windows_after_max_iters`

Total Rust: ≥12 atuais + ≥10 novos = ≥22.

### Unit Tests (JS) — RF-05

**`src/profile-display.test.js`** (NOVO):

- `modeMeta returns RDP for rdp connect_mode`
- `modeMeta returns VNC for web_vnc connect_mode`
- `modeMeta defaults to RDP when connect_mode is undefined`
- `profileState maps Up... to running`
- `profileState maps Exited to exited`
- `profileState maps undefined to absent`
- `primaryActionMeta returns connect for running state`
- `primaryActionMeta returns resume for paused state`

Total JS: 3 atuais + 8 novos = 11.

### Integration Tests

Não fazemos integration test real (container Docker) neste PRD — o trade-off é tempo de CI e flakiness. Os testes mock-based cobrem o branching critical. Integration real fica como backlog.

## Development Sequencing

### Build Order

1. **T1 — DockerClient trait + CliDocker** (RF-01). Bloqueia tudo. Pode shippar isoladamente — apenas refactor sem mudar comportamento.
2. **T2 — MockDocker em cfg(test)** (RF-01). Necessário pros tests do T3.
3. **T3 — LaunchError enum + launch_error.rs** (RF-03). Independent de T1/T2 do ponto de vista compile, mas conceitualmente segundo.
4. **T4 — Reescrita assinatura `commands::launch` para genérico + LaunchError** (RF-03). Consome T1+T3.
5. **T5 — Integration tests `ensure_running` / `start` com MockDocker** (RF-02). Consome T2+T4.
6. **T6 — Tauri command surface devolve LaunchError serializado** (RF-04). Consome T4.
7. **T7 — Extrair profile-display.js + testes** (RF-05). Independent — pode ser paralelo a tudo.
8. **T8 — renderLaunchError + i18n keys** (RF-04). Consome T6 (estrutura) + T7 não-bloqueador.
9. **T9 — CI workflow** (RF-06). Consome todos os anteriores estarem verdes.
10. **T10 — TROUBLESHOOTING.md PT-BR** (RF-07). Independent, last.
11. **T11 — docs/DEBUGGING.md EN-US** (RF-08). Independent, last.
12. **T12 — `cargo clippy -D warnings` clean** (RF-09). Pode rodar concorrente com T9 mas final.

### Technical Dependencies

- Sem deps novas.
- GitHub Actions runner precisa de pacotes `libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev` para `cargo test` linkar (Tauri faz cdylib).

## Monitoring and Observability

- Logs do `xfreerdp` em `~/.cache/winbox/rdp-<profile>.log` (já existe via commit 7f32333).
- `LaunchError::ContainerCrash` carrega `log_tail` (últimas ~50 linhas) — útil pro toast e pro futuro relatório de bug.
- Sem novo sink de telemetria neste PRD.

## Technical Considerations

### Key Decisions

| Decisão | Alternativa rejeitada | Razão |
|---|---|---|
| Trait em `core::docker`, MockDocker em `#[cfg(test)] mod` | Novo subdir `core::docker::*` | Evitar inchaço de arquivos pra ganho marginal |
| Generic `<D: DockerClient>` | `Box<dyn DockerClient>` | Monomorfização zero-cost; testes ficam rápidos |
| Trait i18n no frontend | Backend traduzido | Mantém Rust i18n-free; locale switch já é frontend-only |
| Stderr-parsing pra erros | Pre-check (lsof, stat) | Captura caso real (compose já tentou), sem custo extra |
| Sem cross-platform CI matrix | Matrix Win/macOS | Tauri build cross-OS é caro; é trabalho de release pipeline |
| Mock-only para tests | Container real (busybox) | Determinismo, sem daemon dependency em CI |

### Known Risks

- **Stderr-parsing pode pegar parcial**: regex `\d+\.\d+\.\d+\.\d+:(\d+)` pode falhar se compose mudar formato. Mitigação: fallback `Other { message }` cobre todos os casos não-classificados; testes garantem que pelo menos `bind: address already in use` rendena `PortConflict` corretamente.
- **Generic explosion**: passar `<D: DockerClient>` em muitas fns. Mitigação: limite a `commands::launch` e `commands::lifecycle`. Outros módulos não precisam mudar.
- **CI cold start**: primeiro PR em main pós-rollout não tem cache. Aceitável (~5min one-time).
- **`tauri-build` no CI**: precisa GTK/WebKit dev headers — adicionado no workflow. Pode falhar em runner não-ubuntu-latest. Mitigação: lock pin a `ubuntu-latest`.

### Standards Compliance

- `.dw/rules/` apenas tem README — sem regras específicas a cumprir. Estilo padrão Rust 2021 (rustfmt default).
- Sem dependências GPL — manter MIT-compatible.
- Sem secrets em código (já era prática; `0o600` em `config.env` documentado em `arch.md`).

### Relevant Files

- `src-tauri/src/core/docker.rs` — refactor + trait
- `src-tauri/src/core/launch_error.rs` — NOVO
- `src-tauri/src/commands/launch.rs` — generic refactor + LaunchError
- `src-tauri/src/commands/lifecycle.rs` — generic refactor
- `src-tauri/src/lib.rs` — Tauri command signatures
- `src/main.js` — import profile-display, renderLaunchError
- `src/profile-display.js` — NOVO
- `src/profile-display.test.js` — NOVO
- `src/locales/pt-BR.js` + `src/locales/en-US.js` — chaves launch.error.*
- `.github/workflows/ci.yml` — NOVO
- `TROUBLESHOOTING.md` — NOVO
- `docs/DEBUGGING.md` — NOVO

## Related ADRs

(none yet)

---
updated_at: "2026-07-08T14:39:06Z"
---

## Architecture Overview

Binário único Tauri 2 (GUI + CLI): `main.rs` roteia por argv. Camadas estritas sem ciclos:
`core/` (infra, Ce≈0) ← `commands/` (fluxos de negócio) ← `lib.rs` (31 comandos Tauri) e
`cli.rs` (clap, superfície `--json`/`--progress jsonl` consumida por clia/neodrive).
**Filesystem é o estado**: `config.env` por perfil (0600, fonte de verdade) +
`compose.yml` derivado (sempre regenerado) em `$XDG_CONFIG_HOME/winbox/profiles/<p>/`;
disco da VM em `$XDG_DATA_HOME`. Sem banco, sem tokio (blocking via `spawn_blocking`),
sem framework de log. Frontend vanilla JS: hub `src/main.js` com re-render total por
polling 3s + eventos push `operation-progress`; escape obrigatório via `dom-utils.js`.

## Key Components

| Component | Path | Responsibility |
|-----------|------|----------------|
| Adaptação Tauri | `src-tauri/src/lib.rs` | 31 comandos, run_blocking, emit_progress, actionable_error |
| CLI | `src-tauri/src/cli.rs` | clap; dispatch humano + dispatch_json (duplicados) |
| Fluxos | `src-tauri/src/commands/` | install, launch (máquina de estados), set (tri-state), lifecycle |
| Docker | `src-tauri/src/core/docker.rs` | trait DockerClient + CliDocker + MockDocker; classify_compose_stderr |
| Compose | `src-tauri/src/core/compose.rs` | 4 templates YAML via format! por ImageFamily (+ bloco GPU/VFIO) |
| Validação | `src-tauri/src/core/validation.rs` | TODA validação (nome, env anti-injeção, drvfs, recursos) |
| Erro estruturado | `src-tauri/src/core/launch_error.rs` | 16 variantes `{code: snake_case}` → i18n no front |
| Paths (god node Ca=23) | `src-tauri/src/core/paths.rs` | XDG/AppData + pins dockurr/windows:5.14, qemux/qemu:7.29 |
| Hub frontend | `src/main.js` | estado global, templates, invoke(), listeners delegados (1734 linhas, sem testes) |
| Design authority | `DESIGN.md` | v2 "Ops Premium Clean" — tokens/layout/a11y canônicos |

## Data Flow

Launch: `click [data-act] (main.js)` → `invoke launch_profile (lib.rs)` → `spawn_blocking` →
preflights (kvm/docker) → `ensure_started (commands/launch.rs)`: status → rm -f stale →
`docker compose up` → wait (TCP ou grep "windows started successfully" nos logs) →
abre noVNC `127.0.0.1:<WEB_PORT>` no browser padrão (WebKitGTK não entrega input ao canvas) →
progresso via evento `operation-progress` → erro vira `LaunchError {code}` → i18n.

Install: validação (validation.rs) → alocação de portas (bind real) → config.env 0600 →
compose::write → OEM firstlogon.ps1 + bundles → up. cloud-init (linux_cloud): download
Ubuntu image + SHA256 + seed ISO via genisoimage dentro da imagem qemux.

## Conventions

- Commits: Conventional Commits com escopo (`feat(windows):`, `fix(ui):`); branches `feat/`, `chore/`.
- Rust: snake_case, mensagens de usuário PT-BR; erros `anyhow` + `{e:#}` na borda; funções puras
  extraídas para teste sem I/O; testes inline `#[cfg(test)]`; mock por trait com gravação de chamadas.
- Códigos de erro estáveis snake_case (`wsl_distro_down`); serde alias camelCase/snake_case nas args.
- JS: templates `*Template`, delegação por `data-act`/`data-i18n`, escapeHtml/escapeAttr em TODO
  dado dinâmico, i18n dot-namespace com paridade en-US/pt-BR (224 chaves), zero console.*.
- Regra dura: nunca mudar VERSION/LANGUAGE de perfil com disco instalado (dockur reinstala).
- Portas sempre 127.0.0.1; senha só em config.env 0600; capabilities Tauri mínimas (sem shell).

# Rules do Projeto — winbox-gui

> Auto-gerado por `/dw-analyze-project` em 2026-07-08. Baseline declarativa da stack: ver
> `.dw/rules-library/common.md` + `.dw/rules-library/rust.md` (referência, não cópia).

## Visão Geral

GUI **e** CLI em Tauri 2 para gerenciar VMs Windows/Linux via Docker/QEMU (imagens
`dockurr/windows` e `qemux/qemu`). Um único binário decide CLI vs GUI pela presença de argv.
O app lista perfis, mostra status ao vivo, cria perfis, abre o viewer noVNC no navegador,
altera recursos, aplica bundles PowerShell, gerencia GPU VFIO, expõe snapshots e diagnostica
o host antes de ações críticas. **Não há banco de dados: o filesystem é o estado**
(`$XDG_CONFIG_HOME/winbox/profiles/<nome>/config.env` + `compose.yml`; disco da VM em
`$XDG_DATA_HOME/winbox/profiles/<nome>/storage`).

Produto **independente**, consumido programaticamente por **clia** e **neodrive** (via CLI
`--json` / `--progress jsonl` e tarballs de release). Frente ativa: perfil Office + WinApps
como bundle instalável (one-pager em `.dw/spec/ideas/winbox-office-instalavel.md`;
protótipo bash em `scripts/setup-host-excel.sh`).

## Estrutura

Projeto único (app Tauri = wrapper npm + crate Rust). Sem monorepo, sem git submodules.

### Árvore de Projetos

```
winbox-gui/
├── src/                      [Frontend HTML+CSS+JS vanilla, ES modules, sem framework]
│   └── locales/              [i18n en-US + pt-BR, 224 chaves com paridade 1:1]
├── src-tauri/                [Backend Rust: Tauri 2 + clap; binário único GUI+CLI]
│   ├── src/main.rs           [argv>1 → cli::run(), senão GUI]
│   ├── src/lib.rs            [31 comandos Tauri + eventos operation-progress]
│   ├── src/cli.rs            [CLI clap com --json e --progress jsonl]
│   ├── src/commands/         [fluxos de alto nível: install, launch, set, lifecycle...]
│   ├── src/core/             [infra: docker, compose, profile, validation, health...]
│   └── assets/               [bundles .ps1 + templates OEM embutidos via include_dir]
├── tests/e2e/                [WebdriverIO + tauri-driver (INOPERANTE hoje — ver concerns)]
├── scripts/                  [setup-host-excel.sh — provisionamento host Office/WinApps]
├── tools/windows-spike/      [Exploração PowerShell do port Windows; fora do CI]
└── .github/workflows/        [ci.yml, release-cli.yml, windows-build.yml]
```

### Índice de Projetos

| Projeto | Caminho | Stack | Tipo | Rules |
|---------|---------|-------|------|-------|
| Backend Rust | `src-tauri/` | Rust + Tauri 2 + clap 4 | app (GUI+CLI) | [backend-rust.md](backend-rust.md) |
| Frontend | `src/` | HTML+CSS+JS vanilla (ES modules) | app (webview) | [frontend-js.md](frontend-js.md) |
| Infra/CI/Scripts | `.github/`, `scripts/`, `tools/` | GitHub Actions + bash + PowerShell | infra | [infra-ci.md](infra-ci.md) |

### Matriz de Dependências

| Projeto | Depende de | Dependido por |
|---------|-----------|---------------|
| Frontend (`src/`) | Backend via `window.__TAURI__` (invoke + eventos) | — |
| Backend (`src-tauri/`) | Docker/Compose do host, imagens dockurr/qemux | Frontend, CLI consumers (clia, neodrive), setup-host-excel.sh |
| Infra/CI | Backend (builda/testa/release) | clia/neodrive (tarballs), setup-host-excel.sh |

### Padrões de Comunicação

- **Frontend ↔ Backend:** exclusivamente Tauri IPC — `invoke(cmd, args)` (31 comandos) e
  eventos `operation-progress` / `profile-error`. Detalhes em [integrations.md](integrations.md).
- **Backend ↔ VMs:** `docker compose` via `std::process::Command` sobre compose.yml gerado
  em runtime; contrato de env vars com dockurr/qemux (ver integrations.md).
- **Consumidores externos (clia/neodrive):** CLI `--json` (envelope `{ok, value|error}`) +
  `--progress jsonl`; tarballs `winbox-<triple>.tar.gz` do GitHub Release.
- **WinApps/FreeRDP (frente Office):** o backend NÃO conhece FreeRDP; publica `RDP_PORT`
  (3389 tcp+udp em 127.0.0.1) que o WinApps externo consome.

## Stack Tecnológica

| Aspecto | Tecnologia |
|---------|------------|
| Linguagem backend | Rust (edition 2021; anyhow, serde, clap 4, sysinfo, include_dir, dialoguer) |
| Framework | Tauri 2 (CSP restritiva, capabilities mínimas: core:default + dialog:default) |
| Frontend | HTML + CSS + JavaScript vanilla — sem TypeScript, sem bundler, sem framework |
| Banco de Dados | Nenhum — filesystem é o estado (config.env por perfil) |
| Testes | Rust: `#[cfg(test)]` inline (108 testes) · JS: node:test · E2E: WebdriverIO + tauri-driver |
| CI/CD | GitHub Actions (ci.yml, release-cli.yml, windows-build.yml) |
| Linter/Formatter | `cargo fmt` + `cargo clippy -D warnings` (gates de CI) · JS: só `node --check` (sem lint) |
| Containerização | Nenhuma do app; compose.yml das VMs é GERADO em runtime (`core/compose.rs`) |
| Monorepo tools | N/A |
| i18n | `src/i18n.js` + `src/locales/{en-US,pt-BR}.js`, fallback en-US, chaves dot-namespace |
| Design | `DESIGN.md` na raiz (v2 "Ops Premium Clean") — autoridade viva, tokens batem com `styles.css` |

## Convenções Git

- Estilo de commit: **Conventional Commits com escopo** — `feat(windows):`, `fix(ui):`,
  `ci(windows):`, `chore(dw):`, `docs(planning):`, `test(e2e):`.
- Padrão de branch: `feat/…`, `fix/…`, `chore/…`, `ci/…` (kebab-case descritivo).
- PR template: nenhum. Merge via PR no GitHub (`brunosps/winbox-gui`), main protegida por convenção.

## Submodules

Nenhum detectado.

## Comandos Úteis

| Comando | Descrição |
|---------|-----------|
| `npm run dev` | tauri dev — GUI + backend com frontend estático de `src/` |
| `npm run build` | tauri build — bundles em `src-tauri/target/release/bundle/` |
| `npm test` | test:js (node:test) + test:rust (cargo test, 108 testes, sem Docker — mocks por trait) |
| `npm run check:js` | `node --check` por arquivo JS (só sintaxe — não é lint) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Gate de lint do CI |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Gate de formatação do CI |
| `npm run test:e2e` | seed + WebdriverIO (requer tauri-driver; suíte hoje INOPERANTE — seletores stale) |
| `./src-tauri/target/release/winbox-gui --json list` | Superfície CLI machine-readable |
| `bash scripts/setup-host-excel.sh` | Provisiona host Linux p/ Excel via winbox + WinApps |
| `docker logs winbox-<perfil>` | Log de evidência primário de qualquer VM |

## Referência Rápida

- [backend-rust.md](backend-rust.md) — camadas core←commands←{cli,lib}, LaunchError, trait DockerClient, topologia.
- [frontend-js.md](frontend-js.md) — hub main.js, escape obrigatório, i18n, re-render por polling 3s.
- [infra-ci.md](infra-ci.md) — workflows, release por tag `v*`, setup-host-excel.sh, spike Windows.
- [integrations.md](integrations.md) — contratos: Tauri IPC, dockurr/qemux, WinApps/FreeRDP, clia/neodrive.
- [concerns.md](concerns.md) — mapa de riscos (onde é perigoso mexer).
- `DESIGN.md` (raiz) — autoridade de design do frontend; **não** criar direção visual fora dele.
- Baseline declarativa: `.dw/rules-library/common.md` + `.dw/rules-library/rust.md`.

# Rules — Infra, CI/CD e Scripts

> Auto-gerado por `/dw-analyze-project` em 2026-07-08.

## Arquitetura

3 workflows GitHub Actions com papéis separados + 1 script de provisionamento de host + tooling
de spike. **Não existe Dockerfile nem docker-compose versionado** — o compose.yml de cada VM é
GERADO em runtime pelo app (`src-tauri/src/core/compose.rs`); o repo versiona o gerador, não o
YAML. O único compose estático é o fixture de teste (`tests/e2e/fixtures/seed.sh`, busybox).

## Estrutura de Diretórios

```
.github/workflows/
├── ci.yml              # PR + push main: jobs rust (fmt/clippy/test, 25min) e js (check/test, 5min)
├── release-cli.yml     # tag v* / dispatch: matriz 3 OS → tarballs CLI anexados ao release
└── windows-build.yml   # push/PR main + tag: cargo tauri build → msi+nsis (attach só em tag)
scripts/
└── setup-host-excel.sh # frente ativa: provisiona host Linux p/ Excel via winbox+WinApps (144 linhas)
tools/windows-spike/    # exploração PowerShell do port Windows; NÃO integra CI; links de contexto quebrados
docs/
├── DEBUGGING.md        # contributor (en): logs, XDG paths, tabela LaunchError (parcialmente STALE)
└── E2E-SETUP.md        # setup tauri-driver+wdio; admite gaps (mock Docker, E2E fora do CI)
```

## Contexto do Projeto

- **Tipo:** infra · **Depende de:** backend (builda/testa/release)
- **Dependido por:** clia/neodrive (tarballs de release), setup-host-excel.sh (download do binário)

## Padrões de Código

### Nomenclatura

Workflows e scripts kebab-case; env vars UPPER_SNAKE (`WINBOX_VERSION`, `INSTALL_DIR`); funções
bash snake_case (`apt_install_one`, `log`/`warn`/`die`); PowerShell Verb-Noun (`Winget-Install`,
`Maybe-Reboot`); artefatos `winbox-<target-triple>.tar.gz`; scripts npm prefixados por linguagem
(`check:js`, `test:rust`). Docs de usuário em CAPS na raiz; docs de contributor em `docs/`.

### Tratamento de Erros

Três estilos por camada:
- **Bash:** `set -euo pipefail` + `log`/`warn`/`die` — preflights fatais com `die` (root, sudo,
  x86_64, VT-x, /dev/kvm, download); instalações best-effort com `|| warn` (**exit 0 não garante
  host funcional** — ver concerns.md). Cleanup via `trap ... EXIT`.
- **PowerShell (spike):** `$ErrorActionPreference = "Continue"` deliberado — o spike COLETA
  falhas (test-results.txt), não para nelas; estado em spike.state.json + RunOnce no registry.
- **CI:** gates duros — `clippy -D warnings`, `fmt --check`, `if-no-files-found: error`,
  `fail-fast: false` na matriz, timeouts explícitos (5min js / 25min rust / 40min release).

### Testes

CI roda: `cargo test`, `npm run check:js`, `npm run test:js`. **E2E NÃO roda em CI** (exigiria
xvfb + webkit2gtk-driver — gap documentado em docs/E2E-SETUP.md). Sem testes para workflows,
setup-host-excel.sh ou scripts do spike.

## Fluxos

### Release do CLI (tag v* → tarballs consumidos por clia/neodrive)

1. **Trigger:** `release-cli.yml` — tag `v*` ou dispatch; comentário formaliza: "These tarballs
   are what downstream apps (e.g. clia.local) bundle via their prepare script"
2. **Matriz** (`fail-fast: false`): ubuntu→x86_64-linux-gnu, windows→x86_64-msvc (.exe),
   macos→aarch64-apple-darwin
3. **Build:** `cargo build --release --target <triple>` (binário puro, sem bundle Tauri)
4. **Package:** `tar -czf winbox-<triple>.tar.gz` só com o binário
5. **Publicação dupla:** upload-artifact sempre + `softprops/action-gh-release@v2` anexa SÓ em tag
6. **Consumo:** `setup-host-excel.sh:85-100` baixa via `curl -fL --retry 3`, instala em
   `~/.local/share/winbox/winbox`, symlink `~/.local/bin/winbox` — **sem checksum/assinatura**

### Installers Windows GUI (mesma tag → msi + nsis)

`windows-build.yml`: roda em TODO push/PR na main (sem filtro de paths — custo alto de runner) +
tags; `cargo install tauri-cli` a cada run; `cargo tauri build --target x86_64-pc-windows-msvc`;
anexa msi/nsis ao MESMO release da tag. Uma tag `v*` publica: 3 tarballs CLI + installers Windows.

### CI de qualidade (PR/push main)

Job rust: deps GTK/WebKit → fmt --check → clippy -D warnings → cargo test (sequencial).
Job js: Node 20 → npm ci → check:js → test:js. `permissions: contents: read`.

### Provisionamento do host Office/WinApps (frente ativa)

`scripts/setup-host-excel.sh` ponta a ponta:
1. **Preflight fatal:** recusa root, exige sudo, x86_64, `vmx|svm` no cpuinfo, `/dev/kvm`
2. **Deps:** libs Tauri + Docker + Compose via `apt_install_one` (fallback de nomes t64 do 24.04)
3. **FreeRDP flatpak:** Flathub + `com.freerdp.FreeRDP` (3.27+, contorna bug RAIL "X_CopyArea
   BadMatch" do freerdp3-x11 3.5.1 do Ubuntu 24.04) + `flatpak override --filesystem=home`
4. **Docker:** `systemctl enable --now` + grupos docker/kvm (avisa relogin)
5. **winbox do release:** baixa tarball (default hardcoded `v0.1.0`, sobrescrevível por
   `WINBOX_VERSION`/`WINBOX_REPO`) + `.desktop` no menu
6. **WinApps:** `git clone --depth 1` do upstream em `~/code/winapps` (copyleft — NUNCA embutir)
7. **Handoff manual impresso:** criar perfil + Office via noVNC; RDPApps.reg; `winapps.conf` com
   `RDP_USER/RDP_PASS/RDP_IP=127.0.0.1/RDP_PORT/WAFLAVOR="manual"/FREERDP_COMMAND=flatpak...`;
   `./setup.sh --user --setupAllOfficiallySupportedApps`

**Gap conhecido:** `RDP_FLAGS` (`/cert:ignore +home-drive`), o bundle custom `msoffice` com
OfficeSetup.exe e a associação xls/xlsx no Thunar NÃO existem no repo — são configuração viva do
host do dono, ainda não absorvida pelo produto. É exatamente o que o one-pager
`.dw/spec/ideas/winbox-office-instalavel.md` propõe fechar (o script é o protótipo bash do wizard).

## Padrões de Segurança

- Workflows com `permissions` mínimos explícitos; `contents: write` só nos de release; único
  third-party na cadeia: `softprops/action-gh-release@v2`. Sem secrets customizados.
- setup-host-excel.sh recusa root; sudo pontual; flatpak FreeRDP ganha `--filesystem=home`
  (concessão de sandbox necessária ao `+home-drive` do WinApps).
- **Releases sem checksums nem assinatura** — binário baixado e executado via curl sem verificação.
- Sem dependabot/renovate para as actions.

## Infraestrutura

Produto desktop distribuído por release — sem staging/prod, sem deploy contínuo. "Ambiente" é o
host do usuário (alvo: Linux Mint 22.x/Ubuntu 24.04, x86_64, KVM). Cache cargo via
actions/cache@v4 chaveado em Cargo.lock em todos os workflows; Node 20 fixado.

## Padrões de Performance

CI: caches cargo com keys distintas (`-cargo-` vs `-cargo-release-`), timeouts calibrados,
`fail-fast: false`. Ponto fraco: windows-build.yml reinstala tauri-cli a cada run e roda em todo
PR sem filtro de paths.

## Observabilidade

CI: só logs do Actions (sem coverage/relatórios estruturados). Script de host: prefixos coloridos
`[setup]/[aviso]/[erro]` em stdout, sem arquivo de log. Spike: spike.log timestamped +
test-results.txt (coletor de evidências).

## Papel do spike (tools/windows-spike/)

Exploração viva-mas-órfã do port Windows — nada o referencia em CI/build; seus links de contexto
(`.dw/spec/prd-windows-port/{SPIKE,PLAN}.md`) apontam para diretório removido (commit f737289).
`windows-build.yml` é o sucessor produtizado. `winbox-spike.ps1` é código hostil: máquina de
estados persistida + auto-resume via RunOnce + `Restart-Computer -Force` — mexer sem entender o
ciclo pode deixar um host Windows em loop de reboot. Decisão pendente: restaurar docs, arquivar
o spike ou consertar os links.

## Docs

- `TROUBLESHOOTING.md` (usuário, pt-BR, sintoma→diagnóstico→fix) — excelente e atual.
- `docs/DEBUGGING.md` (contributor, en) — **parcialmente STALE:** documenta `FreeRdpMissing`/
  `preflight_freerdp`/`rdp-<profile>.log`, removidos do código (docker.rs:630). Nuance: o FreeRDP
  voltou ao stack pela porta do WinApps (RemoteApp), não do viewer — o viewer é noVNC no browser.
- `docs/E2E-SETUP.md` — honesto sobre os gaps (mock Docker backlog, E2E fora do CI).

## Variáveis de Ambiente

| Nome | Descrição |
|------|-----------|
| WINBOX_VERSION / WINBOX_REPO | Tag e repo do release a baixar no setup-host-excel.sh (default v0.1.0 hardcoded) |
| INSTALL_DIR / BIN_DIR / WINAPPS_DIR | Destinos de instalação no host |
| RUST_LOG | Previsto em docs/DEBUGGING.md; tracing-subscriber ainda não wired (backlog) |
| RDP_USER / RDP_PASS / RDP_IP / RDP_PORT / WAFLAVOR / FREERDP_COMMAND | Chaves do `~/.config/winapps/winapps.conf` (fora do repo), preenchidas manualmente |

## Comandos

| Comando | Descrição |
|---------|-----------|
| `bash scripts/setup-host-excel.sh` | Provisiona host p/ Excel via winbox+WinApps |
| `gh workflow run release-cli.yml` | Release manual (workflow_dispatch) |
| `git tag v0.x.y && git push --tags` | Dispara release completo (CLI 3 OS + installers Windows) |
| `.\winbox-spike.ps1 [-Status\|-SkipReboot]` | Spike Windows (idempotente, sobrevive a reboot) |

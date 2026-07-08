# Rules — Backend Rust (src-tauri)

> Auto-gerado por `/dw-analyze-project` em 2026-07-08. Baseline declarativa: `.dw/rules-library/rust.md`.

## Arquitetura

Camadas estritas com injeção de dependência por trait para o Docker:

1. **Entrada dupla** — `src/main.rs` (10 linhas): `argv > 1 → cli::run()`, senão GUI (`winbox_gui_lib::run()`).
2. **Adaptação Tauri** — `src/lib.rs` (913 linhas): registra 31 comandos via `tauri::generate_handler!`;
   só orquestra (resolve perfil, emite progresso, delega para `spawn_blocking`). Sem estado gerenciado
   (`.manage()` não é usado); cada comando relê o filesystem.
3. **Fluxos de alto nível** — `src/commands/{install,launch,set,lifecycle,list,reapply,logs}.rs`.
4. **Infraestrutura** — `src/core/` (~26 módulos): docker.rs (wrapper do CLI docker), compose.rs
   (geração do compose.yml), profile.rs (store filesystem), validation.rs (TODAS as validações),
   health.rs (diagnóstico do host), launch_error.rs (erro estruturado), paths.rs (rotas XDG/AppData).

Ponto de extensão central: trait `DockerClient` (core/docker.rs:36) com `CliDocker` real e
`MockDocker` para testes; wrappers de função livre (docker.rs:537-568) preservam a API antiga.
**Regra da casa:** extrair função pura para tudo que precisa de teste sem I/O
(`classify_compose_stderr`, `parse_pull_progress_line`, `user_data`...).

## Estrutura de Diretórios

```
src-tauri/
├── Cargo.toml              # crate winbox-gui, lib winbox_gui_lib; release: LTO, strip, panic=abort
├── tauri.conf.json         # CSP restritiva, frontendDist ../src, bundle targets "all"
├── capabilities/default.json  # permissões mínimas: core:default + dialog:default (SEM shell)
├── assets/
│   ├── bundles-builtin/    # bundles .ps1 embutidos no binário (include_dir)
│   └── templates/          # firstlogon-header/footer.ps1, install.bat
└── src/
    ├── main.rs             # roteador CLI vs GUI
    ├── lib.rs              # 31 comandos Tauri, emit_progress, run_blocking, actionable_error
    ├── cli.rs              # 1124 linhas: clap, dispatch + dispatch_json (duplicados), install interativo
    ├── commands/           # install / launch / set / lifecycle / list / reapply / logs
    └── core/               # docker, compose, profile, validation, launch_error, paths, env_file,
                            # health, cloud_init, bootstrap, gpu*, vfio_setup, snapshots, connect,
                            # image_family, ports, host, oem, bundles, opener, desktop,
                            # health_wsl, wsl_autostart, wsl_config_writer (port Windows/WSL)
```

## Contexto do Projeto

- **Localização:** `src-tauri/` · **Tipo:** app (binário único GUI+CLI) · **Submodule:** não
- **Depende de:** Docker/Compose do host, `/dev/kvm`, imagens pinadas `dockurr/windows:5.14` e
  `qemux/qemu:7.29` (paths.rs:6-7 — "Upgrade by bumping these constants intentionally")
- **Dependido por:** frontend (`src/`), consumidores CLI (clia, neodrive), `scripts/setup-host-excel.sh`

## Padrões de Código

### Nomenclatura

- Módulos/funções snake_case inglês; **mensagens de usuário em PT-BR** (`"Perfil '{}' não existe."`).
- Comandos Tauri verbo_substantivo: `launch_profile`, `install_profile`, `gpu_setup_apply`.
- Args do frontend aceitam camelCase + alias snake_case: `#[serde(rename = "gpuBdf", alias = "gpu_bdf")]`.
- Códigos de erro estáveis snake_case: `wsl_distro_down`, `container_oom_killed`, `port_conflict`.
- Container `winbox-<perfil>`; chaves do config.env em SCREAMING_SNAKE (RAM_SIZE, GPU_BDF).
- Prefixo `preflight_*` = check que retorna `LaunchError`; `check_*`/`require_*` = retorna anyhow.

### Tratamento de Erros

Dois regimes coexistem:

**(a) Regime geral:** `anyhow::Result` interno com `bail!`/`.with_context()`, convertido na borda
Tauri para `Result<T, String>` com a cadeia completa (`format!("{e:#}")`):

```rust
// src-tauri/src/lib.rs
async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}
```

`actionable_error` (lib.rs:165) injeta a dica `"Próxima ação: ..."` por matching de substring —
**atenção: o frontend depende dessa frase literal** (contrato textual frágil, ver concerns.md).

**(b) Regime estruturado (só launch):** enum `LaunchError` com 16 variantes serializadas como
`{"code": "snake_case", ...campos}` — o frontend resolve i18n por `code`:

```rust
// src-tauri/src/core/launch_error.rs
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum LaunchError {
    KvmDenied,
    DockerMissing,
    DockerDaemonDown,
    PortConflict { port: u16 },
    // ... wsl_distro_down { distro }, container_oom_killed { container, mem_limit }, disk_full { path, needed_bytes }
}
```

`launch_profile` é o ÚNICO comando com erro estruturado; os demais retornam `Result<_, String>`
(assimetria conhecida — install/update/snapshot não são tratáveis por código no front). O stderr
do compose é classificado na função pura `classify_compose_stderr` (docker.rs:255). Best-effort
deliberado com `let _ =` é aceito quando comentado (lifecycle.rs kill/remove).

### Padrão de API

Superfície tripla sobre o mesmo core — ver [integrations.md](integrations.md) para a lista dos
31 comandos Tauri, os 2 eventos e o contrato CLI `--json`/`--progress jsonl`.

### Validação

Centralizada em `core/validation.rs`, invocada na camada commands ANTES de qualquer efeito
colateral. O frontend NUNCA é confiado — tudo revalidado no Rust:

- Nome de perfil: `[a-z0-9][a-z0-9_-]*` (anti-traversal); snapshot bloqueia `.`/`..`.
- `validate_env_value` rejeita `\n \r \0` (anti-injeção de chaves no config.env).
- RAM 1..1024G, CPU 1..512, disco 8..8192G, normalizados para `NG`; `MEM_LIMIT = ram+2G`.
- BDF PCI hex estrito `0000:01:00.0`; extra_ports parseadas para struct `PortForward` tipada.
- Storage path: absoluto, **rejeição hard de `/mnt/<letra>` (drvfs ~30× mais lento)**, mkdir +
  probe de escrita + espaço livre ≥ DISK_SIZE via `df`:

```rust
// src-tauri/src/core/validation.rs
if is_wsl_drvfs(path) {
    bail!(
        "'{v}' está em /mnt/ (drvfs do Windows), que é lento demais para discos de VM \
         (~104 MB/s vs. ~3 GB/s no disco da distro). Escolha um caminho dentro do WSL, \
         como /home/bruno/winbox-disks — ele grava no mesmo disco físico, mas rápido."
    );
}
```

### Testes

Unitários inline `#[cfg(test)] mod tests` no fim do arquivo — 16 de 38 arquivos, 108 `#[test]`.
Sem diretório `tests/` (sem integração separada). Mock por trait gravando chamadas em ordem:

```rust
// src-tauri/src/commands/launch.rs
#[test]
fn ensure_running_paused_calls_unpause_only() {
    let docker = MockDocker::new();
    let profile = "paused-prof";
    let c = container_name(profile);
    docker.seed_status(&c, "paused");
    ensure_running(profile, &docker).unwrap();
    let calls = docker.calls();
    assert_eq!(calls, vec![format!("status:{c}"), format!("unpause:{c}")]);
}
```

Cobertura forte: validation, docker (parsing), launch_error (roundtrip serde), bootstrap,
cloud_init. **Zero testes:** gpu.rs (parser lspci), snapshots.rs, ports.rs, vfio_setup.rs,
env_file.rs, set.rs. Teste confessadamente vazio: `wait_for_windows_times_out_to_timeout_windows`
(launch.rs:363 — POLL_ITERS não é injetável).

## Exemplos de Fluxo de Requisição

### Launch de perfil (GUI)

1. **Entrada:** `lib.rs:299` — `launch_profile(app, name)` → `Result<OperationResult, LaunchError>`
2. **Resolução:** `core/profile.rs:52` — nome explícito, default ou perfil único
3. **Progresso:** `lib.rs:145-163` — evento `operation-progress` `{profile, op, step, status, message}`
4. **Thread:** `lib.rs:333` — todo trabalho blocking em `spawn_blocking` (nada de docker no event loop)
5. **Preflights:** `core/docker.rs:601-628` — KVM, docker binário, daemon
6. **Máquina de estados:** `commands/launch.rs:56-92` — `ensure_started()`: paused→unpause,
   running→noop, absent→compose up, exited/dead→**rm -f + compose up** (container_name hardcoded colide com stale)
7. **GPU gate:** `core/gpu_hooks.rs:30` — falha rápido se GPU_BDF configurado sem driver vfio-pci
8. **Compose:** `core/docker.rs:93-123` — `docker compose -p winbox-<p> --env-file config.env up -d`;
   stderr classificado por `classify_compose_stderr`
9. **Espera:** `wait_for_web_port` (TCP 60×1s) ou `wait_for_windows` (grep "windows started
   successfully" em docker logs, 120×2s) → `TimeoutLinux`/`TimeoutWindows`
10. **Viewer:** abre `http://127.0.0.1:<porta>/?autoconnect=true&resize=scale` no BROWSER padrão
    (WebKitGTK não entrega teclado/mouse ao canvas noVNC no webview — workaround permanente)
11. **Erro:** LaunchError serializado `{code}` → frontend renderiza hint i18n

### Criação de perfil (install)

1. **Entrada:** `lib.rs:571` — `install_profile(params)` (aliases serde) → `commands/install.rs:56`
2. **Validação:** nome → família (Windows exige version; linux_distro exige BOOT em
   SUPPORTED_DISTROS; linux_iso exige ISO absoluta) → env fields → senha (só Windows) → recursos
3. **Portas:** `core/ports.rs:44` — varre config.env de todos os perfis + testa bind TCP/UDP real,
   incrementando de WEB=8006/RDP=3389/SSH=2222
4. **Escrita:** config.env com 27 chaves + **chmod 0600** (senha em plaintext) → `compose::write()`
   renderiza template por ImageFamily → OEM (Windows): install.bat + firstlogon.ps1 com bundles
5. **cloud-init (linux_cloud):** baixa Ubuntu cloud image (SHA256 verificado), qemu-img resize,
   seed ISO NoCloud via genisoimage DENTRO da imagem qemux
6. **Resposta:** erro em qualquer etapa volta como String actionable via `emit_operation_result`

### Edição de perfil (set)

1. `lib.rs:681` `update_profile` → `commands/set.rs:23` — semântica tri-state: `None`=não toca,
   `Some("")`=limpa, `Some(v)`=valida e grava
2. `core/env_file.rs:29` `set_key()` — reescreve SÓ a linha `KEY=`, preservando ordem e comentários
3. `compose::write()` sempre; `restart=true` → down → gpu_hooks → up
4. **set.rs deliberadamente NÃO expõe VERSION/LANGUAGE** — mudar com disco instalado faz o dockur
   reinstalar do zero.

## Padrões de Segurança

- **Autenticação/Autorização:** N/A — app desktop local single-user.
- **Secrets:** senha Windows em plaintext no config.env mitigada com chmod 0600 (unix);
  `get_profile_config` NUNCA devolve a senha ao front (`password: String::new()`, lib.rs:665).
- **Rede:** todas as portas publicadas presas em `127.0.0.1` nos templates compose.
- **Escalação:** scripts VFIO via `pkexec` (fallback `sudo -n`); inputs interpolados passam por
  `validate_bdf`/`normalize_id` antes — superfície de injeção controlada, mas existe.
- **Supply chain:** cloud image verificada contra SHA256SUMS oficial; imagens Docker pinadas por
  tag com teste de regressão contra `:latest` (compose.rs:389-422).
- **Tauri:** CSP restritiva (connect-src ipc + 127.0.0.1:*; object-src 'none'); capabilities
  mínimas; frontend sem permissão de shell.
- **Anti-destrutivo:** bootstrap WSL nunca sobrescreve distro existente sem `force`
  (`NeedsConfirmation`, bootstrap.rs:334 — cicatriz de bug que apagou WSL de usuário).

## Infraestrutura

- Sem containerização do app; compose.yml das VMs gerado em runtime (`core/compose.rs` — 4
  templates `render_*` por ImageFamily via `format!`, indentação manual; só iso_path escapado).
- CI: fmt + clippy `-D warnings` + test (ver [infra-ci.md](infra-ci.md)).
- Build release: panic=abort, LTO, codegen-units=1, opt-level='s', strip; assets embutidos
  (include_dir) — binário único auto-contido.

## Padrões de Performance

- Tudo síncrono sobre `std::process::Command` — **não há tokio**; comandos Tauri são `async fn`
  apenas para `spawn_blocking` (não travar o event loop; `docker stop -t 120` pode levar minutos).
- Polling com `std::thread::sleep`: boot Windows 120×2s (re-lê o log INTEIRO do container a cada
  iteração — custo cresce com o log); porta web 60×1s.
- Otimizações: `web_port_reachable` antes de esperar; cache OnceLock do binário compose;
  `docker pull` streaming com parse de progresso por camada; snapshots com `cp --sparse=always`.
- Não detectado: cache de status de containers (`list()` = N processos docker para N perfis).

## Observabilidade

- **Sem framework de logging** (nenhum log/tracing no Cargo.toml; tracing-subscriber é backlog
  documentado em docs/DEBUGGING.md).
- Observabilidade voltada ao usuário: eventos `operation-progress` (o "trace" da GUI); CLI
  `--progress jsonl` timestamped RFC3339; `docker logs` repassado cru; `host_health` com 8 checks
  `{status, message, detail, action_hint}` e overall = pior check; `bootstrap_status` (Windows).
- Não detectado: Sentry/telemetria, métricas, log em arquivo.

## Contratos de API

- Sem OpenAPI/GraphQL — o contrato é: comandos Tauri (lib.rs) + envelope CLI JSON + códigos de
  erro snake_case estáveis. Ver [integrations.md](integrations.md).
- Versionamento: tag `v*` no repo; sem versão de schema no config.env (perfis antigos dependem
  de `env_file::get` retornar `""` como default).

## Análise de Topologia

### Grafo de Dependências (módulos mais conectados)

```
cli        → commands/* (7), core/{bundles,connect,docker,env_file,gpu,health,host,image_family,opener,paths,profile,snapshots,vfio_setup}
lib        → core/{bootstrap,bundles,connect,docker,env_file,gpu,health,host,image_family,launch_error,opener,paths,profile,snapshots,vfio_setup}
commands/install → core/{cloud_init,compose,desktop,docker,gpu_hooks,host,image_family,oem,paths,ports,profile,validation}
commands/launch  → core/{connect,docker,env_file,gpu_hooks,image_family,launch_error,opener,paths}
core/health      → core/{docker,env_file,gpu,paths,ports,profile}
core/compose     → core/{env_file,image_family,paths,validation}
core/docker      → core/{launch_error,mock,paths}
```

### Nós Críticos

| Arquivo | Ca (in) | Ce (out) | Instabilidade | Classificação |
|---------|---------|----------|---------------|---------------|
| core/paths | 23 | 0 | 0.00 | God node (maior raio de impacto) |
| core/env_file | 13 | 0 | 0.00 | God node |
| core/docker | 10 | 3 | 0.23 | God node / Hub |
| core/validation | 10 | 0 | 0.00 | God node |
| core/profile | 9 | 2 | 0.18 | God node |
| core/image_family | 8 | 2 | 0.20 | Estável, muito dependido |
| commands/install | 1 | 12 | 0.92 | Instável (orquestrador — esperado) |
| cli | 0 | 20 | 1.00 | Instável (entry point — esperado) |
| lib | 0 | 15 | 1.00 | Instável (entry point — esperado) |
| core/wsl_autostart | 0 | 0 | — | Isolado (467 linhas SEM chamadores) |
| core/wsl_config_writer | 0 | 0 | — | Isolado (301 linhas SEM chamadores) |

### Dependências Circulares

Nenhuma detectada — o grafo core→core é um DAG; core nunca importa commands.

### Observações

- Mexer em `paths.rs`, `env_file.rs`, `validation.rs` ou `docker.rs` tem raio de impacto máximo —
  rodar a suíte inteira e revisar consumidores.
- `wsl_autostart.rs` e `wsl_config_writer.rs` (port Windows) estão declarados em mod.rs mas sem
  chamadores — aguardando fiação do wizard de bootstrap ou código morto a decidir.
- Duplicações que vão doer: `dispatch`/`dispatch_json` no cli.rs (~30 subcomandos em dobro), os 5
  handlers de lifecycle copiados no lib.rs (457-638), `build_firstlogon` em reapply.rs vs oem.rs.
- Usuário `bruno` hardcoded como default de linux_cloud (cloud_init.rs:230, cli.rs:748, com teste
  blindando) + path pessoal em mensagem de erro — parametrizar antes de adoção externa.

## Banco de Dados

Nenhum. Estado = filesystem: `config.env` (fonte de verdade por perfil, editado cirurgicamente
por `env_file::set_key` preservando formato) + `compose.yml` (derivado, sempre regenerado).

## Variáveis de Ambiente

**Lidas:** `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_CACHE_HOME` (+fallbacks), `HOME` (shared_root
`~/Windows`, chave SSH), `USERPROFILE`/`APPDATA`/`LOCALAPPDATA` (Windows), `USER` (default install
interativo), `PATH` (pkexec/sudo).

**Geradas no config.env por perfil** (contrato com dockurr/qemux — ver integrations.md):
IMAGE_FAMILY, VERSION, BOOT, ISO_PATH, RAM_SIZE, CPU_CORES, DISK_SIZE, USERNAME, PASSWORD,
LANGUAGE, REGION, KEYBOARD, TZ, WEB_PORT, DESKTOP_WEB_PORT, RDP_PORT, SSH_PORT, CONTAINER_NAME,
STORAGE_DIR, SHARED_DIR, OEM_DIR, MEM_LIMIT, CPU_LIMIT, BUNDLES, CLOUD_INIT_PROFILE, EXTRA_PORTS,
GPU_BDF.

## Comandos

| Comando | Descrição |
|---------|-----------|
| `cargo test --manifest-path src-tauri/Cargo.toml` | 108 testes inline (sem Docker — mocks por trait) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Gate do CI |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Gate do CI |
| `cargo tauri dev` | GUI em dev |
| `winbox-gui --json list` | CLI machine-readable |
| `winbox-gui install <nome> --yes --pass <senha> --json --progress jsonl` | Install não-interativo (contrato clia/neodrive) |

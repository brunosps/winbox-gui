---
type: techspec
schema_version: "1.0"
status: draft
---

# TechSpec: perfil Office instalável (winbox + WinApps)

## Resumo Executivo

Este TechSpec desenha o perfil Office como uma extensão do winbox existente, sem criar um produto
paralelo: o backend Rust continua sendo o orquestrador, o estado continua em filesystem por perfil,
o frontend continua renderizando estado e progresso, e Docker/FreeRDP/WinApps/ODT entram atrás de
interfaces testáveis. A jornada fica: wizard Office dedicado → pré-flight → aceite BYOL →
perfil Windows 11 Pro dockurr/windows → instalação pós-boot do Office via ODT no guest →
WinApps upstream pinado → `.desktop`/MIME para Excel, Word e PowerPoint → launcher wrapper do
winbox que garante VM up antes de delegar ao WinApps.

A decisão central é manter a máquina de estados do provisionamento no backend Rust, persistida por
perfil e retomável, emitindo os eventos `operation-progress` já usados pela UI. O frontend ganha um
módulo JS novo no padrão `profile-display.js`: puro, testável e injetável. O MVP não adiciona
dependência Rust nova, não usa tokio, não vendoriza WinApps, não adiciona updater Tauri e não
quebra o contrato CLI `--json`/`--progress jsonl` usado por clia/neodrive.

## Branch name

`feat/prd-winbox-office-instalavel`

## Arquitetura do Sistema

### Visão Geral dos Componentes

| Componente | Responsabilidade | Camada |
|------------|------------------|--------|
| `commands/office.rs` | Orquestrar a máquina de estados do perfil Office, retries, adoção, remoção limpa e launch de apps Office. | commands |
| `core/office_state.rs` | Ler/gravar estado persistido por perfil, validar transições e calcular próximo passo idempotente. | core |
| `core/office_odt.rs` | Gerar `configuration.xml`, scripts PowerShell/Batch CRLF, comandos de verificação e parsing de evidências do Office. | core |
| `core/guest_executor.rs` | Abstrair execução de script no Windows via FreeRDP/RDP + share `\\host.lan\Data`, com `MockGuestExecutor`. | core |
| `core/winapps.rs` | Clonar/pinar WinApps, gerar `winapps.conf`, rodar setup/uninstall, verificar launchers e patch controlado dos `.desktop`. | core |
| `core/flatpak.rs` | Detectar FreeRDP flatpak, versão, override `--filesystem=home` e comando `xfreerdp`. | core |
| `core/telemetry.rs` | Opt-in, pseudônimo por instalação, envio HTTPS mínimo via `curl` e persistência local da preferência. | core |
| `cli_office.rs` | Subcomando `winbox office`, com handler único para humano e `--json`. | CLI |
| `src/office-wizard.js` | Estado de apresentação, labels, validações leves, templates e actions do wizard, com dependências injetadas. | frontend |
| `tests/e2e/fixtures/mock-docker.sh` | Mock Docker/Compose para E2E do wizard sem Docker real. | testes |

Fluxo principal:

1. `office_preflight` valida host e detecta setup manual compatível antes de qualquer efeito caro.
2. `office_start_provisioning` cria/atualiza perfil com `PROFILE_KIND=office`, grava estado
   `office-provisioning.json`, prepara `/shared` e `/oem` mínimo e inicia a VM.
3. O backend espera RDP, confirma `remoteapp_prepare` por marker/no-op e só então executa scripts
   PowerShell no guest por FreeRDP + share, fazendo polling de marker files no `SHARED_DIR`.
4. Depois de Office verificado, `winapps-setup --user --setupAllOfficiallySupportedApps` gera os
   launchers; o winbox reescreve apenas os `.desktop` Office para apontarem para `winbox office launch`.
5. A verificação final confirma Office, RDP, WinApps, launchers, MIME e estado persistido.

### Fronteiras

- **Dentro do pacote winbox:** código Rust/JS, templates próprios, scripts gerados do zero,
  estado local, `.desktop` wrapper do winbox e documentação/atribuições.
- **Baixado em runtime:** WinApps upstream, pinado por commit, em `$XDG_DATA_HOME/winbox/winapps`.
- **Nunca embutido:** WinApps, WinApps-Launcher, `RDPApps.reg`, `install.bat` ou artefatos derivados
  AGPL/GPL. O winbox pode produzir scripts próprios equivalentes, escritos do zero.
- **BYOL:** Windows, Office e ativação M365 permanecem responsabilidade do usuário.

## Design de Implementação

### Interfaces Principais

Interfaces são pequenas para manter mocks baratos e a suíte Rust atual rodando sem Docker/VM.

```rust
trait GuestExecutor {
    fn run_script(&self, profile: &str, script: GuestScript) -> Result<GuestRun>;
    fn read_marker(&self, profile: &str, marker: &str) -> Result<Option<GuestMarker>>;
}
```

```rust
trait WinAppsClient {
    fn ensure_pinned_clone(&self, commit: &str) -> Result<WinAppsClone>;
    fn setup_office_apps(&self, profile: &OfficeProfile) -> Result<WinAppsSetup>;
    fn uninstall_user_install(&self) -> Result<()>;
}
```

```rust
trait FlatpakClient {
    fn freerdp_status(&self) -> Result<FreeRdpStatus>;
    fn ensure_home_override(&self) -> Result<FlatpakOverride>;
}
```

```rust
trait TelemetrySink {
    fn load_opt_in(&self) -> Result<TelemetryPrefs>;
    fn send_event(&self, event: TelemetryEvent) -> Result<TelemetrySend>;
}
```

`CliGuestExecutor`, `CliWinAppsClient`, `CliFlatpakClient` e `CurlTelemetrySink` usam
`std::process::Command`, como `cloud_init.rs`. Mocks gravam chamadas em ordem, seguindo
`DockerClient`/`MockDocker`.

### Modelos de Dados

#### Estado persistido do provisionamento

Arquivo por perfil:

`$XDG_CONFIG_HOME/winbox/profiles/<profile>/office-provisioning.json`

Formato lógico:

```text
schemaVersion: "1.0"
profile: string
profileKind: "office"
status: "draft" | "running" | "blocked" | "ready" | "failed" | "adopted" | "removed"
createdAt, updatedAt: RFC3339
byol: { accepted: bool, acceptedAt?: RFC3339, textVersion: "2026-07-08" }
options: { productId, language, officeChannel, windowsVersion, winappsCommit }
phases: { [phaseName]: PhaseState }
lastError?: { code, message, phase, retryable, details? }
managedPaths: { desktopFiles: string[], mimeTypes: string[], winappsConfOwned: bool }
adoption?: { found: AdoptionFinding[], userConfirmedAt?: RFC3339 }
activeSessions?: bool
```

`PhaseState`:

```text
status: "pending" | "running" | "done" | "failed" | "skipped"
attempt: number
startedAt?: RFC3339
finishedAt?: RFC3339
evidence?: { marker?, exitCode?, files?, registry?, launcherIds? }
```

Fases canônicas:

1. `preflight`
2. `byol_acceptance`
3. `profile_config`
4. `windows_prepare`
5. `windows_install`
6. `remoteapp_prepare`
7. `office_stage_odt`
8. `office_install`
9. `winapps_config`
10. `desktop_registration`
11. `file_association`
12. `final_verify`
13. `first_launch`

`ready` é alcançado em `final_verify`; `first_launch` é pós-provisionamento, informativa e não
bloqueia o perfil pronto. Transição é idempotente: fase `done` só reexecuta quando o usuário pede
retry explícito e a fase é marcada como segura para retry. Escrita usa arquivo temporário no mesmo
diretório e rename atômico.

Grafo de dependência:

```text
preflight -> byol_acceptance -> profile_config -> windows_prepare -> windows_install
windows_install -> remoteapp_prepare -> office_stage_odt -> office_install
office_install -> winapps_config -> desktop_registration -> file_association -> final_verify
final_verify -> first_launch
```

Regra de retry: retry da fase N rebaixa todas as fases dependentes para `pending`, preservando
evidências antigas apenas como histórico. Caso concreto: retry de `winapps_config` reexecuta
`winapps-setup`, que reescreve `.desktop`; portanto `desktop_registration`, `file_association` e
`final_verify` devem reexecutar. Invariante de teste: `retry_phase_invalidates_descendants`.

#### Tipos de contrato auxiliares

`PreflightCheck`:

```text
id: string
status: "ok" | "warning" | "blocker"
requirement: string
impact: string
action_hint?: string
details?: object
```

`Resources`:

```text
ramGb: number
cpuCores: number
diskGb: number
storagePath?: string
warningOverride?: bool
```

`AdoptionFinding`:

```text
id: string
kind: "container" | "profile" | "winapps_conf" | "winapps_clone" | "desktop_entry" | "rdp_port" | "office_install" | "odt_asset"
status: "compatible" | "partial" | "unsafe"
evidence: string
managedByDefault: bool
```

`managedScope`:

```text
manageProfileConfig: bool
manageWinAppsConf: bool
manageDesktopEntries: bool
manageFileAssociations: bool
manageDiskLifecycle: bool
preserveExistingWinAppsClone: bool
```

Sinais de adoção: container `winbox-windows` ou `winbox-<profile>`, perfil com `RDP_PORT`
publicado, `~/.config/winapps/winapps.conf`, clone WinApps existente, `.desktop` Office
existentes, Office já instalado no guest e asset ODT já baixado no guest/share.

#### Chaves novas do `config.env`

Sem migration formal. Perfis antigos usam `env_file::get(...).unwrap_or("")` como hoje; ausência
de chave significa perfil não-Office ou default vazio.

| Chave | Default | Regra |
|-------|---------|-------|
| `PROFILE_KIND` | `""` | `office` para perfis criados/adotados pelo wizard Office. |
| `OFFICE_PRODUCT_ID` | `O365ProPlusRetail` | Escolha do wizard: `O365ProPlusRetail`, `O365BusinessRetail`, `O365HomePremRetail`. |
| `OFFICE_LANGUAGE` | `pt-br` ou `en-us` | Imutável após provisionamento, pareado com `LANGUAGE`/`REGION`. |
| `OFFICE_CHANNEL` | `Current` | Explícito, mesmo sendo default do ODT, para determinismo. |
| `OFFICE_APPS` | `excel,word,powerpoint` | MVP fixo; sem Outlook/Access/OneNote/Teams. |
| `OFFICE_ODT_MODE` | `configure_cdn` | ODT setup pré-staged; payload Office baixa no guest via CDN. |
| `WINAPPS_COMMIT` | `5cbf7381f9a12af630e5a289d6dab5f7adc70e5d` | Pin runtime; sem tags upstream. |
| `WINAPPS_FLAVOR` | `manual` | Winbox gerencia VM/container; WinApps só usa RDP. |
| `FREERDP_COMMAND` | `flatpak run --command=xfreerdp com.freerdp.FreeRDP` | Valor gravado no `winapps.conf`. |
| `RDP_FLAGS` | `/cert:ignore +home-drive` | Robusto para VM recriada; decisão registrada em ADR. |

Valores passam por `validate_env_value`; product/language/channel usam allowlist em
`core/validation.rs`.

Guarda FR-4.4: a UI mostra `VERSION` e `OFFICE_LANGUAGE` como imutáveis na revisão de configuração
e nas telas de settings. Para `PROFILE_KIND=office`, `update_profile`, `set` e `reapply_bundles`
não podem alterar `VERSION`, `LANGUAGE` ou `OFFICE_LANGUAGE`; qualquer tentativa retorna erro
estruturado ou aviso de reinstalação destrutiva que exige fluxo futuro separado.

#### ODT `configuration.xml`

Config canônica gerada:

- `OfficeClientEdition="64"`.
- `Channel="Current"` explícito.
- `Product ID` escolhido pelo usuário; default `O365ProPlusRetail`.
- Primeiro `Language` define Shell UI: `pt-br` ou `en-us`.
- `ExcludeApp`: Access, Groove, Lync, OneDrive, OneNote, Outlook, OutlookForWindows, Publisher,
  Teams.
- `Display Level="None" AcceptEULA="TRUE"`.

Product ID errado impede ativação; por isso o wizard expõe Enterprise/Business/HomePrem em vez de
assumir um único SKU [source: https://learn.microsoft.com/en-us/troubleshoot/microsoft-365-apps/office-suite-issues/product-ids-supported-office-deployment-click-to-run, version: ms.date 2025-11-07 (updated_at 2026-06-25), retrieved: 2026-07-08].
O formato e os atributos vêm do ODT [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/office-deployment-tool-configuration-options, version: ms.date 2024-12-19 (updated_at 2026-05-01), retrieved: 2026-07-08].

#### Marker files no share

O executor guest grava marcadores no `SHARED_DIR`, visível no Windows como `\\host.lan\Data`:

```text
winbox-office/
  scripts/
  logs/
  markers/
    office_install.json
    remoteapp_prepare.json
    final_verify.json
```

Marker lógico:

```text
phase: string
status: "running" | "done" | "failed"
exitCode?: number
office?: { productReleaseIds, versionToReport, platform, exePaths }
error?: { code, message, logPath }
updatedAt: RFC3339
```

Scripts gerados pelo winbox usam CRLF. O dockur só converte o `install.bat` de primeiro nível; `.ps1`
auxiliares precisam nascer com line endings corretos [source: https://raw.githubusercontent.com/dockur/windows/v5.14/src/define.sh, version: v5.14, retrieved: 2026-07-08].

### Endpoints de API e Contratos

Contrato first: as shapes abaixo devem ser revisadas antes da implementação. Nomes seguem
`verbo_substantivo`, args aceitam camelCase com alias snake_case, e erros retornam `{code}`.

#### Erros estruturados

Decisão: criar `OfficeError` novo, não estender `LaunchError`. Motivo: o domínio Office tem fases
e códigos próprios; `LaunchError` continua estável para `launch_profile`.

`OfficeError` é enum serde com `#[serde(tag = "code", rename_all = "snake_case")]`, no padrão
`LaunchError`. Exemplo serializado:

```text
{
  "code": "guest_phase_timeout",
  "phase": "office_install",
  "retryable": true,
  "details": { "timeoutSeconds": 3600, "marker": "office_install.json" }
}
```

Códigos iniciais estáveis:

| Código | Uso |
|--------|-----|
| `byol_not_accepted` | Tentativa de criar/provisionar sem aceite. |
| `preflight_kvm_missing` | KVM ausente ou inacessível. |
| `preflight_docker_missing` | Docker/Compose ausente. |
| `preflight_subnet_conflict` | Subnet do perfil conflita com rede local. |
| `flatpak_freerdp_missing` | `com.freerdp.FreeRDP` ausente. |
| `flatpak_home_override_missing` | Override de home ausente ou falhou. |
| `native_freerdp_too_old` | `xfreerdp` nativo existe, mas major <3 ou não executa. |
| `office_product_invalid` | Product ID fora da allowlist. |
| `profile_not_office` | Comando Office em perfil sem `PROFILE_KIND=office`. |
| `profile_state_conflict` | Estado persistido e filesystem divergem. |
| `guest_remoteapp_not_prepared` | RDP responde, mas `/app:` não executa script no-op. |
| `guest_rdp_unreachable` | Porta/login RDP indisponível. |
| `guest_executor_failed` | FreeRDP abriu mas script não completou. |
| `guest_phase_timeout` | Marker esperado não apareceu no timeout da fase. |
| `guest_disk_full` | Guest/share sem espaço para download/instalação Office. |
| `office_windows_failed` | Falha de `windows_prepare`/`windows_install`; preserva `details.launchCode`. |
| `office_odt_stage_failed` | Falha no host ao baixar ODT setup ou escrever no share. |
| `office_odt_failed` | ODT retornou falha ou marker de erro. |
| `office_detection_failed` | Registry/executáveis não confirmam Office. |
| `winapps_clone_failed` | `git clone/fetch/checkout` falhou. |
| `winapps_no_config` | `winapps.conf` ausente ou inválido. |
| `winapps_missing_deps` | Exit code 5 do setup. |
| `winapps_bad_port` | Exit code 13 do setup. |
| `winapps_rdp_failed` | Exit code 14 do setup. |
| `winapps_app_scan_failed` | Exit code 15 do setup. |
| `winapps_pin_mismatch` | Clone não está no commit esperado. |
| `desktop_registration_failed` | `.desktop` ou launcher ausente. |
| `file_association_failed` | MIME não registrado/verificado. |
| `app_not_registered` | App Office solicitado não tem launcher. |
| `app_launch_failed` | Delegação para WinApps falhou. |
| `file_outside_home` | `office_launch_app.files[]` contém path fora de `$HOME`. |
| `office_apps_maybe_open` | Stop/pause/restart de perfil Office exige confirmação de risco. |
| `remove_requires_confirmation` | Remoção destrutiva sem confirmação específica. |

Mapeamento de WinApps: 4→`winapps_no_config`, 5→`winapps_missing_deps`, 13→`winapps_bad_port`,
14→`winapps_rdp_failed`, 15→`winapps_app_scan_failed` [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].

Bridge `LaunchError`→`OfficeError`: fases `windows_prepare` e `windows_install` chamam o launch
core existente. Qualquer `LaunchError { code }` é envelopado como `office_windows_failed`, com
`details.launchCode=<code original>` e demais campos preservados em `details.launchDetails`. Esta
escolha evita duplicar 16 variantes no domínio Office e preserva a causa machine-readable para UI,
CLI e telemetria.

Cobertura de erro por fase:

| Fase | Erros primários |
|------|-----------------|
| `preflight` | `preflight_kvm_missing`, `preflight_docker_missing`, `preflight_subnet_conflict`, `flatpak_freerdp_missing`, `native_freerdp_too_old` |
| `byol_acceptance` | `byol_not_accepted` |
| `profile_config` | `office_product_invalid`, `profile_state_conflict` |
| `windows_prepare` | `office_windows_failed` com `details.launchCode` |
| `windows_install` | `office_windows_failed` com `details.launchCode` |
| `remoteapp_prepare` | `guest_remoteapp_not_prepared`, `guest_phase_timeout` |
| `office_stage_odt` | `office_odt_stage_failed` |
| `office_install` | `office_odt_failed`, `guest_phase_timeout`, `guest_disk_full` |
| `winapps_config` | `winapps_clone_failed`, `winapps_*`, `winapps_pin_mismatch` |
| `desktop_registration` | `desktop_registration_failed` |
| `file_association` | `file_association_failed` |
| `final_verify` | `office_detection_failed`, `app_not_registered`, `file_association_failed` |
| `first_launch` | `app_not_registered`, `app_launch_failed`, `file_outside_home` |

#### Comandos Tauri novos

| Comando | Args | Resposta |
|---------|------|----------|
| `office_preflight` | `{ name?, resources: Resources }` | `{ checks: PreflightCheck[], warnings, blockers, adoptionCandidates: AdoptionFinding[] }` |
| `office_get_state` | `{ name }` | `OfficeProvisioningState` subset: `{status, phases, lastError, activeSessions, adoption, managedPaths}` |
| `office_start_provisioning` | `{ name, productId, language, resources: Resources, byolAccepted, telemetryOptIn?, adoptionId? }` | `OfficeProvisioningState` subset: `{status, phases, lastError}` |
| `office_retry_phase` | `{ name, phase }` | `OfficeProvisioningState` |
| `office_adopt_profile` | `{ name, adoptionId, managedScope, confirm }` | `OfficeProvisioningState` |
| `office_launch_app` | `{ name, appId, files?: string[] }` | `{ appId, delegatedTo, filesAccepted, activeSessions }` |
| `office_remove_profile` | `{ name, deleteDisk, confirmToken? }` | `{ state: "confirmation_required" | "removed", confirmToken?, preservedDisk?, removedPaths? }` |
| `office_telemetry_set_opt_in` | `{ enabled }` | `{ enabled: bool }` |

Todos rodam trabalho bloqueante via `spawn_blocking`; nenhum comando chama Docker, Flatpak, Git,
FreeRDP ou Curl no event loop.

`office_launch_app.files[]` aceita somente paths absolutos dentro de `$HOME`; fora disso retorna
`file_outside_home` com hint explicando que `+home-drive` só expõe o home do usuário ao guest.

Confirm token de remoção: chamada a `office_remove_profile` sem token retorna
`remove_requires_confirmation` com `details.confirmToken`. O token é efêmero, single-use,
armazenado apenas no backend, com escopo `{profile, deleteDisk}` e expiração curta. GUI e CLI usam
o mesmo fluxo; a CLI imprime o token e a frase de confirmação na primeira chamada, e a segunda
chamada passa `--confirm <token>`. Teste de contrato: `office_remove_requires_single_use_token`.

#### Eventos `operation-progress`

Reusar o evento existente:

```text
{ profile, op: "office_provision" | "office_launch" | "office_remove",
  step, status: "running" | "success" | "error", message }
```

Steps novos nomeados:

`office_preflight`, `office_byol`, `office_windows_prepare`, `office_windows_install`,
`office_rdp_wait`, `office_odt_stage`, `office_odt_install`, `office_verify_install`,
`office_winapps_clone`, `office_winapps_setup`, `office_desktop_register`,
`office_file_association`, `office_final_verify`, `office_cold_start`, `office_launch_remoteapp`,
`office_remove_winapps`, `office_remove_desktop`.

Dialeto `--progress jsonl`: o subcomando Office usa as mesmas chaves do emissor existente:
`type`, `profile`, `operation`, `phase`, `status`, `message`, `timestamp`. Falha de fase emite
`status:"error"`.

Exemplo literal:

```json
{"type":"progress","profile":"office","operation":"office_provision","phase":"office_install","status":"error","message":"ODT falhou; veja logs no share","timestamp":"2026-07-08T18:30:00Z"}
```

#### CLI `winbox office`

Novo subcomando:

```text
winbox office status <profile> [--json]
winbox office preflight [profile] [--json]
winbox office provision <profile> [--product-id ...] [--language pt-br|en-us] [--json] [--progress jsonl]
winbox office retry <profile> <phase> [--json] [--progress jsonl]
winbox office adopt <profile> --confirm [--json]
winbox office launch <profile> <excel|word|powerpoint> [--gui-progress] [--json] [--progress jsonl] [-- <files>...]
winbox office remove <profile> [--delete-disk] --confirm <token> [--json]
```

O branch `Cmd::Office` não passa por `dispatch_json -> anyhow -> machine_error_code`. Ele usa
`run_office_command(args, OutputMode) -> Result<Value, OfficeError>` e imprime o envelope com
`error.code` vindo diretamente do `OfficeError`:

```text
{ ok: true, value: ... }
{ ok: false, error: { code, message, hint } }
```

O contrato `clia`/`neodrive` permanece intocado: comandos existentes, envelope e `--progress jsonl`
não mudam. O subcomando evita repetir o padrão `dispatch`/`dispatch_json` duplicado de `cli.rs`,
endereçando o concern H5.

Teste de contrato: `office_json_error_envelope_preserves_office_error_code` valida que um
`OfficeError::GuestPhaseTimeout` sai como `{ok:false,error:{code:"guest_phase_timeout"}}`, sem
`machine_error_code`.

### Fluxo do Executor Guest

Instalação Office é pós-boot. Porém qualquer uso do `GuestExecutor` via FreeRDP `/app:` depende de
RemoteApp já habilitado no Windows. Por isso `remoteapp_prepare` é fase 6, antes de
`office_stage_odt` e `office_install`.

1. `remoteapp_prepare` aplica o canal primário no disco fresco via bootstrap `/oem`. O script é
   clean-room, não copia `RDPApps.reg`, e escreve diretamente estas chaves equivalentes:
   - `HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server\fDenyTSConnections=0`
   - `HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server\WinStations\RDP-Tcp\UserAuthentication=1`
   - `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Terminal Server\TSAppAllowList\fDisabledAllowList=1`
   - `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Terminal Server\TSAppAllowList\fAllowUnlistedRemotePrograms=1`
   - `HKLM\SOFTWARE\Microsoft\Terminal Server Client\Default\AddIns\RDPDR\IgnoreRemoteKeyboardLayout=1`
   O script grava marker `remoteapp_prepare.json` no share quando concluir. Isso satisfaz FR-3.6:
   o artefato é próprio, não derivado do upstream AGPL.
2. Pré-flight do canal executa um script no-op por RemoteApp. Se `/oem` não rodou ou adoção pegou
   VM manual sem RemoteApp pronto, o no-op falha e retorna `guest_remoteapp_not_prepared`.
   Ação guiada: abrir desktop via noVNC e executar `C:\OEM\install.bat`, ou aplicar as chaves por
   sessão RDP full-desktop com passo manual. O MVP não automatiza full-desktop scripting sem
   RemoteApp porque isso depende de interação gráfica frágil.
3. `office_stage_odt` baixa ou localiza o ODT setup e grava `setup.exe` + `configuration.xml` no
   `SHARED_DIR/winbox-office/odt/`. A adoção detecta tanto Office já instalado quanto asset ODT já
   existente no guest/share e evita redownload quando a evidência for confiável.
4. `office_rdp_wait` sobe a VM via fluxo de lifecycle existente e valida porta/login RDP.
5. `GuestExecutor` dispara PowerShell no guest por FreeRDP, passando script no share e registrando
   marker files. O padrão copia a mecânica do WinApps: script trafega pelo share e é executado em
   sessão RDP/RemoteApp [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].
6. Execução no guest é detached: o comando RDP inicia `schtasks` ou `start` para liberar a sessão,
   e o host polla markers no share até timeout. O processo remoto não mantém a conexão RDP aberta
   como fonte de verdade.
7. O script roda `setup.exe /configure configuration.xml`. Sem `SourcePath`, o ODT usa a pasta do
   `setup.exe` e depois o CDN; o guest tem internet por NAT default do dockur [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/overview-office-deployment-tool, version: ms.date 2025-05-08, retrieved: 2026-07-08] [source: https://raw.githubusercontent.com/dockur/windows/v5.14/Dockerfile, version: v5.14 / qemu 7.29, retrieved: 2026-07-08].
8. Verificação exige exit code de sucesso, registry Click-to-Run e existência de
   `EXCEL.EXE`, `WINWORD.EXE`, `POWERPNT.EXE` em `C:\Program Files\Microsoft Office\root\Office16`.
   Microsoft recomenda detecção pós-install além de exit code [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/overview-office-deployment-tool, version: ms.date 2025-05-08, retrieved: 2026-07-08] [source: https://learn.microsoft.com/en-us/archive/blogs/jensha/click2run-installationen-erkennen, version: arquivo/Q&A sem ms.date formal, retrieved: 2026-07-08].

Timeouts por fase:

| Fase | Timeout inicial | Erro em timeout |
|------|-----------------|-----------------|
| `windows_prepare` | 10 min | `office_windows_failed` |
| `windows_install` | 60 min | `office_windows_failed` |
| `office_rdp_wait` | 5 min | `guest_rdp_unreachable` |
| `remoteapp_prepare` | 5 min | `guest_remoteapp_not_prepared` |
| `office_stage_odt` | 10 min | `office_odt_stage_failed` |
| `office_install` | 60 min | `guest_phase_timeout` ou `guest_disk_full` por marker |
| `winapps_config` | 10 min | `guest_phase_timeout` ou mapping WinApps |
| `desktop_registration` | 2 min | `desktop_registration_failed` |
| `final_verify` | 5 min | erro específico da evidência faltante |

Integridade do `OfficeSetup.exe`: URL Microsoft fwlink fixa para o ODT, verificação por
Authenticode no guest quando possível e SHA256 conhecido por release quando o build for pinado.
Se a Microsoft rotacionar o binário e o hash pinado não bater, `office_odt_stage_failed` orienta
revalidar o hash/release antes de prosseguir.

### WinApps e Launchers

Ordem obrigatória:

1. Clonar WinApps em `$XDG_DATA_HOME/winbox/winapps`, checkout
   `5cbf7381f9a12af630e5a289d6dab5f7adc70e5d`.
2. Gerar `~/.config/winapps/winapps.conf` com `0600`.
3. Garantir VM up, RDP ok e Office já instalado.
4. Rodar `winapps-setup --user --setupAllOfficiallySupportedApps`.
5. Verificar `excel-o365`, `word-o365`, `powerpoint-o365`.
6. Reescrever apenas os `.desktop` Office para `Exec=winbox office launch <profile> <app> --gui-progress -- %F`, preservando `Icon`, `Name`, `StartupWMClass`, `Categories` e `MimeType`.

WinApps é automatizável por `setup.sh --user --setupAllOfficiallySupportedApps`, desde que
`winapps.conf` exista e a VM esteja acessível por RDP [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].
Os launchers corretos para M365 C2R x64 são `excel-o365`, `word-o365`, `powerpoint-o365`
[source: https://github.com/winapps-org/winapps/tree/main/apps, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].

`RDP_FLAGS="/cert:ignore +home-drive"` é a decisão do MVP: `/cert:ignore` evita quebra após VM
recriada com certificado novo, e `+home-drive` é necessário para arquivos do Linux chegarem ao
Office via `\\tsclient\home`. FreeRDP 3.x suporta ambos [source: https://github.com/FreeRDP/FreeRDP/blob/master/client/common/cmdline.h, version: FreeRDP 3.28.0, retrieved: 2026-07-08].

`winapps.conf` gerado inclui `RDP_USER` e `RDP_PASS` derivados de `USERNAME`/`PASSWORD` do
`config.env`, além de `RDP_IP`, `RDP_PORT`, `WAFLAVOR`, `FREERDP_COMMAND`, `RDP_FLAGS` e timeouts.
Como o arquivo é bash-sourced pelo WinApps, o perfil Office restringe senha Windows a uma allowlist
segura para single quotes ou grava com quoting shell próprio testado. Trade-off: restringir senha
do perfil Office reduz compatibilidade de caracteres, mas evita injeção em config bash sem trazer
parser novo.

Launcher wrapper:

- `.desktop` chama `winbox office launch <profile> <app> --gui-progress -- %F`.
- Caminho base sem crate novo: o wrapper usa `notify-send` via `std::process::Command` nos marcos
  `office_cold_start`, `office_rdp_wait`, `office_launch_remoteapp` e falha final. Ubuntu/Mint
  trazem libnotify em instalações desktop comuns; ausência de `notify-send` degrada para progresso
  CLI/jsonl sem falhar o launch.
- Com `--gui-progress`, o wrapper spawna o próprio binário em modo GUI dedicado:
  `winbox --window=office-progress <profile> <app>`. `main.rs` já roteia por argv; esse argv passa
  a ser contrato. A janela mínima consome os mesmos eventos `operation-progress` e fecha quando o
  WinApps recebe a delegação.
- Se a GUI principal já estiver aberta, o backend emite o evento na janela existente quando houver
  handle disponível. Sem single-instance plugin no MVP, degradação aceita: duas janelas de
  progresso podem abrir.

Detecção proxy de sessão ativa para FR-7.6:

- `office_get_state.activeSessions` é `true|false` best-effort, calculado por processo `xfreerdp`
  flatpak conectado ao `RDP_PORT` do perfil (`pgrep` + `ss`/procfs).
- Se a detecção for indeterminada e `PROFILE_KIND=office` com VM `running`, a UI/CLI assumem
  `office_apps_maybe_open`.
- `commands/lifecycle.rs` chama hook Office antes de stop/restart/pause. O hook retorna confirm-code
  `office_apps_maybe_open`; sem confirmação, a ação é bloqueada e avisa risco de trabalho não salvo.

Ativação pendente (FR-7.4) não tem detecção programática no MVP: o winbox não lê estado de conta
Microsoft nem tenta inferir licença. A UI mostra orientação estática pós-launch: se o app abrir e
pedir sign-in/ativação, isso é esperado e responsabilidade do usuário. Erros de launch só
classificam VM/RDP/WinApps/app ausente.

### Empacotamento Linux

- `tauri.conf.json` deve trocar `bundle.targets: "all"` por `["deb", "appimage"]`, alinhando build
  local e CI. Tauri suporta `deb` e `appimage` por `cargo tauri build --bundles deb,appimage`
  [source: https://v2.tauri.app/reference/cli/ + https://v2.tauri.app/distribute/debian/ + https://v2.tauri.app/distribute/appimage/, version: Tauri 2.x docs, retrieved: 2026-07-08].
- Release Linux roda em Ubuntu 22.04 para baseline glibc [source: https://v2.tauri.app/distribute/appimage/, version: Tauri 2.x docs, retrieved: 2026-07-08].
- `.deb` declara WebKit/GTK automaticamente; Docker/Flatpak/FreeRDP não entram como hard-depends,
  pois são diagnosticados pelo wizard [source: https://v2.tauri.app/distribute/debian/ + https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/interface/rust.rs, version: tauri-cli dev branch (compatível 2.10.x), retrieved: 2026-07-08].
- AppImage embute WebKitGTK, mas Ubuntu 24.04 exige `libfuse2t64` ou `--appimage-extract-and-run`
  [source: https://docs.appimage.org/user-guide/troubleshooting/fuse.html, version: AppImage docs atual, retrieved: 2026-07-08].
- MVP sem `tauri-plugin-updater`: no Linux o updater só cobre AppImage, não `.deb`; update manual
  por release preserva estado local, suficiente para FR-1.4 [source: https://v2.tauri.app/plugin/updater/, version: tauri-plugin-updater 2.x docs, retrieved: 2026-07-08].
- CI publica `SHA256SUMS` em step próprio, porque Tauri/tauri-action não geram checksums [source: https://github.com/tauri-apps/tauri-action README + https://v2.tauri.app/distribute/, version: tauri-action v1 / Tauri 2.x, retrieved: 2026-07-08].

## Pontos de Integração

### dockurr/windows

- Imagem permanece `dockurr/windows:5.14`, pinada em `core/paths.rs`.
- `VERSION="11"` para Windows 11 Pro; `MANUAL` nunca é setado.
- NAT default do guest baixa ODT/Office do CDN; DHCP/macvlan fica fora.
- `/shared` é canal runtime vivo para ODT/scripts/markers; `/oem` é bootstrap de disco fresco.
- Alterar `VERSION`/`LANGUAGE` após disco instalado continua proibido.

`VERSION=11` significa Windows 11 Pro no v5.14 [source: https://raw.githubusercontent.com/dockur/windows/v5.14/readme.md, version: v5.14, retrieved: 2026-07-08].
`/oem` só entra na ISO na instalação inicial; mudar depois não reexecuta [source: https://raw.githubusercontent.com/dockur/windows/v5.14/src/define.sh, version: v5.14, retrieved: 2026-07-08].

### FreeRDP flatpak

- Detecção nativa primeiro: `xfreerdp /version` ou `xfreerdp3 /version`, gate major >=3, mesmo
  critério do WinApps.
- Se FreeRDP nativo está ausente, major <3 ou falha smoke test, o perfil força
  `FREERDP_COMMAND="flatpak run --command=xfreerdp com.freerdp.FreeRDP"` no `winapps.conf`.
- Detecção flatpak: `flatpak info com.freerdp.FreeRDP` por exit code.
- Versão flatpak: `flatpak list --app --columns=application,version`, parser tolerante a `3.28.`.
- Override: `flatpak override --user --filesystem=home com.freerdp.FreeRDP`, validado por
  `flatpak info --show-permissions`.
- Flatpak ausente não é hard-error imediato: o pré-flight oferece instalação guiada
  `flatpak install flathub com.freerdp.FreeRDP` com consentimento. `flatpak_freerdp_missing` vira
  bloqueante só se o usuário recusar ou a instalação falhar.
- Invocação: `flatpak run --command=xfreerdp com.freerdp.FreeRDP`.

O manifest atual concede rede, mas só `xdg-download` como filesystem; o override de home é
necessário para `+home-drive` [source: https://raw.githubusercontent.com/flathub/com.freerdp.FreeRDP/master/com.freerdp.FreeRDP.json, version: Flathub FreeRDP 3.28.0 / WinApps main, retrieved: 2026-07-08].

### Subnet Docker

FR-2.5 usa checagem best-effort real:

1. Descobrir redes que o compose usará por `docker network inspect` quando a rede já existir.
2. Ler `default-address-pools` do `daemon.json` quando configurado, para prever redes novas.
3. Ler rotas locais do host por `ip route`.
4. Comparar CIDRs; interseção retorna `preflight_subnet_conflict` com `action_hint` concreto, por
   exemplo configurar `default-address-pools` no daemon Docker e recriar a rede do perfil.

Limitação assumida: Docker pode escolher pool diferente em runtime se configuração mudar entre
pré-flight e `compose up`; por isso o check é preventivo, não prova formal.

### WinApps upstream

- Clone runtime em `$XDG_DATA_HOME/winbox/winapps`.
- Pin por commit, porque upstream não tem tags/releases [source: https://github.com/winapps-org/winapps/releases, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].
- `WAFLAVOR=manual`, `RDP_IP=127.0.0.1`, `RDP_PORT=<perfil>`, `PORT_TIMEOUT/RDP_TIMEOUT/APP_SCAN_TIMEOUT/BOOT_TIMEOUT`
  aumentados para primeira execução.
- `--uninstall` não-interativo na remoção limpa [source: https://raw.githubusercontent.com/winapps-org/winapps/main/setup.sh, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08].

### Office Deployment Tool

- O winbox pré-stageia ODT setup + XML; payload Office baixa no guest via `/configure` e CDN.
- Download completo esperado na ordem de GBs; UX deve comunicar rede lenta/download grande [source: https://learn.microsoft.com/en-us/microsoft-365-apps/best-practices/build-dynamic-lean-universal-packages, version: ms.date 2024-05-25, retrieved: 2026-07-08].
- `/download` offline fica como ADR/recomendação futura, não caminho do MVP.

### Telemetria self-hosted

- Endpoint HTTPS mínimo no Coolify do dono: `POST /events`.
- Client usa `curl` via `std::process::Command`.
- Falha de telemetria nunca falha provisionamento; é best-effort comentado, conforme P-006.
- Retenção curta: 30 dias no MVP beta; export simples NDJSON/CSV.

## UI (dw-ui-discipline)

### 1. Autoridade de design

Autoridade: `DESIGN.md`, v2 "Ops Premium Clean". O wizard usa os tokens existentes:
`--bg`, `--surface`, `--surface-subtle`, `--border`, `--text`, `--muted`, `--accent`, `--ok`,
`--warn`, `--danger`, `--radius`, `--control-h`. A superfície segue `ops-bar` e painéis de
trabalho; sem landing page, sem cards dentro de cards, sem gradientes dominantes e raio máximo 8px.

### 2. Job da superfície

Esta superfície ajuda o power user Linux a provisionar e verificar um perfil Office real em VM
Windows para abrir Excel, Word e PowerPoint pelo desktop Linux sem montar Docker, Office, FreeRDP
e WinApps manualmente.

### 3. State matrix

| Estado | Tratamento no wizard |
|--------|----------------------|
| `default` | Etapa atual, resumo do perfil e CTA primária contextual. |
| `hover` | Apenas em controles clicáveis; mudança visual sutil por token existente. |
| `active` | Botões com estado pressionado sem alterar layout. |
| `focus-visible` | Ring forte e distinto de hover, conforme `DESIGN.md`. |
| `disabled` | CTA indisponível com motivo visível e sem handler ativo. |
| `loading` | Progresso por fase; operações >10s usam stepper/progress e mensagem, não spinner genérico. |
| `empty` | Sem perfil Office: CTA "Criar perfil Office" e diagnóstico inicial. |
| `error` | Erro categorizado por `{code}`, fase afetada, evidência e ação/retry. |
| `success` | Perfil pronto, checks finais e CTAs para abrir Excel/Word/PowerPoint. |

Estados de domínio:

| Estado de domínio | Uso |
|-------------------|-----|
| `pending` | Fase ainda não iniciada. |
| `running` | Fase ativa com `aria-live="polite"`. |
| `done` | Fase verificada; mostra evidência curta. |
| `failed` | Fase falhou; mostra retry quando seguro. |
| `skipped` | Adoção detectou etapa já satisfeita, ex. Office existente. |
| `warning-override` | RAM/disco abaixo do recomendado; risco explícito + confirmação. |
| `resume` | Estado persistido interrompido; CTA retoma último passo verificável. |
| `adoption-review` | Setup manual encontrado; lista o que será gerenciado. |
| `remoteapp-not-prepared` | Canal `/app:` falhou; mostra ação guiada via noVNC/manual. |
| `final-verification` | Office/RDP/WinApps/menu/MIME checados antes de "pronto". |
| `cold-start` | Launcher Office iniciou VM parada; progresso visível antes de delegar ao WinApps. |
| `apps-maybe-open-warning` | Stop/pause/restart em perfil Office running exige confirmação de risco. |
| `office-progress-window` | Janela mínima aberta por `--gui-progress`, alimentada por `operation-progress`. |

### 4. Scene sentence

Um power user Linux usa isto em um notebook ou monitor desktop durante uma sessão de setup longa,
em luz ambiente comum, com pouca paciência para tentativa e erro e alta atenção a mensagens de
risco/licença.

### Acessibilidade floor

- Teclado end-to-end: todas as etapas, overrides, retries, adoção e remoção acessíveis por Tab,
  Enter e Escape em modais.
- `aria-live="polite"` no progresso e `role="alert"` em erro bloqueante.
- Contraste WCAG 2.2 AA usando tokens do `DESIGN.md`.
- Controles principais com `--control-h: 44px`; icon-only sempre com `aria-label`.
- Erros de formulário usam `aria-invalid` + `aria-describedby`.
- `prefers-reduced-motion` preservado; sem animações longas em fase de progresso.
- Strings novas em `en-US.js` e `pt-BR.js`, paridade 1:1.

## Abordagem de Testes

### Testes Unitários

Rust:

- `office_state`: transições válidas, retomada, retry seguro, escrita/leitura de schema v1 e
  compatibilidade quando arquivo ausente.
- `office_odt`: product IDs allowlist, XML com `ExcludeApp` correto, idioma primário, CRLF nos
  scripts e rejeição de env value inválido.
- `guest_executor`: montagem de comando FreeRDP sem executar processo real, parsing de marker
  `done/failed`, timeouts e preservação de logs.
- `flatpak`: parser tolerante da versão `3.28.`, detecção por exit code, parsing de permissões.
- `winapps`: geração de `winapps.conf` 0600, mapping de exit codes, patch dos `.desktop` Office
  preservando campos observáveis.
- `telemetry`: redaction, pseudônimo não derivado de máquina, opt-in/opt-out e falha best-effort.
- `OfficeError`: roundtrip serde e códigos estáveis.
- `office_state`: `retry_phase_invalidates_descendants` garante invalidação em cascata.
- CLI JSON: `office_json_error_envelope_preserves_office_error_code`.
- Remoção: `office_remove_requires_single_use_token`.
- Lifecycle: `office_lifecycle_blocks_when_apps_maybe_open`.

JS:

- `office-wizard.js`: reducer de estados, labels por fase, render helpers com escape injetado,
  CTA habilitado/desabilitado, state matrix e form validation leve.
- Locales: paridade `en-US`/`pt-BR` e ausência de strings hardcoded novas no wizard.

Metas: ~80% de cobertura nos módulos `core/office_*`, `flatpak`, `winapps`, `telemetry`; ~70% em
`commands/office.rs`, porque orquestra I/O mockado.

Tooling de cobertura: Rust mede com `cargo-llvm-cov` no job CI de Rust; JS mede com
`node:test --experimental-test-coverage` ou `c8` no job CI de JS. Se a ferramenta não estiver
instalada no primeiro PR, a task de CI deve adicioná-la explicitamente ou registrar waiver no
TechSpec/Tasks.

### Testes de Integração

- Extender `DockerClient`/`MockDocker` para status/compose/logs relevantes do perfil Office, sem
  exigir Docker real.
- Adicionar `MockGuestExecutor`, `MockFlatpakClient`, `MockWinAppsClient`, `MockTelemetrySink`.
- Garantir que os 108 testes Rust atuais continuem sem Docker, VM, Flatpak ou rede.
- Testar `commands/office.rs` com mocks: criação limpa, adoção, interrupção/resume, retry de ODT,
  falha de WinApps, remoção preservando disco, remoção com delete disk confirmada.
- Testar CLI `office` em modo humano e `--json` pelo mesmo handler, validando envelope e
  `--progress jsonl`.

### E2E

O TechSpec inclui consertar o harness E2E antes de confiar no wizard:

1. Criar `tests/e2e/fixtures/mock-docker.sh` e ajustar `DOCKER_HOST`/PATH de teste, alinhado com
   `wdio.conf.mjs`.
2. Atualizar seletores stale (`.profile-card` → superfície atual table/workbench).
3. Seed de perfil Office mockado com `office-provisioning.json`.
4. Adicionar shims por PATH de teste, controláveis por fixture/env:
   - `flatpak`
   - `git`
   - `curl`
   - `xfreerdp`
   - `winapps-setup`
   - `notify-send`
   Alternativa aceitável: modo de teste do binário que injeta `MockGuestExecutor`,
   `MockWinAppsClient`, `MockFlatpakClient` e `MockTelemetrySink`.
5. Fluxo E2E mínimo:
   - abrir app, iniciar wizard Office;
   - ver pré-flight mockado com warning RAM/disco e override;
   - simular `guest_remoteapp_not_prepared` e ver ação guiada;
   - aceitar BYOL;
   - iniciar provisionamento mockado com eventos `operation-progress`;
   - simular falha `office_odt_failed`, ver retry;
   - retry concluir e mostrar final verification;
   - clicar Excel com VM parada, ver `notify-send` shim, janela `--window=office-progress` e
     delegação mockada;
   - tentar pause/restart com `activeSessions` indeterminado e ver confirmação
     `office_apps_maybe_open`;
   - simular upgrade vN→vN+1: provisiona mock em vN, instala/builda vN+1, valida estado,
     launchers, menu e associações preservados;
   - validar a11y básica: foco, `aria-live`, labels, sem overflow em 1024x768.

## Sequenciamento de Desenvolvimento

### Ordem de Construção

1. **Estado/modelo Office:** `office-provisioning.json`, allowlists e transições. Base para tudo e
   sem I/O externo.
2. **RemoteApp prepare + preflight:** bootstrap `/oem` clean-room, no-op `/app:`, fallback de
   adoção, FreeRDP nativo/flatpak e subnet Docker.
3. **Executor guest + ODT staging:** scripts CRLF, marker files, geração XML, integridade do ODT e
   verificação Office. Destrava o risco técnico central.
4. **WinApps:** clone pinado, `winapps.conf`, setup e
   mapping de errors.
5. **Comandos Tauri/CLI:** contracts, `OfficeError`, `operation-progress`, handler único CLI.
6. **Wizard UI:** módulo `office-wizard.js`, templates no `main.js` só como integração fina,
   i18n e CSS usando tokens existentes.
7. **Launcher/desktop/MIME:** patch dos `.desktop`, wrapper `winbox office launch`, cold-start e
   remoção limpa.
8. **Telemetria opt-in:** prefs, pseudônimo, envio `curl`, endpoint e export.
9. **Empacotamento/CI:** targets `deb/appimage`, Ubuntu 22.04, `SHA256SUMS`, docs AppImage FUSE.
10. **E2E:** consertar harness, shims PATH, fluxos wizard, lifecycle, upgrade e launch.

### Dependências Técnicas

- Host Linux x86_64 com KVM, Docker/Compose, Flatpak e rede para Microsoft CDN.
- `git`, `curl`, `flatpak`, `docker`, `xfreerdp` via Flatpak; nenhum crate Rust novo.
- Endpoint HTTPS da telemetria antes do beta público; o produto funciona sem ele.
- ADRs aprovados antes de implementação dos pontos legais/certificados/lista ODT.

## Monitoramento e Observabilidade

### Progresso local

- `operation-progress` é o trace primário da GUI.
- CLI `--progress jsonl` emite fases Office com timestamp RFC3339.
- `office-provisioning.json` guarda última fase, erro e evidências.
- Logs do guest ficam em `SHARED_DIR/winbox-office/logs/`, com paths relativos no marker.

### Telemetria

Opt-in explícito antes de qualquer evento. Preferência:

`$XDG_CONFIG_HOME/winbox/telemetry.json`

Shape:

```text
schemaVersion: "1.0"
enabled: bool
pseudonym: random 128-bit hex generated from OS randomness, not machine-id/hostname/user/path
createdAt, updatedAt: RFC3339
```

Evento JSON:

```text
{
  "schemaVersion": "1.0",
  "event": "wizard_started|phase_started|phase_completed|phase_failed|wizard_completed|wizard_abandoned",
  "release": "0.1.0",
  "pseudonym": "...",
  "profileKind": "office",
  "phase": "office_odt_install",
  "durationMs": 1234,
  "errorCode": "office_odt_failed",
  "host": { "osFamily": "linux", "distroFamily": "ubuntu_or_mint", "arch": "x86_64" },
  "options": { "language": "pt-br", "productFamily": "enterprise|business|home" },
  "timestamp": "RFC3339"
}
```

Não coletar: credenciais, chaves, nomes de arquivos, conteúdo de documentos, paths absolutos,
hostname, username, machine-id, MAC, IP, serial, dados dentro da VM. `pseudonym` pode ser apagado
no opt-out; novo opt-in gera outro pseudônimo.

Endpoint:

- `POST https://<coolify-domain>/events`
- Beta usa URL pública sem auth. Risco aceito: poisoning/spam de eventos pode poluir métricas; o
  servidor mitiga por rate limit, validação de schema e retenção curta, não por segredo no client.
- Client chama `curl --max-time 5` em modo fire-and-forget; falha de envio não bloqueia fluxo nem
  aparece como erro de provisionamento.
- Abandono é derivado server-side pelo último evento por pseudônimo e release, não por evento
  explícito confiável do client.
- Servidor grava JSON append-only e aplica retenção de 30 dias.

## Considerações Técnicas

### Decisões Principais

1. **Máquina de estados no backend Rust, persistida por perfil.**  
   Rationale: retomada e idempotência são comportamento de domínio, não estado visual; o frontend
   só renderiza. Respects: P-001, P-002, P-004, P-006.

2. **Estado novo em `office-provisioning.json`; `config.env` recebe só chaves estáveis do perfil.**  
   Rationale: evita versionar `config.env` como schema complexo e mantém compatibilidade de perfis
   existentes via defaults vazios. Respects: P-001.

3. **RemoteApp prepare antes de qualquer `GuestExecutor`.**  
   Rationale: execução `/app:` depende das chaves RemoteApp; bootstrap `/oem` clean-room é a fonte
   primária em disco fresco e adoção valida com script no-op. Respects: P-001, P-006.  
   Source: [source: .dw/spec/prd-winbox-office-instalavel/research.md seção odt, version: retrieved 2026-07-08, retrieved: 2026-07-08].

4. **Instalação Office pós-boot, via ODT staged em `/shared`, não `/oem` como fonte de verdade.**  
   Rationale: `/oem` só roda em disco fresco e `install.bat` é best-effort; o orquestrador precisa
   verificar markers e permitir retry. Respects: P-001, P-006.  
   Sources: [source: https://raw.githubusercontent.com/dockur/windows/v5.14/src/define.sh, version: v5.14, retrieved: 2026-07-08] [source: https://github.com/dockur/windows/issues/677, version: issues 2024-2025, retrieved: 2026-07-08].

5. **ODT `/configure` direto do CDN no guest para o MVP; `/download` offline vira ADR/backlog.**  
   Rationale: pré-stage do instalador + XML é leve; payload Office de ~3GB no share aumentaria
   complexidade de cache/retry antes de haver demanda. Respects: P-004, P-005.  
   Sources: [source: https://learn.microsoft.com/en-us/microsoft-365-apps/deploy/overview-office-deployment-tool, version: ms.date 2025-05-08, retrieved: 2026-07-08] [source: https://learn.microsoft.com/en-us/microsoft-365-apps/best-practices/build-dynamic-lean-universal-packages, version: ms.date 2024-05-25, retrieved: 2026-07-08].

6. **Product ID default `O365ProPlusRetail`, mas escolha exposta no wizard.**  
   Rationale: Enterprise é a recomendação de compliance do PRD, mas ID errado impede ativação para
   Business/HomePrem. Respects: P-001, P-008.  
   Source: [source: https://learn.microsoft.com/en-us/troubleshoot/microsoft-365-apps/office-suite-issues/product-ids-supported-office-deployment-click-to-run, version: ms.date 2025-11-07 (updated_at 2026-06-25), retrieved: 2026-07-08].

7. **WinApps upstream em runtime, pinado por commit, nunca vendorizado.**  
   Rationale: fronteira AGPL limpa e upgrade deliberado; upstream não publica tags. Respects:
   P-005.  
   Sources: [source: https://github.com/winapps-org/winapps/releases, version: main@5cbf738 (2026-07-07), retrieved: 2026-07-08] [source: .dw/spec/prd-winbox-office-instalavel/research.md seção licensing, version: retrieved 2026-07-08, retrieved: 2026-07-08].

8. **`WAFLAVOR=manual` e `RDP_FLAGS="/cert:ignore +home-drive"`.**  
   Rationale: o winbox é dono do lifecycle da VM; `/cert:ignore` reduz falha após recriar VM, e
   `+home-drive` é requisito funcional para abrir arquivos do Linux. Respects: P-001, P-007.  
   Sources: [source: https://github.com/winapps-org/winapps/blob/main/README.md, version: main 2026-07, retrieved: 2026-07-08] [source: https://github.com/FreeRDP/FreeRDP/blob/master/client/common/cmdline.h, version: FreeRDP 3.28.0, retrieved: 2026-07-08].

9. **Flatpak FreeRDP como caminho suportado do perfil Office.**  
   Rationale: evita FreeRDP nativo quebrado/antigo; detecção e override são programáveis. Respects:
   P-001, P-006.  
   Source: [source: https://raw.githubusercontent.com/flathub/com.freerdp.FreeRDP/master/com.freerdp.FreeRDP.json, version: Flathub FreeRDP 3.28.0 / WinApps main, retrieved: 2026-07-08].

10. **`.desktop` Office executa wrapper `winbox office launch`, não o WinApps diretamente.**  
   Rationale: permite cold-start com progresso, erros estruturados e preserva WinApps como executor
   final. Respects: P-002, P-008.

11. **`OfficeError` separado de `LaunchError`.**  
    Rationale: mantém contrato de `launch_profile` estável e dá códigos específicos ao domínio
    Office. Respects: P-002, P-004.

12. **Wizard em módulo JS novo e puro.**  
    Rationale: evita inflar `main.js`, segue padrão `profile-display.js` e destrava node:test.
    Respects: P-003, P-004, P-008.

13. **Telemetria por `curl` via `Command`, opt-in e best-effort.**  
    Rationale: atende funil beta sem dependência Rust nova nem SDK; falha não bloqueia usuário.
    Respects: P-006, P-007.

14. **Empacotamento `deb` + `appimage`, sem updater Tauri no MVP.**  
    Rationale: FR-1.4 exige preservar estado em upgrade, não auto-update; updater só cobre
    AppImage e criaria assimetria com `.deb`. Respects: no applicable principle: empacotamento e
    update manual não são cobertos diretamente pela constitution atual.  
    Source: [source: https://v2.tauri.app/plugin/updater/, version: tauri-plugin-updater 2.x docs, retrieved: 2026-07-08].

### Minimalismo (ladder full)

| Item novo | Degrau | Justificativa |
|-----------|--------|---------------|
| `commands/office.rs` | 2/7 | Reusa camada commands; módulo próprio evita inflar `lib.rs` e concentra state machine. |
| `core/office_state.rs` | 7 | Estado novo exigido por FR-6.1; JSON com `serde_json` já instalado. |
| `core/office_odt.rs` | 7 | XML/scripts/verificação Office são domínio novo e testável; sem dep nova. |
| `core/guest_executor.rs` | 7 | Necessário para mockar RDP/guest e manter testes sem VM. |
| `core/winapps.rs` | 7 | Encapsula clone/setup/uninstall/pin e exit codes; evita shell solto em commands. |
| `core/flatpak.rs` | 7 | Detecção/override tem parsing próprio e mocks; não cabe em `health.rs` sem misturar domínio. |
| `core/telemetry.rs` | 7 | Opt-in/eventos precisam storage e redaction; usa `curl`/stdlib. |
| `cli_office.rs` | 2/7 | Reusa CLI, mas separa handler único para não repetir `dispatch_json`. |
| `src/office-wizard.js` | 2/7 | Reusa padrão `profile-display.js`; evita novo framework e god file. |
| `winbox --window=office-progress` | 4/6 | Reusa roteamento argv/Tauri existente; janela mínima evita plugin single-instance no MVP. |
| `notify-send` via `Command` | 3 | Reusa ferramenta desktop do host; sem crate/libnotify nova. |
| `tests/e2e/fixtures/mock-docker.sh` | 7 | Harness E2E referencia fixture ausente; necessário para validar wizard sem Docker. |
| Shims E2E `flatpak/git/curl/xfreerdp/winapps-setup` | 7 | Necessários para E2E determinístico sem rede, VM, Flatpak real ou WinApps real. |
| Dependência Rust nova | 1 | Não adicionar. `serde`, `serde_json`, `anyhow`, `chrono` e `Command` bastam. |
| Dependência JS nova | 1 | Não adicionar. Vanilla JS + node:test bastam. |
| `tauri-plugin-updater` | 1 | Fora do MVP; update manual preserva estado e evita assimetria AppImage/.deb. |

### Riscos Conhecidos

| Risco | Impacto | Mitigação |
|-------|---------|-----------|
| `/oem` não rodou e RemoteApp não está preparado | `GuestExecutor` via `/app:` falha antes do Office | `remoteapp_prepare` antes do executor, no-op preflight e erro `guest_remoteapp_not_prepared` com ação via noVNC/manual. |
| ODT falha por rede/CDN lento | Provisionamento parcial | Progress por fase, retry seguro, logs no share e marker de falha. |
| `setup.exe /configure` baixa ~3GB com disco justo | Falha tardia ou guest sem espaço | Pré-flight de disco, marker `guest_disk_full`, warning override e ação para aumentar disco/recriar. |
| Reaplicar XML ODT em instalação existente faz `ExcludeApp` virar merge se idiomas divergem | Apps excluídos podem permanecer ou remoção não ser determinística | Não reaplicar XML parcial em Office existente; adoção verifica e, se divergente, marca manual/reinstall. |
| Registry/paths Office variam | Falso negativo de verificação | Combinar exit code, registry Click-to-Run e três executáveis; erro acionável. |
| WinApps muda setup.sh/main | Setup quebra | Pin commit, hash check pós-setup e smoke test; bump deliberado. |
| `git pull` do setup em detached HEAD é não-contratual | Pin pode ser ignorado no futuro | Verificar commit depois do setup; se divergir, `winapps_pin_mismatch`. |
| `flatpak override --filesystem=home` amplia sandbox | Usuário concede home ao FreeRDP | Explicar no pré-flight; necessário para `+home-drive`; escopo local single-user. |
| `/cert:ignore` aceita cert RDP sem TOFU | Menos proteção MITM | Porta RDP presa em `127.0.0.1`; decisão registrada em ADR; opção futura para tofu. |
| Detecção de apps Office abertos é proxy | Pode pausar VM com documento aberto se proxy falhar | Se indeterminado e perfil Office running, sempre exigir confirmação `office_apps_maybe_open`. |
| `--gui-progress` sem single-instance plugin | Duas janelas de progresso podem abrir | Degradação aceita no MVP; eventos vão para janela existente quando houver handle. |
| Checagem de subnet é best-effort | Docker pode escolher rede diferente depois | Comparar inspect/pools/rotas e emitir action_hint; erro real ainda mapeado se compose falhar. |
| E2E quebrado | Regressão de UI sem rede | Consertar harness antes de finalizar feature. |
| Telemetria endpoint indisponível | Perda de métricas beta | Best-effort; buffer local curto opcional, sem bloquear usuário. |

### Conformidade com Padrões

| Regra | Como o design respeita |
|-------|------------------------|
| `.dw/rules/backend-rust.md` | Mantém core←commands←{cli,lib}; sem tokio; I/O em `spawn_blocking`; validação central; errors `{code}`; traits para mocks. |
| `.dw/rules/frontend-js.md` | Módulo JS puro, escape obrigatório, data-attributes, i18n dot-namespace e paridade local. |
| `.dw/rules/infra-ci.md` | CI continua fmt/clippy/test/js; release Linux adiciona targets Tauri e `SHA256SUMS`; E2E fica explicitamente consertado. |
| `.dw/rules/integrations.md` | Reusa IPC Tauri, `operation-progress`, config.env, pins dockurr/qemux e envelope CLI. |
| `.dw/rules/concerns.md` F1 | dockurr fica pinado; `/oem` não é fonte de verdade; markers e verificação reduzem string mágica. |
| `.dw/rules/concerns.md` F2 | qemux fica inalterado; perfil Office não muda lista de distros nem resolver Linux. |
| `.dw/rules/concerns.md` F3 | Erros novos usam codes estáveis, não substring de stderr. |
| `.dw/rules/concerns.md` F4 | cloud-images não entram no fluxo Office; sem novo acoplamento a URLs Ubuntu. |
| `.dw/rules/concerns.md` F5 | FreeRDP/WinApps real do dono vira produto: RDP_FLAGS, bundle Office e MIME entram no design. |
| `.dw/rules/concerns.md` F6 | Viewer noVNC permanece no browser padrão; o wizard não tenta renderizar canvas no WebKitGTK. |
| `.dw/rules/concerns.md` F7 | Release passa a publicar SHA256SUMS para canal de distribuição. |
| `.dw/rules/concerns.md` F8 | Superfície nova evita contrato textual `"Próxima ação:"`; usa `OfficeError`. |
| `DESIGN.md` | Usa tokens, ops-bar, timeline, 44px, contraste, foco, sem card-in-card. |
| `.dw/constitution.md` P-001..P-008 | Decisões listam `Respects`; nenhum desvio high/critical existe porque princípios atuais são `info`, mas o design segue todos. |

### Arquivos Relevantes

Arquivos a criar/alterar na implementação futura:

- `src-tauri/src/commands/office.rs`
- `src-tauri/src/commands/lifecycle.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/core/office_odt.rs`
- `src-tauri/src/core/guest_executor.rs`
- `src-tauri/src/core/winapps.rs`
- `src-tauri/src/core/flatpak.rs`
- `src-tauri/src/core/telemetry.rs`
- `src-tauri/src/cli_office.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/cli.rs`
- `src-tauri/src/core/validation.rs`
- `src-tauri/src/core/compose.rs`
- `src-tauri/tauri.conf.json`
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/main.js`
- `src/styles.css`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`
- `THIRD-PARTY.md`
- `.github/workflows/ci.yml`
- `.github/workflows/*release*`
- `tests/e2e/fixtures/mock-docker.sh`
- `tests/e2e/*`

## Cobertura FR → Componente/Decisão

| FR | Cobertura |
|----|-----------|
| FR-1.1 | Empacotamento `deb/appimage`, Ubuntu 22.04 build, SHA256SUMS. |
| FR-1.2 | Wizard Office dedicado via `src/office-wizard.js` e comandos `office_*`. |
| FR-1.3 | CLI existente preservado; novo `office` usa handler único sem alterar envelope. |
| FR-1.4 | Estado em `$XDG_CONFIG_HOME`; updater omitido; upgrade manual preserva perfis e registros. |
| FR-2.1 | `office_preflight` valida KVM, Docker, rede, FreeRDP/flatpak, CPU/RAM/disco. |
| FR-2.2 | `OfficeError` codes + checks com `action_hint`. |
| FR-2.3 | `warning-override` para RAM/disco abaixo do recomendado. |
| FR-2.4 | `core/flatpak.rs` detecta FreeRDP inadequado e aplica fallback flatpak. |
| FR-2.5 | Pré-flight de subnet antes de criar VM. |
| FR-3.1 | Tela BYOL com texto versionado antes de criar/baixar/provisionar. |
| FR-3.2 | `byol.accepted` obrigatório no estado; erro `byol_not_accepted`. |
| FR-3.3 | Copy do wizard/docs declara single-user/single-machine. |
| FR-3.4 | Wizard informa OEM/M365 plans sem enforcement. |
| FR-3.5 | WinApps runtime clone pinado; nunca embutido. |
| FR-3.6 | Artefatos AGPL não entram nos assets; scripts próprios ou upstream runtime. |
| FR-3.7 | `THIRD-PARTY.md` + superfície de atribuições/licenças no wizard/docs. |
| FR-4.1 | Wizard cobre diagnóstico, BYOL, recursos, provisionamento, verificação e launch. |
| FR-4.2 | M365 Apps Current; Product ID exposto e documentado. |
| FR-4.3 | `VERSION=11` Windows 11 Pro; Home fora da allowlist. |
| FR-4.4 | `VERSION`/`OFFICE_LANGUAGE` imutáveis após provisionamento. |
| FR-4.5 | `operation-progress` com fases nomeadas. |
| FR-4.6 | UI comunica até ~45 min e downloads grandes. |
| FR-4.7 | `pt-br`/`en-us` no wizard; primeiro idioma define Shell UI. |
| FR-5.1 | ODT instala Excel/Word/PowerPoint e exclui demais apps. |
| FR-5.2 | `remoteapp_prepare` e `final_verify` validam RDP/RemoteApp antes do menu. |
| FR-5.3 | WinApps gera ícones/launchers em runtime; winbox não embute marcas Microsoft. |
| FR-5.4 | `file_association` registra `.xls/.xlsx/.doc/.docx/.ppt/.pptx`. |
| FR-5.5 | `final_verify` exige Office, RDP, WinApps, desktop e MIME. |
| FR-5.6 | Estado por fase + retry seguro + diagnóstico parcial. |
| FR-6.1 | `office-provisioning.json` retomável. |
| FR-6.2 | Fases idempotentes e verificação antes de executar. |
| FR-6.3 | `office_preflight` retorna `adoptionCandidates`. |
| FR-6.4 | `office_adopt_profile` com tela `adoption-review`. |
| FR-6.5 | `managedPaths`, confirmação específica e delete disk separado. |
| FR-7.1 | Perfil Office continua perfil winbox com lifecycle existente. |
| FR-7.2 | Primeira execução explica RemoteApp e sign-in M365. |
| FR-7.3 | Nenhum fluxo automatiza ativação; só orientação. |
| FR-7.4 | `office_launch_app` diferencia VM/RDP/WinApps/ativação/app ausente. |
| FR-7.5 | `office_remove_profile` usa WinApps uninstall, remove MIME/desktop e confirma disco. |
| FR-7.6 | Lifecycle de perfil Office exige aviso se apps Office abertos ou estado desconhecido. |
| FR-7.7 | `.desktop` chama `winbox office launch --gui-progress`, cold-start com progresso. |
| FR-8.1 | `office_telemetry_set_opt_in` e checkbox explícito. |
| FR-8.2 | Shape sem IDs persistentes de máquina/user; pseudônimo aleatório por instalação. |
| FR-8.3 | Eventos por fase e duração p50/p90 calculáveis. |
| FR-8.4 | Beta público usa release GitHub + telemetria + feedback qualitativo. |

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — fronteira AGPL/GPL, runtime clone, proibição de
  vendorizar WinApps/Launcher/`RDPApps.reg`/`install.bat`.
- `adrs/adr-office-odt-download-strategy.md` — MVP com ODT setup pré-staged e `/configure` direto
  do CDN no guest; analisar alternativa `/download` para layout offline no share.
- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — aceitar `/cert:ignore` no loopback por robustez de VM
  recriada, trade-off e opção futura TOFU.

## Auto-gate do TechSpec

- Template completo: sim.
- Seções obrigatórias extras: sim.
- 45 FRs mapeados: sim, FR-1.1 a FR-8.4.
- UI grounding: 4 perguntas, state matrix, scene e acessibilidade: sim.
- Source grounding: decisões vendor/framework com citações inline: sim.
- Reuso da arquitetura existente: sim, core←commands←{cli,lib}, `spawn_blocking`, traits,
  `operation-progress`, `env_file`, bundles e i18n.
- Erros com codes estáveis: sim, `OfficeError`.
- Testes com invariantes, mocks e E2E explícito: sim.
- PT-BR, sem segredos, sem paths pessoais: sim.
- Ordem de fases executável: sim, `remoteapp_prepare` é fase 6, antes de qualquer
  `GuestExecutor`, com bootstrap `/oem`, no-op e fallback de adoção.
- FR-2.4: sim, FreeRDP nativo major >=3, fallback flatpak e instalação guiada com consentimento.
- FR-2.5: sim, Docker networks/default-address-pools comparados com `ip route`, best-effort.
- FR-7.6: sim, proxy `xfreerdp`/RDP_PORT, `activeSessions` e confirm-code
  `office_apps_maybe_open`.
- FR-7.7: sim, `notify-send`, argv `--window=office-progress`, eventos existentes e degradação
  sem single-instance plugin.
- Taxonomia de erro: sim, tabela fase→erro cobre 13 fases e bridge `LaunchError`.
- Invalidação em cascata: sim, grafo de dependência e teste `retry_phase_invalidates_descendants`.
- E2E executável: sim, fixture `tests/e2e/fixtures/mock-docker.sh`, shims PATH e upgrade mockado.

Auto-nota: 9.3/10. O restante é risco operacional aceito, não lacuna contratual: a etapa manual de
fallback para `guest_remoteapp_not_prepared` em adoção depende de noVNC/full-desktop quando o
Windows existente não recebeu o bootstrap `/oem`.

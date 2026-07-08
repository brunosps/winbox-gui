---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 16.0: Implementar launcher Office, progresso GUI e validação de arquivos

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar o wrapper `winbox office launch` usado pelos `.desktop`, com auto-start da VM, progresso via `notify-send`, janela mínima `--window=office-progress` e validação dos arquivos recebidos.

**Requisitos Funcionais cobertos**: FR-7.4, FR-7.7

**Depende de**: 10.0, 11.0, 12.0, 13.0.

**Constitution**: respects P-001, P-002, P-004, P-006, P-007.

<requirements>
- `.desktop` chama o binário winbox e delega ao launcher WinApps.
- `winbox office launch` garante VM up com progresso antes de abrir o app.
- Caminho base usa `notify-send` via `Command`; sem crate nova.
- `--gui-progress` spawna o próprio binário como `winbox --window=office-progress <perfil> <app>`.
- Decisão de ordenação: a superfície frontend da janela `office-progress` fica na task 21.0, após o shell JS da task 18.0; esta task completa o spawn/contrato backend.
- Os comandos Tauri de launch podem nascer como stubs contract-first na task 10.0 e serem completados aqui.
- Se a GUI principal estiver aberta, evento vai para a janela existente quando possível; duas janelas são degradação aceita no MVP.
- Validar `files: string[]` absolutos e retornar `file_outside_home` para path fora de `$HOME`.
</requirements>

## Subtarefas

### Implementação
- [ ] 16.1 Implementar caminho `office launch` compartilhado por CLI e `.desktop`.
- [ ] 16.2 Emitir step `office_cold_start` nos marcos de VM up/RDP ready.
- [ ] 16.3 Emitir step `office_launch_remoteapp` ao delegar para o launcher WinApps.
- [ ] 16.4 Implementar notificações `notify-send` nos marcos de cold-start.
- [ ] 16.5 Adicionar roteamento argv `--window=office-progress` em `main.rs`.
- [ ] 16.6 Validar arquivos absolutos dentro de `$HOME` antes de passar ao WinApps.

### Testes Automatizados
- [ ] 16.7 Teste `office_launch_file_outside_home_returns_hint` — invariante: `+home-drive` só expõe `$HOME`; camada: unit Rust commands/office.
- [ ] 16.8 Teste `gui_progress_argv_is_spawned_without_new_dependency` — invariante: `--gui-progress` usa o próprio binário; camada: integração CLI Rust com command mock.
- [ ] 16.9 Teste `launch_emits_cold_start_and_remoteapp_steps` — invariante: launch emite `office_cold_start` e `office_launch_remoteapp`; camada: integração commands.
- [ ] 16.10 Teste `launch_failure_reports_vm_rdp_winapps_or_app_context` — invariante: erro de launch preserva contexto acionável; camada: integração commands com mocks.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `validate_launch_files` | arquivo absoluto em home, relativo, fora de home, path inexistente |
| `launch_office_app` | VM já up, cold-start, WinApps app ausente, RDP falha |
| `spawn_progress_window` | flag ausente, flag presente, GUI já aberta best-effort |

### Mocks Necessários
- Trait de comando host para `notify-send` e spawn do próprio binário.
- `MockWinAppsClient`.
- `MockDocker`.

## Detalhes de Implementação

Ver techspec §§Launcher, FR-7.7, Contratos de API, FR-7.4 e Validação de Arquivos.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- office_launch` passa.
- Nenhuma dependência Rust nova é adicionada.

## Arquivos Relevantes
- `src-tauri/src/commands/office.rs`
- `src-tauri/src/cli_office.rs`
- `src-tauri/src/core/winapps.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/core/validation.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/commands/office.rs src-tauri/src/cli_office.rs src-tauri/src/core src-tauri/src/main.rs
git commit -m "feat(office): launch office apps with progress"
```

## Related ADRs

- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — launcher usa a sessão RDP definida no ADR.

---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 9.0: Executar instalação Office no guest e verificar readiness

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Estender o executor guest criado na task 7.0 para disparar scripts PowerShell de forma detached, pollar markers no share e verificar instalação de Excel, Word e PowerPoint.

**Requisitos Funcionais cobertos**: FR-5.1, FR-5.5

**Depende de**: 7.0, 8.0.

**Constitution**: respects P-002, P-004, P-006, P-007.

<requirements>
- Estender o trait `GuestExecutor` e `MockGuestExecutor` criados na task 7.0.
- Usar sessão FreeRDP/RemoteApp apenas depois de `remoteapp_prepare`.
- Disparar execução detached no guest via `schtasks`/`start`, liberando a sessão.
- Polling de markers com timeouts por fase, incluindo `office_install >= 60min`.
- Verificar exit code, registry ClickToRun e existência dos EXEs Office.
- Diferenciar `guest_executor_failed`, `guest_phase_timeout`, `guest_disk_full` e `office_odt_failed`.
</requirements>

## Subtarefas

### Implementação
- [ ] 9.1 Estender `GuestExecutor`/`MockGuestExecutor` para execução detached, polling e verificação Office.
- [ ] 9.2 Implementar renderização de scripts PowerShell CRLF e execução detached.
- [ ] 9.3 Implementar polling de markers e tabela de timeouts por fase.
- [ ] 9.4 Implementar verificação final de instalação Office no guest.

### Testes Automatizados
- [ ] 9.5 Teste `guest_execution_is_detached_and_marker_driven` — invariante: host não depende de sessão RDP aberta durante instalação; camada: unit Rust core/guest_executor.
- [ ] 9.6 Teste `office_install_timeout_returns_guest_phase_timeout` — invariante: marker ausente vira timeout específico; camada: unit Rust core/guest_executor.
- [ ] 9.7 Teste `office_ready_requires_clicktorun_and_three_exes` — invariante: readiness exige registry e EXCEL/WINWORD/POWERPNT; camada: unit Rust core/guest_executor.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `render_guest_script` | CRLF, paths escapados, senha não renderizada em logs |
| `poll_marker` | done, failed, timeout, disco cheio |
| `verify_office_install` | completo, registry ausente, EXE ausente, Platform != x64 |

### Mocks Necessários
- `MockGuestExecutor`.
- Diretório de share temporário com markers.

## Detalhes de Implementação

Ver techspec §§Fluxo do Executor Guest, Timeouts, ODT, Contratos de Erro.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- guest_executor office_install` passa.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte de executor guest passando.

## Arquivos Relevantes
- `src-tauri/src/core/guest_executor.rs`
- `src-tauri/src/core/office_odt.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): execute office install in guest"
```

## Related ADRs

- `adrs/adr-office-odt-download-strategy.md` — rege execução de `setup.exe /configure`.
- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — rege sessão FreeRDP usada pelo executor.

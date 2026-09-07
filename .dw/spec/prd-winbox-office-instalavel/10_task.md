---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 10.0: Expor OfficeError, comandos Tauri e progresso estruturado

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Criar a superfície Tauri Office contract-first, com `OfficeError` serializado por `code`, bridge de `LaunchError`, comandos novos e eventos `operation-progress` com fases Office.

**Requisitos Funcionais cobertos**: FR-1.3, FR-4.5

**Depende de**: 2.0, 9.0.

**Constitution**: respects P-001, P-002, P-004, P-006, P-007.

<requirements>
- Definir `OfficeError` como enum serde `tag = "code"`.
- Preservar código de `LaunchError` em `details.launchCode` ao envelopar `office_windows_failed`.
- Criar comandos Tauri com nomes verbo_substantivo, args camelCase e alias quando necessário.
- Declarar subcampos de `OfficeProvisioningState` que são contrato do frontend.
- Emitir `operation-progress` com steps/fases Office e status `error` em falha.
</requirements>

## Subtarefas

### Implementação
- [ ] 10.1 Criar `commands/office.rs` com comandos Tauri Office.
- [ ] 10.2 Definir `OfficeError` e conversões explícitas sem substring de mensagem.
- [ ] 10.3 Registrar comandos em `lib.rs`.
- [ ] 10.4 Implementar emissão de `operation-progress` para as 13 fases.

### Testes Automatizados
- [ ] 10.5 Teste `office_error_serializes_with_code_tag` — invariante: JSON tem `code` estável; camada: unit Rust commands/office.
- [ ] 10.6 Teste `launch_error_bridge_preserves_original_code` — invariante: falha Windows vira `office_windows_failed` com `details.launchCode`; camada: unit Rust commands/office.
- [ ] 10.7 Teste `operation_progress_office_error_status_shape` — invariante: evento JSONL/Tauri mantém chaves esperadas; camada: integração commands com app/event mock.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `OfficeError` serde | variante simples, variante com details, bridge LaunchError |
| `office_get_state` | perfil Office, perfil não Office, activeSessions desconhecido |
| `emit_office_progress` | pending, running, done, error, skipped |

### Mocks Necessários
- `MockGuestExecutor`.
- `MockDocker`.
- App handle/event emitter fake.

## Detalhes de Implementação

Ver techspec §§Contratos de API, Taxonomia de Erro, Eventos operation-progress, Hyrum.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- office_error operation_progress` passa.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte de contratos Office passando.

## Arquivos Relevantes
- `src-tauri/src/commands/office.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/core/launch_error.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/commands src-tauri/src/lib.rs src-tauri/src/core
git commit -m "feat(office): expose tauri office commands"
```

## Related ADRs

- n/a.

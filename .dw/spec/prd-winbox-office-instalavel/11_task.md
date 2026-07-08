---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 11.0: Implementar CLI `winbox office` com envelope preservado

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Adicionar subcomando `winbox office` com handler único para humano e `--json`, preservando `OfficeError.code` sem passar pelo `machine_error_code` legado.

**Requisitos Funcionais cobertos**: FR-1.3, FR-5.6

**Depende de**: 10.0.

**Constitution**: respects P-002, P-004, P-006.

<requirements>
- Criar `run_office_command -> Result<Value, OfficeError>`.
- Desviar o branch `Cmd::Office` do caminho `dispatch_json -> anyhow -> machine_error_code`.
- Implementar `status`, `launch`, `retry`, `remove`, `preflight`, `provision` e `adopt` conforme techspec.
- Definir `--progress` jsonl com chaves `type/profile/operation/phase/status/message/timestamp`.
- Manter contratos `clia`/`neodrive` existentes intocados.
</requirements>

## Subtarefas

### Implementação
- [ ] 11.1 Criar `src-tauri/src/cli_office.rs`.
- [ ] 11.2 Adicionar parser do subcomando `office` sem duplicar dispatch legado.
- [ ] 11.3 Implementar envelope `{ok,value|error}` para humano e `--json`.
- [ ] 11.4 Implementar saída `--progress` jsonl para fases Office.
- [ ] 11.5 Implementar `office provision` e `office adopt` no mesmo handler `run_office_command`.

### Testes Automatizados
- [ ] 11.6 Teste `office_json_error_envelope_preserves_office_error_code` — invariante: `error.code` é idêntico ao `OfficeError`; camada: integração CLI Rust.
- [ ] 11.7 Teste `office_provision_progress_jsonl_emits_error_status` — invariante: `winbox office provision --progress` usa o dialeto contratado e emite `status:error` em falha; camada: integração CLI Rust.
- [ ] 11.8 Teste `office_adopt_uses_same_error_envelope` — invariante: `adopt --json` preserva `OfficeError.code`; camada: integração CLI Rust.
- [ ] 11.9 Teste `legacy_cli_json_contract_unchanged_for_non_office` — invariante: comandos não Office seguem pelo caminho antigo; camada: integração CLI Rust.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `run_office_command` | status ok, provision, adopt, erro Office, retry phase, remove com token ausente |
| `print_office_json` | ok, error, details preservado |
| `print_progress_jsonl` | running, done, error |

### Mocks Necessários
- `MockOfficeService` ou funções injetáveis equivalentes.
- Captura de stdout/stderr.

## Detalhes de Implementação

Ver techspec §§CLI `winbox office`, Envelope JSON, Contratos de Erro.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- cli_office` passa.
- Teste de contrato prova que `machine_error_code` não toca erros Office.

## Arquivos Relevantes
- `src-tauri/src/cli_office.rs`
- `src-tauri/src/cli.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/cli.rs src-tauri/src/cli_office.rs src-tauri/src/commands/office.rs
git commit -m "feat(office): add office cli command"
```

## Related ADRs

- n/a.

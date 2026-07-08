---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 24.0: Implementar cliente de telemetria opt-in

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar telemetria mínima no cliente winbox, opt-in, com pseudônimo por instalação, sem IDs persistentes de máquina e envio fire-and-forget por `curl`.

**Requisitos Funcionais cobertos**: FR-8.1, FR-8.2

**Depende de**: 18.0.

**Constitution**: respects P-004, P-006, P-007, P-008.

<requirements>
- Guardar preferência de opt-in sem ativar por padrão.
- Gerar pseudônimo por instalação sem coletar hostname, MAC, serial ou machine-id.
- Ler a URL do endpoint de uma constante em `core/paths.rs` com placeholder rotacionável por release.
- Enviar JSON por `curl --max-time 5` via `std::process::Command`.
- Não bloquear provisionamento quando telemetria falhar.
- UI deve permitir ligar/desligar telemetria com copy localizada.
</requirements>

## Subtarefas

### Implementação
- [ ] 24.1 Criar `core/telemetry.rs` com shape de evento e pseudônimo.
- [ ] 24.2 Adicionar constante placeholder da URL em `core/paths.rs`.
- [ ] 24.3 Implementar envio fire-and-forget por `curl`.
- [ ] 24.4 Adicionar preferência opt-in no wizard.
- [ ] 24.5 Emitir eventos de fase do wizard quando opt-in estiver ativo.

### Testes Automatizados
- [ ] 24.6 Teste `telemetry_opt_in_defaults_to_false` — invariante: instalação nova não envia eventos; camada: unit Rust core/telemetry.
- [ ] 24.7 Teste `telemetry_payload_has_no_machine_identifiers` — invariante: payload não contém hostname/MAC/machine-id; camada: unit Rust core/telemetry.
- [ ] 24.8 Teste `telemetry_endpoint_url_comes_from_paths_constant` — invariante: URL pública é placeholder rotacionável em `core/paths.rs`; camada: unit Rust core/telemetry.
- [ ] 24.9 Teste `telemetry_curl_failure_is_best_effort` — invariante: falha do curl não quebra provisionamento; camada: unit Rust core/telemetry.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `build_telemetry_event` | fase started/done/error, opt-in off, sem IDs proibidos |
| `send_telemetry_event` | curl ok, timeout, endpoint indisponível |
| `renderTelemetryPreference` | ligado, desligado, i18n |

### Mocks Necessários
- Trait de comando host para `curl`.
- Store temporário de preferências.
- Callback JS fake para preferência.

## Detalhes de Implementação

Ver techspec §§Telemetria, Privacidade, Minimalismo, UI State Matrix.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- telemetry` passa.
- `node --test src/office-wizard.test.js` passa se a preferência UI for tocada.
- Nenhuma dependência Rust nova é adicionada.

## Arquivos Relevantes
- `src-tauri/src/core/telemetry.rs`
- `src-tauri/src/core/paths.rs`
- `src-tauri/src/core/mod.rs`
- `src-tauri/src/commands/office.rs`
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs src/office-wizard.js src/office-wizard.test.js src/locales
git commit -m "feat(office): add opt-in telemetry client"
```

## Related ADRs

- n/a.

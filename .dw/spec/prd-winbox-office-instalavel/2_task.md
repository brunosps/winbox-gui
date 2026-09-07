---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 2.0: Modelar estado persistido, fases e retry em cascata

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Criar o modelo central de estado Office em Rust, com arquivo persistido por perfil, 13 fases, status por fase, idempotência e grafo de invalidação em cascata.

**Requisitos Funcionais cobertos**: FR-6.1, FR-6.2

**Depende de**: 1.0.

**Constitution**: respects P-004, P-006.

<requirements>
- Implementar `core/office_state.rs` como módulo puro/testável.
- Persistir estado no diretório do perfil no formato definido no techspec.
- Modelar as 13 fases na ordem aprovada, com `remoteapp_prepare` antes de qualquer uso do executor guest.
- Implementar retry de fase N rebaixando todas as fases dependentes para `pending`.
- Garantir que `ready` depende de `final_verify`, não de `first_launch`.
</requirements>

## Subtarefas

### Implementação
- [ ] 2.1 Definir tipos de fase, status, erro resumido, timestamps e versão de schema.
- [ ] 2.2 Implementar leitura/escrita atômica do estado no diretório do perfil.
- [ ] 2.3 Implementar grafo de dependência e função pura de retry em cascata.
- [ ] 2.4 Implementar cálculo de status resumido do perfil Office.
- [ ] 2.5 Instalar `cargo-llvm-cov` localmente com `cargo install cargo-llvm-cov` OU registrar waiver explícito; cobertura obrigatória só será consolidada na task 29.0.

### Testes Automatizados
- [ ] 2.6 Teste `retry_phase_invalidates_descendants` — invariante: retry de fase N rebaixa dependentes para `pending` e preserva ancestrais; camada: unit Rust core/office_state.
- [ ] 2.7 Teste `retry_refused_for_unsafe_phase` — invariante: fase `done` sem marcação retry-safe recusa retry; camada: unit Rust core/office_state.
- [ ] 2.8 Teste `ready_ignores_first_launch_phase` — invariante: `ready` nasce em `final_verify`; camada: unit Rust core/office_state.
- [ ] 2.9 Teste `office_state_roundtrip_preserves_schema_version` — invariante: estado persistido sobrevive a reload; camada: unit Rust core/office_state.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `retry_from_phase` | fase inicial, fase intermediária, fase final, fase desconhecida recusada, fase `done` unsafe recusada |
| `summarize_state` | pending, running, failed, ready, first_launch informativo |
| `load/save` | arquivo ausente, schema atual, JSON inválido com erro estruturado |

### Mocks Necessários
- Diretório temporário de perfil.

## Detalhes de Implementação

Ver techspec §§Modelos de Dados, Sequência de Fases, Retry e Invalidação em Cascata.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- office_state` passa.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte `office_state` passando.

## Arquivos Relevantes
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/core/mod.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core/office_state.rs src-tauri/src/core/mod.rs
git commit -m "feat(office): add persistent provisioning state"
```

## Related ADRs

- `adrs/adr-office-odt-download-strategy.md` — influencia as fases ODT.

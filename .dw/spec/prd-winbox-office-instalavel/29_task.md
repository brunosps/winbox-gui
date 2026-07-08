---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 29.0: Fechar gates de cobertura e regressão de contrato

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Adicionar gates finais de cobertura e regressão de contrato para garantir que upgrade, CLI JSON, erros e telemetria beta não quebrem antes do PR.

**Requisitos Funcionais cobertos**: FR-1.4, FR-8.4

**Depende de**: 10.0, 18.0, 26.0, 27.0, 28.0.

**Constitution**: respects P-002, P-004, P-006, P-008.

<requirements>
- Medir cobertura Rust com `cargo-llvm-cov` para módulos Office.
- Medir cobertura JS com `node --test --experimental-test-coverage` ou `c8` no CI.
- Adicionar checks de contrato para CLI `--json`, `OfficeError`, estado frontend e progress jsonl.
- Incluir verificação de upgrade preservado no QA final.
- Validação em VM real é gate de `/dw-qa` antes do beta: mínimo 3 provisionamentos limpos + 2 adoções.
- Não criar `tasks-validation.md`; o orquestrador gera.
</requirements>

## Subtarefas

### Implementação
- [ ] 29.1 Adicionar job/step de cobertura Rust para core/commands Office.
- [ ] 29.2 Adicionar job/step de cobertura JS para `office-wizard`.
- [ ] 29.3 Consolidar testes de contrato Office como suite nomeada.
- [ ] 29.4 Documentar no PR checklist final de beta/release sem editar `.dw/rules/`.
- [ ] 29.5 Adicionar checklist `/dw-qa` real: 3 provisionamentos limpos, 2 adoções e upgrade preservado.

### Testes Automatizados
- [ ] 29.6 Teste `office_contract_suite_preserves_public_shapes` — invariante: shapes Tauri/CLI/eventos não mudam sem teste; camada: integração commands + CLI.
- [ ] 29.7 Teste `office_coverage_jobs_measure_new_modules` — invariante: CI mede os módulos Office novos; camada: CI config check.
- [ ] 29.8 Teste `office_beta_release_gate_includes_upgrade_scenario` — invariante: QA final roda upgrade vN -> vN+1; camada: E2E wdio/CI.
- [ ] 29.9 Gate manual `/dw-qa` `office_real_vm_beta_readiness` — invariante: 3 provisionamentos limpos + 2 adoções passam em VM real antes do beta; camada: QA manual/real system.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| contrato Office | erro serde, CLI envelope, progress jsonl, frontend state subset |
| cobertura CI | Rust core/commands, JS wizard, E2E upgrade referenciado |
| QA real | 3 provisionamentos limpos, 2 adoções, upgrade preservado |

### Mocks Necessários
- Fixtures JSON de contratos.
- CI config fixtures.
- Shims E2E das tasks 27.0 e 28.0.
- VM real apenas no gate `/dw-qa`, fora da suíte mockada.

## Detalhes de Implementação

Ver techspec §§Estratégia de Testes, Contratos de API, Empacotamento/CI, Auto-gate.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml` passa.
- `npm run check:js` passa.
- `npm run test:js` passa.
- `cargo-llvm-cov` reporta cobertura perto de 80% core Office e 70% commands Office.
- Checklist `/dw-qa` real está registrado: 3 provisionamentos limpos + 2 adoções antes do beta.

## Arquivos Relevantes
- `.github/workflows/ci.yml`
- `.github/workflows/*release*`
- `src-tauri/src/commands/office.rs`
- `src-tauri/src/core/office_state.rs`
- `src/office-wizard.test.js`
- `tests/e2e/*`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add .github/workflows src-tauri src tests/e2e
git commit -m "test(office): add coverage and contract gates"
```

## Related ADRs

- n/a.

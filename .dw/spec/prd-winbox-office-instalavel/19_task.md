---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 19.0: Implementar aceite BYOL e bloqueios legais no wizard

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Adicionar a etapa BYOL do wizard com aceite obrigatório antes de criar, baixar ou provisionar qualquer recurso Office.

**Requisitos Funcionais cobertos**: FR-3.1, FR-3.2

**Depende de**: 18.0.

**Constitution**: respects P-001, P-003, P-004, P-008.

<requirements>
- Exibir disclaimer BYOL antes de qualquer efeito colateral.
- Bloquear criação/download/provisionamento até aceite explícito.
- Persistir aceite via comando backend validado, não confiando só no frontend.
- Garantir copy em pt-BR e en-US.
</requirements>

## Subtarefas

### Implementação
- [ ] 19.1 Adicionar etapa BYOL ao estado do wizard.
- [ ] 19.2 Ligar aceite ao comando backend correspondente.
- [ ] 19.3 Bloquear botões de continuidade antes do aceite.
- [ ] 19.4 Adicionar textos i18n do disclaimer e confirmação.

### Testes Automatizados
- [ ] 19.5 Teste `byol_acceptance_blocks_side_effects_until_checked` — invariante: callbacks de provisionamento não disparam sem aceite; camada: node:test JS puro.
- [ ] 19.6 Teste `byol_backend_rejects_missing_acceptance` — invariante: backend revalida aceite; camada: integração commands.
- [ ] 19.7 Teste `byol_disclaimer_i18n_parallel` — invariante: textos existem nos dois locales; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `renderByolStep` | não aceito, aceito, loading, erro |
| `canStartProvisioning` | aceite ausente, aceite presente |

### Mocks Necessários
- Callback fake de `office_start_provisioning`.
- Comando backend mockado para aceite.

## Detalhes de Implementação

Ver techspec §§BYOL, UI State Matrix, Contratos de API.

## Critérios de Sucesso

- `npm run check:js` passa.
- `node --test src/office-wizard.test.js` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- byol` passa se houver comando backend novo nesta task.

## Arquivos Relevantes
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src src-tauri/src/commands/office.rs
git commit -m "feat(office): require byol acceptance"
```

## Related ADRs

- n/a.

---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 20.0: Implementar UI de preflight, warnings e adoção

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Renderizar no wizard os checks de preflight com ação concreta, warnings com override explícito e revisão de adoção manual com `managedScope`.

**Requisitos Funcionais cobertos**: FR-2.2, FR-6.4

**Depende de**: 4.0, 6.0, 14.0, 18.0.

**Constitution**: respects P-003, P-004, P-006, P-008.

<requirements>
- Mostrar `requirement`, `impact` e `action_hint` de cada `PreflightCheck`.
- Warnings de recurso exigem override explícito.
- Adoção mostra sinais detectados, escopo gerenciado e risco antes de continuar.
- Estado visual cobre `warning-override`, `adoption-review`, erro, success e loading.
- Todo dado vindo do backend é escapado.
</requirements>

## Subtarefas

### Implementação
- [ ] 20.1 Renderizar matriz de preflight no wizard.
- [ ] 20.2 Implementar override explícito de warning.
- [ ] 20.3 Renderizar revisão de adoção com `AdoptionFinding`.
- [ ] 20.4 Adicionar estados de erro/success/loading e i18n correspondente.

### Testes Automatizados
- [ ] 20.5 Teste `preflight_blocker_renders_action_hint_escaped` — invariante: blocker sempre mostra hint escapado; camada: node:test JS puro.
- [ ] 20.6 Teste `resource_warning_requires_ui_override` — invariante: warning não habilita continuação sem confirmação; camada: node:test JS puro.
- [ ] 20.7 Teste `adoption_review_requires_explicit_choice` — invariante: adoção não segue por default; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `renderPreflightStep` | ok, warning, blocker, loading, error |
| `renderAdoptionReview` | nenhum finding, gerenciado, misto, conflito |
| `canContinueFromPreflight` | ok, warning sem override, warning com override, blocker |

### Mocks Necessários
- Fixtures `PreflightCheck`, `Resources` e `AdoptionFinding`.
- Callbacks Tauri fake.

## Detalhes de Implementação

Ver techspec §§UI State Matrix, PreflightCheck, Adoção, Acessibilidade.

## Critérios de Sucesso

- `npm run check:js` passa.
- `node --test src/office-wizard.test.js` passa.
- Contraste AA e navegação por teclado são verificados no review visual/E2E posterior.

## Arquivos Relevantes
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/styles.css`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src/office-wizard.js src/office-wizard.test.js src/styles.css src/locales
git commit -m "feat(office): render preflight and adoption review"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — adoção respeita escopo de artefatos.

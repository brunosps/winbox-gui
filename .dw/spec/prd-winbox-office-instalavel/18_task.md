---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 18.0: Criar shell do wizard Office e entrada no app

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Criar módulo JS novo `office-wizard.js`, puro/testável/injetável, e ligar uma entrada mínima no app para criar perfil Office sem inflar `main.js`.

**Requisitos Funcionais cobertos**: FR-1.2, FR-4.1

**Depende de**: 10.0.

**Constitution**: respects P-003, P-004, P-008.

<requirements>
- Seguir autoridade visual do `DESIGN.md`: tokens, ops-bar e checklist a11y.
- Criar wizard multi-etapa em módulo novo, no padrão `profile-display.js`.
- `main.js` deve só fazer fiação mínima/injeção de dependências.
- Toda string user-facing entra em `en-US.js` e `pt-BR.js`.
- Garantir teclado end-to-end, focus-visible e targets adequados desde o shell.
</requirements>

## Subtarefas

### Implementação
- [ ] 18.1 Criar `src/office-wizard.js` com render puro e dependências injetadas.
- [ ] 18.2 Adicionar fiação mínima no `src/main.js`.
- [ ] 18.3 Adicionar estilos do shell do wizard usando tokens existentes.
- [ ] 18.4 Adicionar chaves i18n iniciais em `en-US.js` e `pt-BR.js`.
- [ ] 18.5 Adicionar `office-wizard.js` e módulos JS novos à lista hardcoded do script `check:js` em `package.json`.

### Testes Automatizados
- [ ] 18.6 Teste `office_wizard_shell_escapes_dynamic_profile_data` — invariante: dados dinâmicos são escapados; camada: node:test JS puro.
- [ ] 18.7 Teste `office_wizard_locale_keys_are_parallel` — invariante: chaves novas existem nos dois locales; camada: node:test JS puro.
- [ ] 18.8 Teste `office_wizard_initial_state_is_keyboard_reachable` — invariante: controles iniciais expõem foco e roles; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `renderOfficeWizard` | estado inicial, perfil existente, dados escapados |
| `bindOfficeWizard` | callbacks injetados, eventos de teclado, teardown |

### Mocks Necessários
- Dependências Tauri injetadas como funções fake.
- DOM em ambiente de teste JS existente.

## Detalhes de Implementação

Ver techspec §§UI dw-ui-discipline, Wizard, Frontend JS, Acessibilidade.

## Critérios de Sucesso

- `npm run check:js` passa.
- `node --test src/office-wizard.test.js` passa.
- `main.js` recebe apenas fiação mínima.

## Arquivos Relevantes
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/main.js`
- `src/styles.css`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`
- `package.json`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src/office-wizard.js src/office-wizard.test.js src/main.js src/styles.css src/locales package.json
git commit -m "feat(office): add office wizard shell"
```

## Related ADRs

- n/a.

---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 23.0: Publicar escopo single-user e orientação de licença

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Adicionar orientação de produto e wizard sobre escopo single-user/single-machine e licenciamento sem enforcement programático.

**Requisitos Funcionais cobertos**: FR-3.3, FR-3.4

**Depende de**: 1.0, 18.0.

**Constitution**: respects P-003, P-008.

<requirements>
- Mostrar que o perfil Office é escopo single-user/single-machine.
- Explicar que o winbox não valida licença nem aplica compliance Microsoft.
- Manter orientação estática e localizada.
- Não adicionar telemetria ou persistência extra nesta task.
</requirements>

## Subtarefas

### Implementação
- [ ] 23.1 Adicionar copy de escopo no wizard Office.
- [ ] 23.2 Adicionar copy de orientação de licença sem enforcement.
- [ ] 23.3 Adicionar chaves i18n pt-BR/en-US.
- [ ] 23.4 Linkar a superfície aos ADRs da task 1.0 quando apropriado.

### Testes Automatizados
- [ ] 23.5 Teste `license_guidance_has_no_programmatic_enforcement_branch` — invariante: orientação não bloqueia por plano/ativação detectada; camada: node:test JS puro.
- [ ] 23.6 Teste `single_user_scope_copy_is_localized` — invariante: copy existe nos dois locales; camada: node:test JS puro.
- [ ] 23.7 Teste `license_guidance_escapes_dynamic_links` — invariante: links/dados renderizados passam por escape; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `renderLicenseScopeStep` | en-US, pt-BR, sem enforcement, link seguro |

### Mocks Necessários
- DOM test fake.
- Locale fixtures.

## Detalhes de Implementação

Ver techspec §§BYOL, Licensing, UI dw-ui-discipline, Related ADRs.

## Critérios de Sucesso

- `npm run check:js` passa.
- `node --test src/office-wizard.test.js` passa.
- Nenhuma validação de licença Microsoft é adicionada.

## Arquivos Relevantes
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src/office-wizard.js src/office-wizard.test.js src/locales
git commit -m "docs(office): add license and scope guidance"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — explica fronteira de responsabilidade.

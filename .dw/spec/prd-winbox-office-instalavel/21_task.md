---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 21.0: Implementar UI de provisionamento, duração e progresso

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Renderizar progresso das 13 fases, suportar retomada depois de fechar o app, apresentar expectativa de duração e criar a superfície frontend da janela `office-progress`.

**Requisitos Funcionais cobertos**: FR-4.5, FR-4.6

**Depende de**: 10.0, 11.0, 18.0.

**Constitution**: respects P-002, P-003, P-004, P-006, P-008.

<requirements>
- Consumir `office_get_state` e eventos `operation-progress`.
- Mostrar fases `pending/running/done/failed/skipped` com `aria-live` no progresso.
- Mostrar expectativa de duração de até cerca de 45 minutos e aviso de downloads grandes.
- Ao abrir o app, resumir estado persistido do perfil.
- Retry de fase deve refletir invalidação em cascata no UI.
- Mostrar erro por `code` com ação guiada quando existir.
- Implementar módulo/tela mínima da janela `winbox --window=office-progress <perfil> <app>`, com strings i18n, alimentada pelos mesmos eventos.
</requirements>

## Subtarefas

### Implementação
- [ ] 21.1 Renderizar timeline das 13 fases com estados da matrix.
- [ ] 21.2 Mostrar mensagem de duração esperada e downloads grandes antes de iniciar provisionamento.
- [ ] 21.3 Assinar `operation-progress` e atualizar UI incrementalmente.
- [ ] 21.4 Implementar resume consultando `office_get_state`.
- [ ] 21.5 Implementar ação de retry por fase e diagnóstico de erro.
- [ ] 21.6 Criar `src/office-progress-window.js` e fiação mínima para `--window=office-progress`.
- [ ] 21.7 Adicionar `office-progress-window.js` à lista hardcoded do script `check:js` em `package.json`.

### Testes Automatizados
- [ ] 21.8 Teste `provisioning_intro_shows_duration_expectation` — invariante: intro mostra até ~45min e downloads grandes; camada: node:test JS puro.
- [ ] 21.9 Teste `provisioning_progress_uses_aria_live` — invariante: mudanças de fase são anunciadas; camada: node:test JS puro.
- [ ] 21.10 Teste `office_progress_window_renders_same_operation_events` — invariante: janela `office-progress` consome o mesmo shape de evento; camada: node:test JS puro.
- [ ] 21.11 Teste `wizard_resume_renders_persisted_failed_phase` — invariante: app fechado reabre no estado persistido; camada: node:test JS puro.
- [ ] 21.12 Teste `retry_ui_repaints_invalidated_descendants` — invariante: descendentes voltam a pending depois do retry; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `renderProvisioningTimeline` | pending, running, failed, ready, resumed |
| `renderProvisioningIntro` | duração ~45min, downloads grandes, i18n |
| `renderOfficeProgressWindow` | cold-start, running, error, done |
| `applyProgressEvent` | status running, done, error, evento de outro perfil ignorado |
| `renderOfficeError` | code conhecido, details, code desconhecido seguro |

### Mocks Necessários
- Event emitter fake.
- Fixtures de `OfficeProvisioningState`.
- Callbacks Tauri fake para retry/status.

## Detalhes de Implementação

Ver techspec §§Wizard, Eventos operation-progress, Retry e Invalidação, Contratos de Erro.

## Critérios de Sucesso

- `npm run check:js` passa.
- `node --test src/office-wizard.test.js` passa.
- UI não depende de texto humano para classificar erro.

## Arquivos Relevantes
- `src/office-wizard.js`
- `src/office-progress-window.js`
- `src/office-wizard.test.js`
- `src/main.js`
- `src/styles.css`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`
- `package.json`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src/office-wizard.js src/office-progress-window.js src/office-wizard.test.js src/main.js src/styles.css src/locales package.json
git commit -m "feat(office): show provisioning progress and retry"
```

## Related ADRs

- n/a.

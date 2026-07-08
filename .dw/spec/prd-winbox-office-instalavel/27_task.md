---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 27.0: Corrigir harness E2E e cobrir fluxo do wizard

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Consertar o harness E2E existente, criar shims por `PATH` para dependências externas e cobrir o fluxo do wizard Office com provisionamento mockado.

**Requisitos Funcionais cobertos**: FR-4.6

**Depende de**: 20.0, 21.0, 22.0, 26.0.

**Constitution**: respects P-004, P-006, P-008.

<requirements>
- Criar `tests/e2e/fixtures/mock-docker.sh`, exatamente como referenciado por `tests/e2e/wdio.conf.mjs`.
- Atualizar seletores stale do harness.
- Criar shims de `flatpak`, `git`, `curl`, `xfreerdp`, `winapps-setup` e `notify-send` controláveis por fixture/env.
- Cobrir fluxo do wizard com provisionamento mockado até ready.
- Validar que a UI apresenta a expectativa de duração (~45min) e downloads grandes.
</requirements>

## Subtarefas

### Implementação
- [ ] 27.1 Criar fixture `tests/e2e/fixtures/mock-docker.sh`.
- [ ] 27.2 Criar diretório de shims PATH para dependências externas.
- [ ] 27.3 Atualizar seletores stale e helpers do harness.
- [ ] 27.4 Adicionar spec E2E Office para wizard com provisionamento mockado.

### Testes Automatizados
- [ ] 27.5 Teste `e2e_office_wizard_mock_provisioning_reaches_ready` — invariante: wizard conclui sem Docker/VM real; camada: E2E wdio.
- [ ] 27.6 Teste `e2e_office_wizard_shows_duration_expectation` — invariante: wizard mostra duração esperada e downloads grandes antes do provisionamento; camada: E2E wdio.
- [ ] 27.7 Teste `e2e_harness_uses_path_shims_not_host_binaries` — invariante: E2E não usa Docker/Flatpak/Git/FreeRDP reais; camada: E2E wdio.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| E2E harness | wizard feliz, preflight blocker, adoção, duração esperada |

### Mocks Necessários
- `tests/e2e/fixtures/mock-docker.sh`.
- Shims PATH: `flatpak`, `git`, `curl`, `xfreerdp`, `winapps-setup`, `notify-send`.
- Fixtures/env para sucesso, falha, timeout e adoção.

## Detalhes de Implementação

Ver techspec §§Estratégia de Testes, E2E Executável, Wizard, UI State Matrix.

## Critérios de Sucesso

- `npm run test:e2e` ou comando E2E existente passa no fluxo wizard Office mockado.
- `npm run check:js` passa se helpers JS forem tocados.
- E2E do wizard não depende de Docker, Flatpak, Git, FreeRDP ou WinApps reais.

## Arquivos Relevantes
- `tests/e2e/fixtures/mock-docker.sh`
- `tests/e2e/fixtures/shims/*`
- `tests/e2e/*`
- `tests/e2e/wdio.conf.mjs`
- `src/office-wizard.js`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add tests/e2e tests/e2e/wdio.conf.mjs
git commit -m "test(e2e): cover office wizard flow"
```

## Related ADRs

- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — shims modelam flags FreeRDP.

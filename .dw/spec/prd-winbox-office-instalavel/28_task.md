---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 28.0: Cobrir E2E de launch, lifecycle e upgrade Office

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Adicionar os fluxos E2E de cold-start launch, aviso de lifecycle com apps abertos e upgrade preservando estado/menu/associações, reusando o harness e os shims da task 27.0.

**Requisitos Funcionais cobertos**: FR-1.4, FR-7.7

**Depende de**: 16.0, 17.0, 26.0, 27.0.

**Constitution**: respects P-004, P-006, P-008.

<requirements>
- Cobrir cold-start launch com progresso visível.
- Cobrir `office_apps_maybe_open` em stop/restart/pause.
- Cobrir upgrade vN -> vN+1 preservando estado, menu e associações.
- Fallback aceitável: se build duplo for inviável no harness, simular upgrade trocando o binário e validar estado/menu; checklist manual na task 29.0 é a última linha de defesa.
- Reusar shims PATH da task 27.0 para `flatpak`, `git`, `curl`, `xfreerdp`, `winapps-setup` e `notify-send`.
</requirements>

## Subtarefas

### Implementação
- [ ] 28.1 Adicionar spec E2E de launch cold-start com `--gui-progress`.
- [ ] 28.2 Adicionar spec E2E de lifecycle com `office_apps_maybe_open`.
- [ ] 28.3 Adicionar spec E2E de upgrade preservado.
- [ ] 28.4 Implementar fallback de simulação de upgrade por troca de binário quando build duplo for inviável.

### Testes Automatizados
- [ ] 28.5 Teste `e2e_office_launch_shows_cold_start_progress` — invariante: launch exibe progresso antes do app; camada: E2E wdio.
- [ ] 28.6 Teste `e2e_office_lifecycle_warns_when_apps_maybe_open` — invariante: stop/restart/pause exigem confirmação quando há sessão ativa/indeterminada; camada: E2E wdio.
- [ ] 28.7 Teste `e2e_office_upgrade_preserves_state_menu_and_associations` — invariante: upgrade preserva estado, menu e MIME; camada: E2E wdio.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| E2E launch/lifecycle/upgrade | launch cold-start, lifecycle warning, upgrade real, upgrade simulado |

### Mocks Necessários
- Shims PATH da task 27.0.
- Fixtures/env para sessão RDP ativa, VM cold-start e upgrade.

## Detalhes de Implementação

Ver techspec §§E2E Executável, Launcher, Lifecycle, Upgrade QA.

## Critérios de Sucesso

- `npm run test:e2e` ou comando E2E existente passa nos fluxos launch/lifecycle/upgrade.
- O fallback de upgrade simulado é documentado no log do teste quando usado.
- E2E não depende de VM real; validação real fica no gate `/dw-qa` da task 29.0.

## Arquivos Relevantes
- `tests/e2e/fixtures/shims/*`
- `tests/e2e/*`
- `tests/e2e/wdio.conf.mjs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add tests/e2e tests/e2e/wdio.conf.mjs
git commit -m "test(e2e): cover office launch lifecycle and upgrade"
```

## Related ADRs

- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — shims modelam flags FreeRDP.

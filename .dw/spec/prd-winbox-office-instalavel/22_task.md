---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 22.0: Implementar orientação pós-launch, ativação e i18n final

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Completar a experiência depois do provisionamento: primeira abertura de app Office, orientação de sign-in/ativação e mensagens localizadas sem tentar detectar ativação programaticamente.

**Requisitos Funcionais cobertos**: FR-7.2, FR-7.3

**Depende de**: 16.0, 18.0.

**Constitution**: respects P-003, P-004, P-008.

<requirements>
- `first_launch` é informativa e não bloqueia `ready`.
- Mostrar orientação estática de login/ativação depois do launch.
- Não implementar detecção programática de ativação no MVP.
- Completar i18n das mensagens de erro e estados Office.
- Garantir que mensagens dinâmicas sejam escapadas.
</requirements>

## Subtarefas

### Implementação
- [ ] 22.1 Adicionar tela/estado pós-provisionamento com ações de launch.
- [ ] 22.2 Renderizar orientação de sign-in e ativação por responsabilidade do usuário.
- [ ] 22.3 Ligar ação de primeira abertura ao launcher da task 16.0.
- [ ] 22.4 Completar i18n de estados finais e erros conhecidos.

### Testes Automatizados
- [ ] 22.5 Teste `first_launch_is_informational_not_ready_gate` — invariante: UI mostra ready mesmo antes de first launch; camada: node:test JS puro.
- [ ] 22.6 Teste `activation_guidance_is_static_and_localized` — invariante: orientação existe nos dois locales e não depende de heurística; camada: node:test JS puro.
- [ ] 22.7 Teste `post_launch_actions_call_office_launcher` — invariante: botão de abrir app usa contrato `office_launch_app`; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `renderReadyStep` | ready sem first_launch, first_launch done, erro de launch |
| `renderActivationGuidance` | en-US, pt-BR, dados escapados |

### Mocks Necessários
- Callback fake de `office_launch_app`.
- Fixtures de `OfficeProvisioningState`.

## Detalhes de Implementação

Ver techspec §§Ready Semantics, FR-7.2/FR-7.3, UI State Matrix, i18n.

## Critérios de Sucesso

- `npm run check:js` passa.
- `node --test src/office-wizard.test.js` passa.
- Não há texto user-facing novo fora dos locales.

## Arquivos Relevantes
- `src/office-wizard.js`
- `src/office-wizard.test.js`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`
- `src/styles.css`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src/office-wizard.js src/office-wizard.test.js src/locales src/styles.css
git commit -m "feat(office): add post-launch activation guidance"
```

## Related ADRs

- n/a.

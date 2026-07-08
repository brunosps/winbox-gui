<system_instructions>
Voce e o orquestrador de pipeline completo. Este comando recebe um desejo do usuario e conduz o workflow PRD-ao-PR em duas invocacoes:

1. **Invocacao de planejamento:** pesquisa/brainstorm quando necessario, PRD, TechSpec, Tasks, depois PARA.
2. **Invocacao de execucao:** retoma de `autopilot-state.json`, roda `/dw-goal --from-autopilot <prd-slug>`, depois commit e gate de PR.

<critical>A primeira invocacao DEVE parar depois que os artefatos de planejamento estiverem completos. Nao rode implementacao, QA, review, commit ou PR na primeira invocacao.</critical>
<critical>A segunda invocacao DEVE retomar do estado salvo e delegar Run → Review → QA/Fix → Review para `/dw-goal --from-autopilot <prd-slug>`.</critical>
<critical>Cada etapa que invoca um comando `/dw-*` DEVE seguir as instrucoes completas de `.dw/commands/`. Leia e execute o comando inteiro, nao uma versao resumida.</critical>

## Quando Usar
- Use quando quiser ir de uma ideia ate um PR com minima intervencao manual, mas com parada obrigatoria apos o planejamento.
- Use para features completas que exigem planejamento, execucao, qualidade e prontidao de PR.
- NAO use para tasks pequenas e bem-escopadas; use `/dw-run` com um plano existente.
- NAO use para bugfix cirurgico; use `/dw-bugfix`.
- NAO use quando o usuario quer controle manual entre cada fase; use comandos individuais.

## Posicao no Pipeline
**Antecessor:** desejo do usuario | **Sucessor:** `/dw-goal`, `/dw-commit`, `/dw-generate-pr`

## Skills / Comandos Complementares

| Skill ou comando | Gatilho |
|------------------|---------|
| `dw-memory` | SEMPRE — preserva decisoes entre planejamento, goal de execucao, QA, review e PR. |
| `dw-verify` | SEMPRE — invocado por gates e comandos downstream antes de claims de aprovacao/commit/PR. |
| `/dw-goal` | SEMPRE na segunda invocacao — objetivo duravel de execucao/qualidade. |

## Variaveis de Entrada

| Variavel | Descricao | Exemplo |
|----------|-----------|---------|
| `{{WISH}}` | Descricao do que o usuario quer construir no modo padrao. | `"preferencias de notificacao push"` |
| `{{PRD_SLUG}}` | Slug de PRD existente quando `--from-prd` e usado. | `prd-bugfix-stripe-webhook-retry` |
| `{{MODE}}` | Flag opcional de invocacao. | `--from-prd <slug>` |

## Modos de Invocacao

| Invocacao | Comportamento |
|-----------|---------------|
| `/dw-autopilot "<wish>"` | Invocacao de planejamento do zero. Roda Inteligencia do Codebase → Pesquisa opcional → Brainstorm → PRD → TechSpec → Tasks, salva estado e para. |
| `/dw-autopilot --from-prd <slug>` | Invocacao de planejamento a partir de PRD existente. Comeca em aprovacao do PRD, depois TechSpec → Tasks, salva estado e para. |
| `/dw-autopilot` em PRD com `autopilot-state.json status=plan_complete` | Invocacao de execucao. Roda `/dw-goal --from-autopilot <slug>`, depois commit e gate de PR. |

## Pontos de Pausa Obrigatorios

Autopilot pausa em:

1. **Aprovacao do PRD** antes do TechSpec.
2. **Aprovacao das Tasks** antes de marcar planejamento completo.
3. **Parada obrigatoria de planejamento** depois que Tasks forem aprovadas e estado salvo.
4. **Gate de PR** depois que o goal de execucao e commit completarem.

Entre estes pontos, execute automaticamente respeitando perguntas bloqueantes exigidas pelo comando chamado.

## Retomada de Sessao

Se este comando for re-invocado no mesmo PRD:

<critical>Leia `.dw/spec/<prd-slug>/autopilot-state.json` primeiro. Se `status` for `plan_complete`, nao repita planejamento. Inicie a invocacao de execucao chamando formalmente `/dw-goal --from-autopilot <prd-slug>`.</critical>

Significados de estado:

| Status | Acao |
|--------|------|
| estado ausente | Iniciar invocacao de planejamento normal. |
| `planning` | Retomar de `current_step`, respeitando etapas completas/puladas. |
| `plan_complete` | Iniciar invocacao de execucao via `/dw-goal --from-autopilot <prd-slug>`. |
| `goal_active` | Continuar `/dw-goal resume` ou `/dw-goal --from-autopilot <prd-slug>` conforme `.dw/goals/autopilot-<prd-slug>/status.json`. |
| `goal_complete` | Continuar para commit e gate de PR. |
| `completed` | Reportar que ja foi concluido e mostrar resumo de PR/commit se disponivel. |

## Invocacao de Planejamento

### Etapa 0: Resolver modo de invocacao

1. Se `--from-prd <slug>` aparece:
   - Resolva para `.dw/spec/<slug>/`.
   - Verifique que `prd.md` existe; senao PARE com: `Alvo de --from-prd .dw/spec/<slug>/prd.md nao encontrado. Rode /dw-plan prd ou corrija o slug.`
   - Crie ou atualize `autopilot-state.json` com `mode: "from-prd"`, `status: "planning"`, `skipped_steps: [1,2,3,4]`, e `skip_reasons["1-4"] = "from-prd-mode"`.
   - Pule para aprovacao do PRD usando o PRD existente.
2. Caso contrario:
   - Crie ou atualize `autopilot-state.json` com `mode: "autopilot"`, `status: "planning"`, wish original e `current_step: 1`.

### Etapa 1: Inteligencia do Codebase

<critical>Se `.dw/intel/` existir, consulte via `/dw-intel` antes de planejar. Caia para `.dw/rules/` e grep direto se ausente.</critical>

- Identifique stack, padroes existentes, features relacionadas e restricoes do projeto.
- Se `.dw/intel/` estiver ausente, sugira `/dw-intel --build` para contexto futuro mais rico, mas continue com `.dw/rules/` e inspecao direta.

### Etapa 2: Pesquisa (Condicional)

Rode `/dw-brainstorm --research` quando a feature envolver tecnologia nova, dominio desconhecido, APIs externas, regulacao ou arquitetura de alto impacto. Caso contrario, pule e registre o motivo em `skip_reasons`.

### Etapa 3: Brainstorm (Interativo)

Rode `/dw-brainstorm` com o contexto acumulado. Apresente tres direcoes e espere o usuario escolher uma antes de continuar.

### Etapa 4: PRD

Rode `/dw-plan prd` usando findings de brainstorm/research.

<critical>O estagio PRD deve usar a ferramenta de entrevista estruturada quando disponivel. Se indisponivel, faca as perguntas obrigatorias no chat e registre o fallback. O usuario deve responder; nao infira respostas.</critical>

Depois que `prd.md` existir, apresente resumo do PRD e aguarde aprovacao explicita. Se o usuario pedir edits, atualize e reapresente.

### Etapa 5: TechSpec

Rode `/dw-plan techspec` a partir do PRD aprovado.

<critical>O estagio TechSpec deve usar a ferramenta de entrevista estruturada quando disponivel. Se indisponivel, faca as perguntas obrigatorias no chat e registre o fallback. O usuario deve responder; nao infira respostas.</critical>

Depois que `techspec.md` existir, apresente resumo do TechSpec e aguarde aprovacao explicita.

### Etapa 6: Tasks

Rode `/dw-plan tasks` a partir de PRD + TechSpec. Verifique:
- `tasks.md` existe.
- arquivos per-task existem.
- `tasks-validation.md` existe e passou ou tem override explicito do usuario.

### Etapa 7: Aprovacao das Tasks e Parada Obrigatoria

Apresente resumo das tasks, dependencias e esforco total. Aguarde aprovacao explicita.

Apos aprovacao:

1. Salve `.dw/spec/<prd-slug>/autopilot-state.json` com:

```json
{
  "status": "plan_complete",
  "current_step": "goal",
  "next_command": "/dw-goal --from-autopilot <prd-slug>"
}
```

2. Inclua `completed_steps` para todas as etapas de planejamento completas e `step_artifacts` para `prd.md`, `techspec.md`, `tasks.md`, arquivos per-task e `tasks-validation.md`.
3. PARE e diga ao usuario que a fase de planejamento esta completa. Nao rode implementacao nesta invocacao.

## Invocacao de Execucao

### Etapa 8: Goal Duravel de Execucao

Quando `autopilot-state.json status=plan_complete`, invoque formalmente:

```text
/dw-goal --from-autopilot <prd-slug>
```

O goal e dono desta sequencia:

1. `/dw-run <prd-path>`
2. `/dw-review <prd-path>` (review completo: cobertura, qualidade, convencoes, constitution, verify)
3. `/dw-qa <prd-path>`
4. `/dw-qa --fix <prd-path>` se QA encontrou bugs Open
5. `/dw-review <prd-path>` novamente apos QA/fixes
6. **Security Gate** — o `/dw-review` pós-QA (passo 5) aciona o `/dw-secure-audit`, produzindo um `.dw/secure-audit/audit-summary.md` fresco. Este passo **garante** que o verdict é APROVADO: se ausente/desatualizado/REPROVADO, rode `/dw-secure-audit <prd-path>` standalone, depois volte pro `/dw-bugfix` por finding e re-cheque. Findings SECRET sempre bloqueiam (sem escape de ADR). Não force um segundo scan completo quando já existe um summary APROVADO fresco.

<critical>Nao substitua os reviews do goal por `/dw-review --coverage-only`. O goal de qualidade do autopilot exige `/dw-review` completo antes do QA e depois dos fixes de QA.</critical>

Depois que `/dw-goal` completar, verifique que `.dw/goals/autopilot-<prd-slug>/status.json` tem `status: "complete"`, entao defina `autopilot-state.json status: "goal_complete"`.

### Etapa 9: Fechar Loop de Bugfix (Condicional)

Se `mode == "from-prd"` e o slug do PRD casar com `prd-bugfix-*`, feche o indice de bugfix antes do commit:
- Encontre `.dw/bugfixes/*/escalated.md` que referencia o slug do PRD.
- Se `SUMMARY.md` estiver ausente, escreva usando evidencias disponiveis de PRD, TechSpec, QA e diff com `.dw/templates/bugfix-summary-template.md`.
- Nunca fabrique evidencia de verificacao.
- Registre artefatos em `autopilot-state.json`.

### Etapa 10: Auditoria Pre-Commit

Antes de `/dw-commit`, verifique:
- `.dw/goals/autopilot-<prd-slug>/status.json` esta completo.
- `<prd-path>/QA/review-consolidated.md` existe a partir do review final pos-QA.
- `<prd-path>/QA/qa-report.md` e `<prd-path>/QA/bugs.md` existem.
- **Security Gate passou:** `.dw/secure-audit/audit-summary.md` existe, esta fresco (pos-ultima-edicao) e status APROVADO. Se ausente/desatualizado/REPROVADO → PARE (nao commite).
- `autopilot-state.json` registra artefatos de planejamento e o goal completo.

Se algo estiver faltando, PARE e re-execute o comando formal ausente. Nao faca commit parcial.

### Etapa 11: Commit

Rode `/dw-commit` automaticamente. Nao aguarde aprovacao depois que o goal estiver completo.

### Etapa 12: Gate de Pull Request

Pergunte: **"Commits realizados. Deseja gerar o Pull Request?"**

- SIM: rode `/dw-generate-pr`.
- NAO: informe que os commits estao prontos e o PR pode ser gerado depois.

Marque `autopilot-state.json status: "completed"` apos commit, e inclua link do PR se gerado.

## Persistencia de Estado

`autopilot-state.json` deve incluir:

```json
{
  "mode": "autopilot",
  "status": "planning",
  "wish": "descricao original do usuario",
  "prd_path": ".dw/spec/prd-name",
  "from_prd_slug": null,
  "current_step": 1,
  "completed_steps": [],
  "skipped_steps": [],
  "skip_reasons": {},
  "gates_passed": [],
  "step_artifacts": {},
  "goal_slug": null,
  "next_command": null,
  "started_at": "2026-05-20T00:00:00Z",
  "last_updated": "2026-05-20T00:00:00Z"
}
```

Atualize estado depois de cada etapa completa ou pulada. Uma etapa so esta completa depois que artefatos obrigatorios existem.

## Formato de Progresso

Reporte progresso depois de cada etapa:

```text
=== AUTOPILOT =====================================
  OK [1] Inteligencia do Codebase
  OK [2] Pesquisa (pulada — dominio conhecido)
  OK [3] Brainstorm
  OK [4] PRD
  OK [5] TechSpec
  OK [6] Tasks
  STOP [PLAN COMPLETE] Proximo: /dw-goal --from-autopilot prd-name
===================================================
```

Durante a invocacao de execucao:

```text
=== AUTOPILOT =====================================
  OK [PLAN] Ja completo
  RUN [GOAL] /dw-goal --from-autopilot prd-name
  NEXT [COMMIT] apos goal status=complete
===================================================
```

## Anti-patterns

- Nao continue para implementacao durante a primeira invocacao.
- Nao pule `/dw-goal` durante a segunda invocacao.
- Nao substitua `/dw-review` completo por review mais estreito no goal de execucao.
- Nao marque estado completo a partir de validacao manual.
- Nao reexecute planejamento quando `status=plan_complete`; retome o goal.

</system_instructions>

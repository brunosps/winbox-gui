---
type: idea-onepager
schema_version: "1.0"
status: draft
date: YYYY-MM-DD
classification: improves | consolidates | new
---

# Ideia: [Título curto e imperativo]

## Problem Statement

[Reformule a ideia bruta como uma frase "How might we":
**How might we** [verbo] **para** [usuário/segmento] **de forma que** [resultado/valor mensurável]?

Foque no problema, não na solução. Evite entrar em "como implementar".]

## Product Context (features existentes mapeadas)

[Inventário das features do produto relevantes para esta ideia. **Nível de produto, não de código.** Liste o que o produto já entrega hoje que se relaciona com a ideia.

Fontes:
- PRDs em `.dw/spec/prd-*/prd.md` (features já entregues ou em desenvolvimento)
- `.dw/rules/index.md` (overview do produto)
- `.dw/intel/` (indice queryable — construido por `/dw-intel --build`, consultado via `/dw-intel`)

Formato:]

- **[nome da feature A]** — `.dw/spec/prd-<slug>/prd.md` — status: live / em desenvolvimento
- **[nome da feature B]** — `.dw/rules/index.md#modulo-Y` — status: live
- **[nome da feature C]** — PRD em progresso, ver `tasks.md`

> Se o produto é greenfield (sem PRDs nem rules), escreva: "Feature Inventory: greenfield — nenhum artefato de produto ainda. Esta é a primeira ideia registrada."

## Classification & Rationale

**Tipo:** IMPROVES | CONSOLIDATES | NEW

[Escolha UM dos três e justifique:]

- **Se IMPROVES** — qual feature existente está sendo aprimorada e por quê aprimorar vale mais do que criar feature separada. Cite o PRD original da feature.
- **Se CONSOLIDATES** — quais features se fundem, o ganho ao unificar (UX mais coesa, menos código duplicado, dados consolidados). Liste os PRDs originais que ficam "superseded" (ou em revisão).
- **Se NEW** — por que o produto precisa dessa capacidade agora, onde ela se conecta às features existentes (mesmo sendo nova, raramente está completamente isolada), e qual gap ela preenche.

## Recommended Direction

[A abordagem recomendada, 1 parágrafo, em **linguagem de produto**:
- Jornada do usuário (quem faz o quê, quando, por quê)
- Valor entregue
- Boundary (o que essa ideia cobre e o que fica de fora)

**NÃO escreva arquitetura técnica aqui** — isso é trabalho do techspec.]

## MVP Scope

[A menor versão que entrega valor real. Pensada em **user stories**, não tasks técnicas.

- Como [persona], eu posso [ação] para [benefício]
- Como [persona], eu posso [ação] para [benefício]

Idealmente 2-4 stories. Se são mais de 5, provavelmente não é MVP.]

## Not Doing (explícito)

[Itens tentadores que ficaram FORA do escopo — e por quê. Força disciplina de scope:]

- **[item tentador 1]** — razão: [fora de escopo porque...]
- **[item tentador 2]** — razão: [pode virar v2 se hipótese X validar]

## Key Assumptions to Validate

[O que precisa ser verdade para essa direção funcionar. Cada assumption com um teste — idealmente **com usuário**, não com código.]

- **[assumption 1]** — teste: [entrevista com 5 usuários do segmento X / pesquisa de mercado / protótipo de baixa fidelidade]
- **[assumption 2]** — teste: [métrica Y aumenta em Z% em 2 semanas após release]

## Open Questions

[Questões que ainda não têm resposta e que o usuário (ou stakeholder) precisa responder antes do PRD:]

- [Pergunta 1 que afeta escopo]
- [Pergunta 2 que afeta prioridade]

## Next Step

Escolha UM:

- **`/dw-plan prd`** com este one-pager como input — quando a direção está clara mas precisamos detalhar user stories, acceptance criteria e passar ao techspec
- **`/dw-run`** — quando é um IMPROVES tão pequeno que cabe em task única (até 3 arquivos, sem novo endpoint/tela) — escreva um PRD curto antes
- **Parar aqui** — se alguma "Open Question" é bloqueante, parar e resolver com stakeholder antes de avançar

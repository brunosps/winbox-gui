---
schema_version: "1.0"
generated_by: dw-analyze-project (Step 9)
last_refreshed: ""
---

# Concerns — Mapa de Riscos

Mapa de riscos deste codebase. Nao sao convencoes ("como fazemos as coisas" — isso e `.dw/rules/`), nao e arquitetura ("como esta construido" — isso e `.dw/intel/arch.md`). Este arquivo responde uma unica pergunta: **onde e perigoso mexer?**

Carregado on-demand por `/dw-plan`, `/dw-run` e `/dw-bugfix` quando o alvo deles toca uma entrada abaixo. Auto-instalado pelo `/dw-analyze-project` Step 9; nunca bloqueia (ausencia = nenhuma area flagada ainda).

## Hot Spots

Arquivos ou modulos com churn alto, reports frequentes de bug ou historico repetido de "mexi aqui e quebrou algo". Mencione em PRDs que toquem a mesma area; adicione revisor extra ou passada extra de teste.

| Path | Por que e quente | Primeiro flag | Ultimo incidente |
|------|------------------|---------------|------------------|
| _ex. `src/auth/session.ts`_ | _3 fixes de token em 60d_ | _YYYY-MM-DD_ | _YYYY-MM-DD_ |

## Integracoes Fragis

Sistemas externos (APIs, filas, vendors, bancos legados) com historico de falhas silenciosas, drift de schema, surpresas de rate-limit ou comportamento nao-documentado. Codigo novo que toque eles precisa de tratamento explicito de retry/timeout/idempotencia.

| Integracao | Modo de falha | Mitigacao esperada |
|------------|---------------|--------------------|
| _ex. export SAP legado_ | _200 OK silencioso com body vazio quando source esta lockado_ | _checar tamanho do body; logar e alertar_ |

## Codigo Hostil

Funcoes especificas, regexes, parsers ou algoritmos dificeis de raciocinar — quem toca precisa entender 100% primeiro (ou reescrever, nao remendar). Suspeitos comuns: regex artesanal, parsers de string ad-hoc, serializadores custom, async com race condition, codigo de transacao manual.

| Path / funcao | Por que e hostil | Owner / contexto |
|---------------|------------------|------------------|
| _ex. `src/billing/parseInvoice.ts:parseLine`_ | _regex de 900 chars com 12 alternativas, sem comentarios_ | _Bruno escreveu em 2024; reescrever se quebrar_ |

## Historico de Bugs Conhecidos

Agregado de `.dw/bugfixes/*/SUMMARY.md` pelo `/dw-intel --build`. Lista modulos com >=2 fixes historicos. Leia junto com Hot Spots ao planejar trabalho relacionado.

| Modulo | Contagem de bugs | Slugs recentes |
|--------|------------------|----------------|
| _ex. `src/payments/`_ | _4_ | _002-stripe-webhook-retry, 007-refund-rounding_ |

## Tech Debt — Reconhecida

Pedacos de debt que o time concorda que existem. Nao sao para limpar oportunisticamente sem coordenacao — podem ser load-bearing de formas nao obvias.

| Area | Descricao do debt | Por que fica | Trigger de cleanup |
|------|-------------------|--------------|--------------------|
| _ex. `src/legacy/userMapper.ts`_ | _Dois codepaths paralelos de field-mapping_ | _Esperando migracao v3 da API_ | _Q3 2026 apos cutover do vendor_ |

---

**Como manter este arquivo:**

- `/dw-analyze-project` reescreve a cada execucao. Entradas escritas a mao entre `<!-- preserved:start -->` e `<!-- preserved:end -->` sao mantidas.
- Quando um bugfix descobrir uma nova area perigosa, adicione manualmente em Hot Spots e deixe a proxima analise confirmar.
- Promova entradas para `.dw/constitution.md` quando virarem regras nao-negociaveis ("nunca toque X sem ADR").

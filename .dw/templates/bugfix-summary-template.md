---
schema_version: "1.0"
slug: ""
created: ""
status: "Fixed | In Review | QA Pending | Reverted"
severity: "Low | Medium | High"
related_concerns: []
---

# Bugfix Summary — {{NNN}}-{{slug}}

Registro de uma pagina de um bugfix. Arquivos irmaos neste diretorio:

- `TASK.md` — a triagem original, respostas das perguntas de clarificacao e o plano de fix que rodou
- `fix-report.md` — evidencia de verificacao (saida do `dw-verify` PASS, prova de reproducao, execucao do teste de regressao)
- `review/` — populado por `/dw-review --bugfix {{NNN}}-{{slug}}`
- `QA/` — populado por `/dw-qa --bugfix {{NNN}}-{{slug}}` (quando aplicavel)

## Sintoma

O que o usuario observou. Cite a descricao original do bug verbatim; nao parafraseie.

> _"…"_

## Causa Raiz

O que estava de fato quebrado, em uma frase. Nao o sintoma — a causa.

_…_

## Resolucao

O que mudou, em 2-4 bullets. Paths de arquivos, nao snippets.

- _mudanca 1_
- _mudanca 2_

## Arquivos Tocados

Lista completa, incluindo testes. <=5 — se mais, o safety valve deveria ter escalado para `/dw-plan`.

| Path | Mudanca |
|------|---------|
| `src/foo/bar.ts` | _fix cirurgico em X_ |
| `src/foo/bar.test.ts` | _teste de regressao adicionado_ |

## Verificacao

Como o fix foi provado, alem de "os testes passam".

- **Reproducao antes do fix:** _passo que disparava o bug, capturado_
- **Reproducao depois do fix:** _mesmo passo, agora passa_
- **Teste de regressao:** _nome + path_
- **Relatorio de verify:** `fix-report.md`

## Relacionado

- **Concerns tocados:** _refs de `.dw/rules/concerns.md` se o fix caiu em area flagada_
- **Bugfixes adjacentes:** _slugs de fixes anteriores no mesmo modulo, se houver_
- **Contexto de PRD:** _se o bug apareceu dentro de uma feature em andamento, link para o path do PRD_

## Followups

Pontas soltas que este fix descobriu mas nao resolveu. Adicione ao `.dw/STATE.md` Open-Loops ao fechar.

- _nenhum_

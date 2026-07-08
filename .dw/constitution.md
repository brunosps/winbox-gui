---
schema_version: "1.0"
generated_by: dev-workflow
last_updated: 2026-07-08
mode: custom
---

# Constituição do Projeto — winbox-gui

> Princípios declarativos que este time escolheu seguir. PRDs, TechSpecs e Code Reviews leem este arquivo como hard gate. Qualquer coisa que viole um princípio com `severity: critical` ou `high` é bloqueada — exceto quando justificada por um ADR explícito.

## Como este arquivo funciona

- **Cada princípio tem um ID (`P-NNN`), severity, regra, `Why` e `Enforcement`.**
- **Escala de severity:** `info` (apenas reporta, nunca bloqueia) → `high` (bloqueia PR sem ADR) → `critical` (bloqueia PR sem ADR + exige aprovação de reviewer).
- **Edite à vontade.** Promova princípios de `info` para `high` quando confiar que o projeto cumpre.
- **Escape via ADR.** Um PR que viola princípio `high`/`critical` é desbloqueado quando um ADR na mesma feature documenta o desvio e o trade-off.
- **Sintetizado dos padrões observados** em 2026-07-08 (`/dw-analyze-project`, opção A) — cada princípio cita a evidência no código real.

---

## Corretude e Contratos

**P-001 — Validação centralizada antes de qualquer efeito colateral** (severity: info)
**Regra:** Todo dado vindo do usuário (GUI, CLI, config) passa por `core/validation.rs` antes de qualquer I/O; o frontend nunca é confiado — revalidação no Rust é obrigatória.
**Why:** Padrão consistente observado (`.dw/rules/backend-rust.md` §Validação — validation.rs tem Ca=10, invocado na camada commands antes de efeitos colaterais; o frontend duplica checks só por UX). Baseline: `.dw/rules-library/common.md` "validate at the boundary".
**Enforcement:** Review checa que comando/handler novo chama `validation::*` antes de tocar fs/process; campo novo interpolado em script/env exige validador equivalente a `validate_env_value`/`validate_bdf`.

**P-002 — Erro machine-readable usa código estável, nunca substring de mensagem** (severity: info)
**Regra:** Superfície nova consumida por máquina (comando Tauri, CLI `--json`) modela erro como enum serializada com `code` snake_case (padrão `LaunchError`); proibido classificar erro por substring de mensagem humana em código novo.
**Why:** O repo tem os dois regimes e a diferença de qualidade é visível: `core/launch_error.rs` (16 variantes tipadas, roundtrip serde testado) vs `machine_error_code` do CLI por substring PT/EN — fragilidade já exposta aos consumidores clia/neodrive (`.dw/rules/integrations.md` §4).
**Enforcement:** Review flageia `contains(`/`match` sobre texto de erro humano em paths novos; variante nova de erro exige teste de serialização do `code`.

**P-005 — Vendors pinados; upgrade é commit deliberado** (severity: info)
**Regra:** Imagens Docker e URLs de vendor vivem em constantes nomeadas pinadas por versão (`core/paths.rs`); nunca `:latest`; bump de versão é commit dedicado com teste de launch.
**Why:** Pins existentes com teste de regressão anti-`:latest` (compose.rs:389-422) e comentário "Upgrade by bumping these constants intentionally"; o contrato com dockurr/qemux é o acoplamento mais frágil do produto (`.dw/rules/concerns.md` F1–F3).
**Enforcement:** Teste existente bloqueia `:latest`; review de qualquer diff em `paths.rs`.

## Qualidade de Código

**P-004 — Lógica de decisão nasce como função pura testável; I/O atrás de injeção** (severity: info)
**Regra:** Parsing/classificação/formatação nova é função pura com teste (padrão `classify_compose_stderr`, `profile-display.js`); dependência de processo/filesystem entra por trait ou parâmetro (padrão `DockerClient`/`MockDocker`), não por chamada direta.
**Why:** É o padrão da casa nos módulos de melhor qualidade — 108 testes rodam sem Docker; onde foi ignorado (`core/gpu.rs`, parser lspci) o resultado é código hostil sem testes (`.dw/rules/concerns.md` H3).
**Enforcement:** Review checa que parser/heurística nova tem `#[test]`/node:test com fixture real, sem I/O.

**P-006 — Best-effort só com comentário; erro real nunca é engolido** (severity: info)
**Regra:** `let _ =` / catch vazio só para operação comprovadamente best-effort E com comentário explicando o porquê; caminho de erro real propaga com contexto (`.with_context`/`bail!` → `{e:#}` na borda).
**Why:** O repo distingue bem os dois casos quando comenta (lifecycle.rs kill/remove) e mal quando não (docker.rs `rm_force` ignora exit code; main.js:250 catch vazio) — antipattern nº1 da análise. Baseline: `.dw/rules-library/rust.md` "No silent `let _ =` on a real error".
**Enforcement:** `dw-silent-failure-hunter` no `/dw-review`; `let _ =` novo sem comentário é finding.

## Segurança

**P-003 — Escape na renderização é inegociável** (severity: info)
**Regra:** Todo dado dinâmico interpolado em template HTML passa por `escapeHtml`/`escapeAttr`; seletores dinâmicos usam `CSS.escape`. Exceção única: strings vindas de `t()` (locale é fonte confiável).
**Why:** Pilar de segurança do frontend, consistente e testado contra XSS (`src/dom-utils.test.js`); o ponto fraco documentado é ser disciplina manual (`.dw/rules/frontend-js.md` §Segurança).
**Enforcement:** Review de qualquer template novo/alterado no `main.js` verifica escape em cada interpolação `${}`.

**P-007 — Segurança operacional do perfil não regride** (severity: info)
**Regra:** Nenhum comando novo: expõe porta fora de `127.0.0.1`, devolve senha ao frontend, grava segredo fora do `config.env` 0600, ou adiciona capability Tauri — sem ADR justificando.
**Why:** Padrões implementados e declarados (compose.rs amarra loopback; `get_profile_config` zera a senha; `capabilities/default.json` mínimo, sem shell; README §Segurança Operacional).
**Enforcement:** `dw-secure-audit` + review obrigatório de diffs em `compose.rs`, `capabilities/` e qualquer campo `password`.

## Consistência de UX

**P-008 — String de UI entra nos dois locales ou não entra** (severity: info)
**Regra:** Toda string user-facing nova no frontend usa chave i18n presente em `en-US.js` E `pt-BR.js` (paridade 1:1); erro novo do backend destinado à UI prefere `code` mapeável a texto embutido.
**Why:** 224 chaves com paridade exata observada; os vazamentos existentes (strings PT hardcoded no main.js, path pessoal no locale en-US) são antipatterns catalogados (`.dw/rules/frontend-js.md`).
**Enforcement:** Review greppa literais user-facing em templates; contagem de chaves dos dois locales precisa bater.

---

## Princípios Customizados

> Adicione princípios específicos do time abaixo. Mesmo formato: `**P-NNN — <nome>** (severity: info|high|critical): <regra>. **Why:** <motivo>. **Enforcement:** <como>.`

---

## Como evoluir este arquivo

1. **Viva em `info` por pelo menos um release-ciclo.** Observe quão frequentemente cada princípio é violado organicamente; o dado diz se vale promover.
2. **Promova para `high` quando violações forem raras e o time concordar.** PRs que violarem princípio `high` passam a exigir ADR.
3. **Promova para `critical` os princípios que protegem usuários/dados.** Candidatos naturais aqui: P-003 (escape/XSS) e P-007 (segurança operacional).
4. **Demote ou remova princípios que não ganharem seu peso.** Constitution é ferramenta, não museu.
5. **Re-rode `/dw-analyze-project`** quando o codebase mudar substancialmente; ele pode propor updates fundamentados em observação fresca.

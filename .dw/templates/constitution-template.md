---
schema_version: "1.0"
generated_by: dev-workflow
last_updated: YYYY-MM-DD
mode: defaults | custom
---

# Constituição do Projeto

> Princípios declarativos que este time escolheu seguir. PRDs, TechSpecs e Code Reviews leem este arquivo como hard gate. Qualquer coisa que viole um princípio com `severity: critical` ou `high` é bloqueada — exceto quando justificada por um ADR explícito.

## Como este arquivo funciona

- **Cada princípio tem um ID (`P-NNN`), severity, regra, `Why` e `Enforcement`.**
- **Escala de severity:** `info` (apenas reporta, nunca bloqueia) → `high` (bloqueia PR sem ADR) → `critical` (bloqueia PR sem ADR + exige aprovação de reviewer).
- **Edite à vontade.** Este arquivo é seu para evoluir. Promova princípios de `info` para `high` quando confiar que o projeto cumpre.
- **Escape via ADR.** Um PR que viola princípio `high`/`critical` é desbloqueado quando um ADR na mesma feature documenta o desvio e o trade-off.
- **Versão analítica regenerável** a qualquer momento via `/dw-analyze-project` (oferece sintetizar princípios a partir dos padrões observados no código).

---

## Qualidade de Código

**P-001 — Sem `any` / `unknown` em TypeScript sem justificativa** (severity: info)
**Regra:** Código de produção não pode usar `as any`, `as unknown` ou `// @ts-ignore` sem comentário inline `// @ts-ignore — motivo: <X>` nomeando a restrição.
**Why:** Escapes silenciosos do tipo vazam bugs de runtime que o type system existia para pegar. Cada escape é um contrato que o type system para de garantir.
**Enforcement:** `dw-code-review` greppa o diff por `as any`/`@ts-ignore`/`@ts-expect-error` sem comentário-razão correspondente.

**P-002 — Funções devem ser testáveis isoladamente** (severity: info)
**Regra:** Função que toca rede, filesystem, banco de dados ou system clock deve receber a dependência como parâmetro (ou via factory) em vez de importar diretamente.
**Why:** Código que constrói suas próprias dependências não pode ser testado sem setup de integração. Testes ficam lentos, são pulados, e bugs vão pra produção.
**Enforcement:** `dw-code-review` flagueia funções importando `fs`, `axios`, `prisma`, `Date.now`, etc., diretamente em módulos de business logic.

---

## Padrões de Teste

**P-003 — Todo bug fix carrega um regression test** (severity: info)
**Regra:** Commit com tipo `fix:` deve adicionar ou modificar pelo menos um teste que teria pego o bug antes do fix.
**Why:** Sem o teste, o bug volta na próxima vez que alguém refatora a área. O fix se deteriora.
**Enforcement:** `dw-code-review` verifica se commits `fix:` incluem diff em paths `**/*test*` ou `**/*spec*`.

**P-004 — Testes devem ser determinísticos** (severity: info)
**Regra:** Sem `sleep`-based waits, sem comparações de relógio real, sem chamadas a serviços live em unit tests. Mockar em boundaries.
**Why:** Testes flaky treinam o time a ignorar falhas. A próxima falha real passa despercebida.
**Enforcement:** `dw-code-review` greppa testes por `setTimeout`, chamadas reais de fetch/axios, e `Date.now()` sem `vi.useFakeTimers()`/`jest.useFakeTimers()`.

---

## Consistência de UX

**P-005 — Strings user-facing vivem em fonte única** (severity: info)
**Regra:** Toda copy visível (labels, mensagens de erro, empty states) passa por um módulo centralizado de i18n / strings — não inline em componentes.
**Why:** Strings inline driftam no tom, quebram esforços de i18n e causam duplicatas da mesma mensagem em variações sutis.
**Enforcement:** `dw-code-review` flagueia text nodes JSX e mensagens de erro declarados dentro de componentes em vez de importados de `src/strings/`, `src/i18n/`, ou equivalente.

**P-006 — Estados de loading + empty + error são obrigatórios em qualquer UI que busca dados** (severity: info)
**Regra:** Componente ou página que faz fetch deve renderizar explicitamente loading, empty e error — não só o happy path.
**Why:** Experiências "só com spinner" e estados de erro silencioso são a #1 fonte de bugs reportados por usuários.
**Enforcement:** `dw-review-implementation` verifica componentes de data-fetching pelos três estados em JSX ou equivalente.

---

## Performance

**P-007 — Mudanças de performance carregam medição** (severity: info)
**Regra:** Qualquer commit que afirme melhorar performance deve incluir a métrica, a ferramenta e os números antes/depois no body do commit OU no techspec.
**Why:** Sem medição, "otimização" de performance é palpite — e geralmente errado (ver `dw-simplification` + `perf-discipline.md`).
**Enforcement:** `dw-code-review` verifica commits `perf:` por números antes/depois; flagueia se ausente.

**P-008 — Queries N+1 são flagueadas em code review** (severity: info)
**Regra:** Loops ou operações em lista que disparam chamadas DB/HTTP por item devem batchar (ex: `IN (...)`, `findMany`, DataLoader) ou ser explicitamente justificadas.
**Why:** Padrões N+1 escalam linearmente com tamanho dos dados e silenciosamente degradam até que a carga de produção revele.
**Enforcement:** `/dw-review` e `/dw-refactor` detectam padrões await-em-loop contra módulos de repository / API client.

---

## Segurança

**P-009 — Authorization server-side em todo endpoint que altera estado** (severity: info)
**Regra:** Endpoint que cria, atualiza ou deleta dado deve verificar autorização do caller no servidor. Gating em UI (botões escondidos, formulários disabled) não é segurança.
**Why:** Browsers são untrusted (ver `dw-testing-discipline/references/security-boundary.md`). Gating em UI é conveniência; só checks server-side protegem dado.
**Enforcement:** `dw-review` e o Security Gate `dw-secure-audit` exigem check de auth explícito (decorator, middleware ou assertion in-handler) em rotas POST/PUT/PATCH/DELETE.

**P-010 — Secrets nunca entram no repositório** (severity: high)
**Regra:** Nenhuma API key, password, signing key, token ou endpoint de produção commitado em source. `.env.example` documenta forma apenas.
**Why:** Histórico de repositório é permanente. Um secret commitado uma vez está vazado mesmo que revertido no commit seguinte.
**Enforcement:** o Security Gate `dw-secure-audit` roda gitleaks + Trivy no diff. Qualquer hit bloqueia — **sem exceção de ADR** (rotacione, não justifique).

**P-011 — Nenhum finding SAST high-severity passa sem revisão** (severity: high)
**Regra:** Código gerado/alterado deve passar na camada SAST (Semgrep) do Security Gate. Um finding high-severity alcançável (SQLi, command/code injection, SSRF, deserialization insegura, authn bypass) bloqueia salvo ADR justificando aceitação.
**Why:** Código gerado por IA reproduz padrões inseguros em escala; SAST determinístico no diff pega o que o review por leitura perde.
**Enforcement:** `dw-secure-audit` roda Semgrep no diff (`--baseline-commit`); findings sobrevivem a uma validação fp-check de reachability antes de bloquear.

---

## Princípios Customizados

> Adicione princípios específicos do seu time abaixo. Mesmo formato: `**P-NNN — <nome>** (severity: info|high|critical): <regra>. **Why:** <motivo>. **Enforcement:** <como>.`

<!-- Exemplo:
**P-100 — Todo cálculo financeiro usa Decimal, nunca Number** (severity: critical)
**Regra:** Valores monetários devem usar tipos `Decimal` / `BigDecimal` end-to-end. Sem `parseFloat`, sem aritmética com `Number`.
**Why:** Erros de arredondamento IEEE 754 acumulam centavos perdidos em milhões de transações; ambientes auditados exigem aritmética exata.
**Enforcement:** `dw-code-review` greppa por `Number(`/`parseFloat(` em qualquer arquivo sob `src/billing/`, `src/payments/`, `src/finance/`.
-->

---

## Como evoluir este arquivo

1. **Viva em `info` por pelo menos um release-ciclo.** Observe quão frequentemente cada princípio é violado organicamente; o dado te diz se vale promover.
2. **Promova para `high` quando violações forem raras e o time concordar.** PRs que violarem princípio `high` agora precisam de ADR.
3. **Promova para `critical` os princípios que protegem usuários / dados / compliance.** Trate-os como load-bearing; o escape via ADR exige aprovação de reviewer, não só opt-out do autor.
4. **Demote ou remova princípios que não ganharam seu peso.** Constitution é ferramenta, não museu.
5. **Re-rode `/dw-analyze-project`** quando o codebase mudar substancialmente (nova stack, refactor grande); ele pode propor updates fundamentados em observação fresca.

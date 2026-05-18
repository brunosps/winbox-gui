---
schema_version: "1.0"
generated_by: dev-workflow
last_updated: YYYY-MM-DD
mode: defaults | custom
---

# Project Constitution

> Declarative principles this team has chosen to follow. PRDs, TechSpecs, and Code Reviews read this file as a hard gate. Anything that violates a principle with `severity: critical` or `high` is blocked unless explicitly justified by an ADR.

## How this file works

- **Each principle has an ID (`P-NNN`), a severity, a rule, a `Why`, and an `Enforcement`.**
- **Severity ladder:** `info` (reports only, never blocks) → `high` (blocks PR without ADR) → `critical` (blocks PR without ADR, requires reviewer sign-off).
- **Edit freely.** This file is yours to evolve. Promote principles from `info` to `high` once you trust the project enforces them.
- **ADR escape hatch.** A PR that violates a `high`/`critical` principle is unblocked only when an ADR in the same feature documents the deviation and trade-off.
- **Regenerate analytical version** anytime via `/dw-analyze-project` (offers to synthesize principles from observed code patterns).

---

## Code Quality

**P-001 — No `any` / `unknown` casts in TypeScript without justification** (severity: info)
**Rule:** Production code must not use `as any`, `as unknown`, or `// @ts-ignore` without an inline `// @ts-ignore — reason: <X>` comment naming the constraint.
**Why:** Silent type escapes leak runtime bugs that the type system was meant to catch. Each escape is a contract the type system stops enforcing.
**Enforcement:** `dw-code-review` greps the diff for `as any`/`@ts-ignore`/`@ts-expect-error` without an accompanying reason comment.

**P-002 — Functions must be testable in isolation** (severity: info)
**Rule:** A function that touches network, filesystem, database, or system clock must accept its dependency as a parameter (or via a factory) instead of importing it directly.
**Why:** Code that constructs its own dependencies cannot be tested without integration setup. Tests slow down, get skipped, and bugs ship.
**Enforcement:** `dw-code-review` flags functions importing `fs`, `axios`, `prisma`, `Date.now`, etc., directly inside business logic modules.

---

## Testing Standards

**P-003 — Every bug fix ships with a regression test** (severity: info)
**Rule:** A commit with type `fix:` must add or modify at least one test that would have caught the bug before the fix.
**Why:** Without the test, the bug returns the next time someone refactors the area. The fix decays.
**Enforcement:** `dw-code-review` checks that `fix:` commits include diff under `**/*test*` or `**/*spec*` paths.

**P-004 — Tests must be deterministic** (severity: info)
**Rule:** No sleep-based waits, no real-clock comparisons, no network calls to live services in unit tests. Mock at boundaries.
**Why:** Flaky tests train the team to ignore failures. The next real failure goes unnoticed.
**Enforcement:** `dw-code-review` greps tests for `setTimeout`, real fetch/axios calls, and `Date.now()` without `vi.useFakeTimers()`/`jest.useFakeTimers()`.

---

## UX Consistency

**P-005 — User-facing strings live in a single source** (severity: info)
**Rule:** All visible copy (labels, error messages, empty states) goes through a centralized i18n / strings module — not inline in components.
**Why:** Inline strings drift in tone, break i18n efforts, and cause duplicate-but-different copies of the same message.
**Enforcement:** `dw-code-review` flags JSX text nodes and error messages declared inside components instead of imported from `src/strings/`, `src/i18n/`, or equivalent.

**P-006 — Loading + empty + error states are mandatory for any data-fetching UI** (severity: info)
**Rule:** Any component or page that fetches data must explicitly render loading, empty, and error states — not just the happy path.
**Why:** "Just the spinner" experiences and silent error states are the #1 source of user-reported bugs.
**Enforcement:** `dw-review-implementation` checks data-fetching components for all three states in JSX or equivalent.

---

## Performance

**P-007 — Performance changes must cite a measurement** (severity: info)
**Rule:** Any commit that claims to improve performance must include the metric, the tool, and the before/after numbers in the commit body OR in the techspec.
**Why:** Without measurement, "performance" optimization is guesswork — and usually wrong (see `dw-simplification` + `perf-discipline.md`).
**Enforcement:** `dw-code-review` checks `perf:` commits for a numeric before/after; flags if missing.

**P-008 — N+1 queries are flagged at code review** (severity: info)
**Rule:** Loops or list operations that issue per-item database/HTTP calls must batch (e.g., `IN (...)`, `findMany`, DataLoader) or be explicitly justified.
**Why:** N+1 patterns scale linearly with data size and silently degrade until production load reveals them.
**Enforcement:** `dw-code-review` and `dw-refactoring-analysis` detect await-in-loop patterns against repository / API client modules.

---

## Security

**P-009 — Server-side authorization on every state-changing endpoint** (severity: info)
**Rule:** Any endpoint that creates, updates, or deletes data must verify caller authorization on the server. UI-level gating (hidden buttons, disabled forms) is not security.
**Why:** Browsers are untrusted (see `dw-testing-discipline/references/security-boundary.md`). UI gating is convenience; only server checks protect data.
**Enforcement:** `dw-code-review` and `dw-security-check` require an explicit auth check (decorator, middleware, or in-handler assertion) on POST/PUT/PATCH/DELETE routes.

**P-010 — Secrets never enter the repository** (severity: info)
**Rule:** No API keys, passwords, signing keys, tokens, or production endpoints checked into source. `.env.example` documents shape only.
**Why:** Repository history is permanent. A secret committed once is leaked even if reverted next commit.
**Enforcement:** `dw-security-check` runs Trivy + secret scanners on the diff.

---

## Custom Principles

> Add your team's specific principles below. Follow the same format: `**P-NNN — <name>** (severity: info|high|critical): <rule>. **Why:** <reason>. **Enforcement:** <how>.`

<!-- Example:
**P-100 — All financial calculations use Decimal, never Number** (severity: critical)
**Rule:** Money values must use `Decimal` / `BigDecimal` types end-to-end. No `parseFloat`, no `Number` arithmetic.
**Why:** IEEE 754 rounding errors accumulate to cents lost over millions of transactions; audited environments mandate exact arithmetic.
**Enforcement:** `dw-code-review` greps for `Number(`/`parseFloat(` in any file under `src/billing/`, `src/payments/`, `src/finance/`.
-->

---

## How to evolve this file

1. **Live in `info` for at least one release.** Watch how often each principle is violated organically; the data tells you if it's worth promoting.
2. **Promote to `high` once violations are rare and the team agrees.** PRs that violate a `high` principle now need an ADR.
3. **Promote to `critical` for principles that protect users / data / compliance.** Treat these as load-bearing; the ADR escape requires reviewer sign-off, not just author opt-out.
4. **Demote or remove principles that don't earn their weight.** A constitution is a tool, not a museum.
5. **Re-run `/dw-analyze-project`** when the codebase shifts substantially (new stack, major refactor); it can propose updates grounded in fresh observation.

---
name: dw-review-rigor
description: |
  Applies review discipline to code-review and analysis commands:
  de-duplicates issues, orders by severity, verifies intent before flagging,
  prioritizes signal over volume, and skips concerns that linters already
  catch. Invoked from dw-code-review, dw-review-implementation, and
  dw-refactoring-analysis. Do not use for fetching reviews from external
  providers or for executing fixes.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
---

# dw-review-rigor — Discipline For Review Commands

A set of rules the caller applies while producing a review report. This skill does not produce its own report file — it shapes the caller's output.

## When Invoked

By `/dw-code-review`, `/dw-review-implementation`, `/dw-refactoring-analysis`. The caller has already identified a scope (files, a PR, a codebase area). This skill governs how findings are selected, deduplicated, ordered, and phrased.

## Required Inputs

- The scope the caller is reviewing (file paths, directories, or PR diff).
- Optional: prior review reports in `.dw/spec/prd-*/reviews/` — so this round only surfaces NEW findings.

## The Five Rules

### 1. De-duplicate before writing

If the same pattern (e.g., missing null check, unhandled error, magic constant) appears in multiple files, **create one finding** for the most representative instance and list the other affected files inside its body. Do not create N identical findings for N files sharing one root cause.

Example — wrong:
```
- [HIGH] src/a.ts:14 — missing null check on user
- [HIGH] src/b.ts:22 — missing null check on user
- [HIGH] src/c.ts:31 — missing null check on user
```

Example — right:
```
- [HIGH] Missing null check on `user` in 3 places.
  Representative: src/a.ts:14
  Also affects: src/b.ts:22, src/c.ts:31
  Fix: add `if (!user) return` at function entry.
```

### 2. Severity-order the output

Always present findings in this order: **critical → high → medium → low**. Inside each severity, order by impact or blast radius, not by file path.

Severity definitions:
- **critical** — correctness bug, security hole, data loss, unavailability.
- **high** — material deviation from PRD/TechSpec, concurrency hazard, significant perf regression, missing error handling on a user path.
- **medium** — maintainability cost that will hurt the next change, missing edge-case handling, inconsistent with project rules.
- **low** — stylistic, naming, minor readability. Often omit unless pattern-level.

### 3. Verify intent before flagging

Before creating a finding, check whether the pattern is intentional:
- adjacent comment explaining the choice?
- ADR in `.dw/spec/*/adrs/` that justifies it?
- test coverage that asserts the behavior?
- rule in `.dw/rules/` that permits it?

If the code looks suspicious but has a clear justification (e.g., `// intentionally ignoring close error on read-only handle`), do NOT create a finding. Only flag patterns that are genuinely problematic, not merely unconventional.

### 4. Skip what linters already catch

Before writing findings, ensure the project's linter/formatter has run. Anything the configured linter would flag is NOT a finding in this review — it is a linter task. Save human attention for issues linters cannot find (logic, architecture, security, requirements).

If the linter cannot run (missing tooling, build errors), note that in the summary and proceed with the review.

### 5. Signal over volume

Aim for fewer, higher-quality findings.

- Keep ALL `critical` and `high`.
- If total findings exceed 20, prune `medium` and `low` to the most impactful ones.
- A review with 8 precise findings is more useful than one with 30 that includes marginal concerns.

**Also note well-implemented aspects.** They inform the summary and calibrate tone — but they do not produce findings.

## Prior-Round Awareness

If the PRD directory has prior review reports:

1. Read them and extract the list of known findings (their titles + file/line signatures).
2. The current round surfaces **only NEW findings**. Do not re-flag items already tracked as pending, resolved, or accepted in earlier rounds.
3. If a prior finding was resolved incorrectly, open it as a NEW finding with "Regression of <prior ref>" in the body.

## Finding Format

Each finding uses:

```
[<severity>] <file.ext>:<line> — <title ≤72 chars>

<1-4 lines describing the problem>
<1-2 lines suggesting the fix>

Also affects: <other paths if de-duplicated, else omit>
Evidence: <relevant code snippet, test output, or reference>
```

## Output Structure

The caller emits:

1. **Merge/ship recommendation** — one of:
   - `Needs fixes before merge` (if any critical or high exist), with blocking findings named.
   - `Safe to merge with follow-ups` (only medium/low).
   - `Clean — ready to merge` (no findings).
2. **Counts** — critical / high / medium / low.
3. **Findings** — ordered by severity, each in the format above.
4. **Well-implemented aspects** — short bulleted list, calibrates tone.

## Critical Rules

- Do not modify source code — this skill shapes findings only, it does not fix.
- Do not create findings for problems a configured linter already catches.
- Do not flag patterns that have a clear adjacent justification or ADR.
- Do not write N identical findings for one root cause — de-duplicate.
- Do not mix severities — order is critical → high → medium → low.

## Integration With Other dev-workflow Commands

- `/dw-code-review` — applies all five rules to its Level-3 review output; uses prior reports in `.dw/spec/*/reviews/` to dedupe across rounds.
- `/dw-review-implementation` — applies de-dup + severity-ordering when listing gaps between PRD requirements and code.
- `/dw-refactoring-analysis` — applies rules 1, 2, 4, 5 when cataloging code smells (rule 3 adapts: a "smell" with a justifying ADR becomes a `low` finding at most).

Callers should mention this skill in their "Skills Complementares" section.

## Inspired by

Ported from Compozy's `cy-review-round` skill (`/tmp/compozy/.agents/skills/cy-review-round/SKILL.md`). Adapted for dev-workflow:

- No `reviews-NNN/` directory convention — dev-workflow reviews already persist in `.dw/spec/*/reviews/` per command's existing contract.
- The five rules are extracted here so three different dev-workflow review commands can share the discipline without duplicating it.
- No issue-file frontmatter (Compozy uses it to interoperate with its remediation engine; dev-workflow's remediation is manual or via `/dw-fix-qa`).

Credit: Compozy project (https://github.com/compozy/compozy).

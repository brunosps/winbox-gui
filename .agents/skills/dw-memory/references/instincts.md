# Instincts — promoting durable memory into reusable behavior

An instinct is a small, atomic, confidence-weighted behavior learned from THIS project's work, promotable
into a constitution principle or rule. Instincts live in `.dw/memory/instincts/*.md` and are proposed and
curated by `/dw-learn` — never written without explicit approval. There is no always-on observer; instincts
are synthesized on demand from what the project already recorded (see `/dw-learn` sources).

## File format (`.dw/memory/instincts/<slug>.md`)

```markdown
---
id: prefer-result-type-in-services
trigger: when writing a service method that can fail
confidence: 0.7          # 0.3 tentative … 0.9 near-certain (same scale as durable decisions)
domain: error-handling   # code-style | testing | error-handling | git | workflow | security
scope: project           # project | global
evidence:
  - "MEMORY.md decision (seen in tasks 3,5)"
  - "bugfix 004-null-order — same root cause"
---

# Prefer Result type in services

## Action
Return a typed Result/union from fallible service methods instead of throwing across layers.

## Why
Two bugfixes traced to exceptions swallowed between layers; the confidence-tagged decision in MEMORY.md
already prefers this.
```

## Confidence (reuse the promotion signal)

Same scale and rule as durable decisions (see the **Confidence signal** section of this skill): ≥2
independent confirmations in recent work → ≥0.7. Lower it when a later task contradicts the instinct; drop
it when it stops being true.

## Promotion path

- **≥0.7, project scope** → propose as a `.dw/constitution.md` principle (via the `/dw-analyze-project`
  Step 8 flow) or a note on the relevant `.dw/rules-library/<stack>.md`.
- **Seen across multiple projects** → mark `scope: global` (a candidate for a shipped rule/skill).

High-confidence instincts (≥0.7) whose `trigger` matches the current work are loaded on demand by `/dw-run`
and `/dw-plan` — the same lazy pattern as `.dw/rules/concerns.md`. Never bulk-load the whole instinct set.

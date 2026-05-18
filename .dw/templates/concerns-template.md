---
schema_version: "1.0"
generated_by: dw-analyze-project (Step 9)
last_refreshed: ""
---

# Concerns — Risk Map

Risk map for this codebase. Not conventions ("how we do things" — that's `.dw/rules/`), not architecture ("how it's built" — that's `.dw/intel/arch.md`). This file answers a single question: **where is it dangerous to mess around?**

Loaded on-demand by `/dw-plan`, `/dw-run`, and `/dw-bugfix` when their target touches an entry below. Auto-installed by `/dw-analyze-project` Step 9; never blocks (absence = no flagged areas yet).

## Hot Spots

Files or modules with high churn, frequent bug reports, or repeated "I touched this and broke something" history. Mention them in PRDs that touch the same area; add an extra reviewer or extra test pass.

| Path | Why it's hot | First flagged | Last incident |
|------|--------------|---------------|---------------|
| _e.g. `src/auth/session.ts`_ | _3 token-handling fixes in 60d_ | _YYYY-MM-DD_ | _YYYY-MM-DD_ |

## Fragile Integrations

External systems (APIs, queues, vendors, legacy databases) that have a track record of silent failures, schema drift, rate-limit surprises, or undocumented behavior. New code touching them needs explicit retry/timeout/idempotency handling.

| Integration | Failure mode | Mitigation expected |
|-------------|--------------|---------------------|
| _e.g. legacy SAP export_ | _silent 200 OK with empty body when source is locked_ | _check body length; log and alert_ |

## Hostile Code

Specific functions, regexes, parsers, or algorithms that are hard to reason about — anyone touching them should fully understand them first (or rewrite, not patch). Common offenders: hand-rolled regex, ad-hoc string parsers, custom serializers, race-prone async, manual transaction code.

| Path / function | Why it's hostile | Owner / context |
|-----------------|------------------|-----------------|
| _e.g. `src/billing/parseInvoice.ts:parseLine`_ | _900-char regex with 12 alternatives, no comments_ | _Bruno wrote it in 2024; rewrite if it breaks_ |

## Known Bug History

Aggregated from `.dw/bugfixes/*/SUMMARY.md` by `/dw-intel --build`. Lists modules with ≥2 historical fixes. Read alongside Hot Spots when planning related work.

| Module | Bug count | Recent slugs |
|--------|-----------|--------------|
| _e.g. `src/payments/`_ | _4_ | _002-stripe-webhook-retry, 007-refund-rounding_ |

## Tech Debt — Acknowledged

Pieces of debt the team has agreed exist. Not to be cleaned up opportunistically without coordination — they may be load-bearing in ways that aren't obvious.

| Area | Debt description | Why it stays | Cleanup trigger |
|------|------------------|--------------|-----------------|
| _e.g. `src/legacy/userMapper.ts`_ | _Two parallel field-mapping codepaths_ | _Awaiting v3 API migration_ | _Q3 2026 after vendor cutover_ |

---

**How to maintain this file:**

- `/dw-analyze-project` rewrites this on each run. Hand-written entries between `<!-- preserved:start -->` and `<!-- preserved:end -->` markers are kept.
- When a bugfix surfaces a new dangerous area, add it manually under Hot Spots and let the next analyze rerun confirm it.
- Promote entries to `.dw/constitution.md` when they become non-negotiable rules ("never touch X without an ADR").

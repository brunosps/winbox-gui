# Blameless discipline — the principles behind postmortems

The postmortem template imposes structure. This reference imposes the discipline that makes the structure useful.

## Why blameless

Postmortems with blame produce three failure modes:
1. **Hidden mistakes** — people learn to omit information that might reflect badly on them.
2. **Compliance theater** — "lessons learned" becomes a ritual, not a tool.
3. **Recurring incidents** — the same kind of failure happens 6 months later because the underlying system never changed.

Blameless framing forces the conversation toward what's actually fixable: systems, processes, assumptions, tooling. Individual mistakes are the entry point; the question is always "why did the system make this mistake easy/likely?"

## The 5-whys protocol

Stack five "why" questions until you reach an assumption or design choice that's actually changeable:

> **Symptom:** payment endpoint returned 500 for 47 minutes.
> **Why?** The new deploy introduced a regression in the order serializer.
> **Why?** The serializer started reading a field that the upstream API stopped sending.
> **Why?** The upstream API changed its response format two weeks ago and we didn't know.
> **Why?** We don't have contract tests against the upstream API.
> **Why?** Contract tests were considered too expensive to set up; the trade-off was never revisited.

**Root cause:** absence of contract tests for upstream APIs (a deliberate-but-stale design decision), not "the developer didn't check the response format."

If you reach "operator error" at any why, you stopped too early. Operator error happens because a system permits it.

## Quality bar for action items

The single most common postmortem failure mode: action items written vaguely so they can be "done" without changing anything.

### Bad action items

- "Improve monitoring."
- "Add more tests."
- "Document the runbook."
- "Make the system more resilient."
- "Better communication during incidents."

These are wishes, not actions. They can't be tracked. They will not happen.

### Good action items

Every action item has THREE components: owner, due date, measurable outcome.

| Action | Owner | Due | Measurable outcome |
|--------|-------|-----|-------------------|
| Add Datadog SLO alert at p99 > 800ms with PagerDuty routing to the payments on-call schedule | @bruno | 2026-06-01 | Alert exists in Datadog UI and fires successfully in test |
| Add idempotency-key header to `POST /api/orders` with 24h deduplication window | @maria | 2026-05-25 | Two identical requests with same key return the same order ID; verified by integration test |
| Open ADR documenting decision to add circuit breaker on Stripe API calls | @bruno | 2026-05-19 | ADR exists in `.dw/spec/<prd>/adrs/`, status: Accepted |

**Test:** can a third party verify the action item is done without asking the owner? If yes, it's good. If no, rewrite.

## Cognitive analysis — beyond the trigger

Two questions to push past "the code was wrong":

### 1. Why didn't we catch this earlier?

This is about the blind spot. The bug existed; we didn't see it. What's the gap in our:
- Tests (unit, integration, contract, E2E)?
- Monitoring (metric, alert, dashboard)?
- Process (code review, deploy checks, canary)?
- Documentation (knew but forgot, never knew)?

Fix the blind spot, not just the bug.

### 2. What ELSE could be hiding behind this gap?

If contract tests against the upstream API are missing → the payment incident is one instance. What ELSE depends on that API in ways the missing tests would catch?

The action item is to add the contract tests, not "fix the payment serializer."

## Cross-incident learning

`/dw-analyze-project` reads `.dw/incidents/` on subsequent runs. Patterns to watch:

- **3+ incidents in the same module** (e.g., billing): structural problem. Open a design-review issue, not another action item.
- **3+ incidents with the same root-cause class** (e.g., contract drift, missing idempotency): a constitution principle is needed (`/dw-adr` + add to `.dw/constitution.md`).
- **Time clustering** (multiple incidents during the same week): possible stress on the team's review/deploy capacity — process issue, not technical.

These patterns are MORE valuable than individual postmortems. Single incidents tell you what broke; patterns tell you what's fragile.

## Common pitfalls (the four big ones)

### Pitfall 1: Skipping triage

**Symptom:** jumping straight to debugging without assessing severity and blast radius.
**Consequence:** wrong priority — might fix a low-impact bug while a high-impact issue festers.
**Fix:** always classify severity first. Two minutes of triage saves hours of misguided investigation.

### Pitfall 2: Blame culture

**Symptom:** postmortem focuses on "who did it" instead of "why did the system allow it?"
**Consequence:** people hide mistakes; incidents recur.
**Fix:** blameless framing. Focus on systemic fixes — better monitoring, safer deploys, guardrails, removed footguns.

### Pitfall 3: No action items

**Symptom:** postmortem written, filed, forgotten.
**Consequence:** same incident in 3 months.
**Fix:** every postmortem has concrete action items with owners, due dates, measurable outcomes (see "Quality bar" above). Track them in the same system as feature work.

### Pitfall 4: Communicating too late

**Symptom:** users discover the outage before the team acknowledges it.
**Consequence:** trust erosion + support ticket flood.
**Fix:** first communication within 15 min for SEV-1/2, even if it's "We're investigating." Status page updates every 30 min until resolved.

## When the discipline bends

- **Internal-only tools:** the communication discipline can be lighter (no public status page).
- **Compliance-driven postmortems** (SOC 2, HIPAA): may require additional fields or sign-off chains beyond the template. Add them as a project-specific extension.
- **Trivial near-misses** (caught in staging before production): consider a "lite" postmortem — 1 page with timeline + lesson + action items. Not full structure.

In all bend cases, document the deviation in the postmortem itself. "Skipped public communication because internal-only tool" is fine; just say it.

## Reference reading

The discipline above is distilled from:
- Google SRE Book — [Incident Response](https://sre.google/sre-book/managing-incidents/) and [Postmortem Culture](https://sre.google/sre-book/postmortem-culture/).
- Etsy — [Debriefing Facilitation Guide](https://github.com/etsy/DebriefingFacilitationGuide) (the original blameless postmortem playbook).
- PagerDuty — [Incident Response Documentation](https://response.pagerduty.com/).

The dev-workflow version adapts these to a single-team or small-org scale; large enterprises may need more layers.

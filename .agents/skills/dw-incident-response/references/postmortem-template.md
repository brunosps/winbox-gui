# Postmortem template — blameless, action-oriented

Use this template for the Phase 5 output (`05-postmortem.md`). Every field is mandatory unless explicitly marked optional.

The discipline behind the template lives in `blameless-discipline.md` — read both.

## Template

```markdown
# Postmortem: <Incident title>

**Date:** YYYY-MM-DD
**Duration:** Xh Ym (from detection to all-clear)
**Severity:** SEV-X
**Authors:** @name1, @name2
**Status:** Draft | Reviewed | Published

## Summary

[2-3 sentences. What happened? What was the user-visible impact? How was it resolved?
No blame, no causes yet — just the observable narrative.]

## Timeline

All times UTC. Aim for ±5 min granularity; tighter for SEV-1/2.

| Time (UTC) | Event |
|------------|-------|
| HH:MM | First alert fired (or first user report) |
| HH:MM | On-call acknowledged |
| HH:MM | War room opened / incident channel created |
| HH:MM | Initial triage complete; severity confirmed |
| HH:MM | Hypothesis: <X> |
| HH:MM | Hypothesis rejected — observed <Y> |
| HH:MM | Root cause identified |
| HH:MM | Fix applied: <commit SHA / deploy>
| HH:MM | Verified recovery (health checks green, error rate baseline)
| HH:MM | All-clear confirmed; communications sent |

## Root cause

[Technical explanation. Walk through the chain:
1. What was the proximate cause? (the immediate trigger)
2. What was the underlying assumption that broke?
3. Why didn't existing safeguards catch it?

No blame. No "X did Y wrong." Frame as: "the system allowed Z because A was assumed."]

## Impact

- **Users affected:** N (counting method: <how we estimated>)
- **Geography:** [if applicable]
- **Revenue impact:** $X estimated (calculation: <method>)
- **Error budget consumed:** X% of monthly SLO
- **Data impact:** [data lost / corrupted? recoverable?]

## Detection

- **How was it detected?** [alert / user report / scheduled check]
- **Time to detection (TTD):** X minutes
- **Was the detection signal adequate?** [yes/no — and what would improve it]

## What went well

Concrete things to keep:
- [item — specific, not "we worked as a team"]
- [item]
- [item]

## What went wrong

Process / tooling / assumptions that failed:
- [item — specific failure mode, no blame]
- [item]
- [item]

## Where we got lucky

[Optional. What worked by accident that we shouldn't rely on next time?
e.g., "The bug only fired during business hours because of <X>; could have been overnight and worse."]

## Action items

Quality bar: each item has **owner**, **due date**, and **measurable outcome**. "Improve monitoring" does NOT count. Read `blameless-discipline.md` for the bar.

| Action | Owner | Due date | Priority | Tracking |
|--------|-------|----------|----------|----------|
| Add Datadog SLO alert at p99 > 800ms with PagerDuty routing | @bruno | 2026-06-01 | P1 | Linear ENG-1234 |
| Add idempotency key to /api/orders POST | @maria | 2026-05-25 | P1 | GitHub #5678 |
| Update runbook with new mitigation steps | @bruno | 2026-05-19 | P2 | Linear ENG-1235 |
| Constitution principle: every state-changing endpoint requires audit log | @team | 2026-06-15 | P3 | ADR-042 |

## Constitution implications

[Did this incident reveal a principle the team should commit to? If yes, an ADR
should be opened via `/dw-adr`. Examples:
- "Every write to billing tables requires audit log entry"
- "Database migrations must run in a separate maintenance window"
- "External API calls must have circuit breakers"

Link the ADR slug here.]

## Cross-incident pattern

[Have we seen this category of incident before? If yes, list the prior incidents.
3+ similar incidents = structural problem; needs design review, not just an action item.]

## References

- Incident channel: [Slack link]
- Diagnostic dashboards: [Grafana / Datadog links]
- Affected commits: [SHA range]
- Related ADRs: [list]
- Related runbook: `<path>` (updated in this incident if applicable)
```

## Quality checklist before publishing

- [ ] Summary is 2-3 sentences — not a wall of text.
- [ ] Timeline events have specific times (not "around lunch time").
- [ ] Root cause section explains the ASSUMPTION, not just the trigger.
- [ ] Every action item has owner + due date + measurable outcome.
- [ ] Action items are filed in the actual tracker (Linear/Jira/GitHub) with links.
- [ ] No names appear in "what went wrong" — only systems, processes, and assumptions.
- [ ] Cross-incident pattern section reviewed against `.dw/incidents/` for prior occurrences.
- [ ] If a constitution principle emerged, ADR opened via `/dw-adr` and linked.

## Publishing

- Postmortems for SEV-1/2 are shared with stakeholders within 48 hours.
- Postmortems for SEV-3 are reviewed in the next team retro.
- Action items track in the project's normal tooling — not in the postmortem file.
- The postmortem file in `.dw/incidents/` is the canonical write-up; everything else (Slack threads, status pages) eventually points back here.

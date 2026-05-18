---
name: dw-incident-response
description: Use when production breaks or for postmortems. 5 phases (triage → investigation → resolution → comms → postmortem), runbook templates. Triggers on outage, SEV-1, SEV-2, on-call handoff.
allowed-tools:
  - Read
  - Write
  - Bash
---

# Incident Response

> **Inspired by** [`wilsto/claude-code-starter-kit/incident-response`](https://github.com/wilsto/claude-code-starter-kit) (MIT). Five-phase workflow structure and runbook templates adapted from that skill; specifics rewritten for dev-workflow's `.dw/` namespace and command surface.

> wilsto credits the original `wshobson/agents` plugin `incident-response` (v1.3.0). Attribution chain: wshobson → wilsto → dev-workflow.

## When to use

- A production incident is declared (SEV-1 through SEV-3).
- You need to write a postmortem after an incident.
- You're generating or updating a runbook for a service.
- You need an on-call handoff template.
- `/dw-bugfix` detects severity `critical` + production marker — auto-escalates here.

## Key concepts

### Severity classification

| Severity | Criteria | Response time | Example |
|----------|----------|---------------|---------|
| **SEV-1 (Critical)** | Service down, data loss, security breach | Immediate (page) | Payment system offline |
| **SEV-2 (Major)** | Significant degradation, partial outage | < 30 min | API latency 10× normal |
| **SEV-3 (Minor)** | Limited impact, workaround exists | < 4 hours | One endpoint returning 500s |
| **SEV-4 (Low)** | Cosmetic, non-urgent | Next business day | Dashboard chart broken |

See `references/severity-and-triage.md` for full criteria and triage commands per stack.

### Behavioral rules

1. **Execute phases in order** — never skip a phase.
2. **Write output files after each phase** — they are the record of truth for the next phase.
3. **STOP at checkpoints** — wait for user confirmation before proceeding.
4. **Halt on failure** — if a step fails, do not continue to the next phase.
5. **File-based context** — read previous phase outputs rather than relying on conversation memory.

## Entry questions

Before starting any phase:

1. **What's happening?** Describe the incident in 1–2 sentences. What's broken and what's the user impact?
2. **Severity?** SEV-1 / SEV-2 / SEV-3 / SEV-4 per the table above.
3. **Mode?**
   - **Full workflow** — all 5 phases (triage → postmortem).
   - **Postmortem only** — incident already resolved; skip to Phase 5.
   - **Runbook generation** — produce a runbook template for a service (no live incident).

## The five phases

Each phase writes to `.dw/incidents/<YYYY-MM-DD>-<slug>/`. Slug is auto-generated from incident title (kebab-case, ≤30 chars).

### Phase 1 — Detection & Triage

**Output:** `.dw/incidents/<date>-<slug>/01-triage.md`

Steps:
1. Classify severity using the table above.
2. Assess blast radius: which services, how many users affected, revenue impact if known.
3. Identify immediate mitigation: rollback, feature flag toggle, traffic redirect.

See `references/severity-and-triage.md` for diagnostic commands per stack (Kubernetes, Docker, generic HTTP).

**Checkpoint:** present triage summary. Wait for user confirmation before moving to investigation.

### Phase 2 — Investigation & Root Cause

**Output:** `.dw/incidents/<date>-<slug>/02-investigation.md`

Steps:
1. Build timeline: when did it start? What changed?
2. Correlate signals: metrics spike + deploy + error logs.
3. Hypothesis testing: one theory at a time; verify each before moving on.
4. Identify root cause: not the first symptom, but the underlying assumption that broke.

Common forensic tools:
- `git bisect` for regressions.
- Recent deploy log: `git log --oneline --since="24 hours ago"`.
- For live monitoring during investigation, `/dw-debug-protocol` flaky-investigation patterns apply.

**Checkpoint:** present root-cause hypothesis. Wait for user confirmation before applying fix.

### Phase 3 — Resolution & Recovery

**Output:** `.dw/incidents/<date>-<slug>/03-resolution.md`

Steps:
1. Apply fix: hotfix branch → fast PR (via `/dw-generate-pr`) → deploy.
2. Verify: health checks green, error rate back to baseline.
3. Monitor 30 minutes post-fix for SEV-1/2 to confirm stability.

**Checkpoint:** confirm full recovery before drafting communications.

### Phase 4 — Communication

**Output:** `.dw/incidents/<date>-<slug>/04-communication.md`

Two communications generated using the templates in `references/communication-templates.md`:
- **Initial notification** (sent during the incident; updated every 30 min for SEV-1/2).
- **Resolution notification** (sent when phase 3 confirms recovery).

### Phase 5 — Postmortem

**Output:** `.dw/incidents/<date>-<slug>/05-postmortem.md`

Generate a **blameless** postmortem using `references/postmortem-template.md`. Sections:
- Summary (2–3 sentences).
- Timeline (per-minute events from alert to all-clear).
- Root cause (technical, no blame).
- Impact (users affected, revenue, error-budget consumed).
- What went well / What went wrong.
- Action items (owner + due date + priority — see `references/blameless-discipline.md` for the quality bar).

**Quality bar for action items:** see `references/blameless-discipline.md`. "Improve monitoring" does NOT count. "Add Datadog SLO alert at p99 > 800ms with on-call routing by 2026-06-01, owner: @bruno" counts.

## Required reading by context

| Doing what | Read |
|------------|------|
| Live incident — triage | `references/severity-and-triage.md` |
| Writing the postmortem | `references/postmortem-template.md` + `references/blameless-discipline.md` |
| Drafting incident communications | `references/communication-templates.md` |
| Generating a runbook (no live incident) | `references/runbook-templates.md` |
| On-call handoff document | `references/runbook-templates.md` (handoff section) |

## Common pitfalls

Detailed in `references/blameless-discipline.md`:

1. **Skipping triage** — jumping to debug without assessing severity/blast-radius wastes the wrong hours.
2. **Blame culture** — postmortems focused on "who did it" hide mistakes and incidents recur.
3. **No action items** — postmortem filed, forgotten, same incident in 3 months.
4. **Communicating too late** — users discover the outage before the team acknowledges; trust erodes.

## Integration with dev-workflow commands

- `/dw-bugfix` with severity `critical` + production marker → offers to escalate here.
- `/dw-autopilot --incident "X"` → runs this workflow end-to-end for declared incidents.
- `/dw-analyze-project` reads `.dw/incidents/` on next execution to surface recurring failure patterns. 3+ incidents touching the same area → flag as "structural problem; needs design review" and propose constitution principles based on observed patterns.
- `/dw-generate-pr` is the fix-deployment path during Phase 3.
- `/dw-adr` is the right tool when the postmortem leads to a deliberate architectural change.

## Output directory layout

```
.dw/incidents/
├── 2026-05-12-checkout-payment-outage/
│   ├── 01-triage.md
│   ├── 02-investigation.md
│   ├── 03-resolution.md
│   ├── 04-communication.md
│   └── 05-postmortem.md
└── 2026-05-08-search-index-stale/
    └── 05-postmortem.md     # postmortem-only mode
```

Files are committed to the repo alongside code — incidents are part of the project history, not ephemeral chat.

## Why this skill exists

dev-workflow's existing surface is "build feature → ship." Nothing covered "production broke, what now?" Teams improvised postmortems, action items got lost, and the same bug recurred. This skill closes that loop: structured response in the moment, blameless reflection after, and cross-incident learning that feeds back into the project's constitution.

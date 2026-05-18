---
type: idea-onepager
schema_version: "1.0"
status: draft
date: YYYY-MM-DD
classification: improves | consolidates | new
---

# Idea: [Short, imperative title]

## Problem Statement

[Reframe the raw idea as a "How might we" sentence:
**How might we** [verb] **for** [user/segment] **so that** [outcome/measurable value]?

Focus on the problem, not the solution. Avoid jumping into "how to implement".]

## Product Context (mapped existing features)

[Inventory of product features relevant to this idea. **Product level, not code level.** List what the product already delivers today that relates to the idea.

Sources:
- PRDs in `.dw/spec/prd-*/prd.md` (features already delivered or in development)
- `.dw/rules/index.md` (product overview)
- `.dw/intel/` (queryable index — built by `/dw-intel --build`, queried via `/dw-intel`)

Format:]

- **[feature A name]** — `.dw/spec/prd-<slug>/prd.md` — status: live / in development
- **[feature B name]** — `.dw/rules/index.md#module-Y` — status: live
- **[feature C name]** — PRD in progress, see `tasks.md`

> If the product is greenfield (no PRDs or rules yet), write: "Feature Inventory: greenfield — no product artifacts yet. This is the first recorded idea."

## Classification & Rationale

**Type:** IMPROVES | CONSOLIDATES | NEW

[Pick ONE of the three and justify:]

- **If IMPROVES** — which existing feature is being improved and why improving is worth more than creating a separate feature. Cite the original PRD.
- **If CONSOLIDATES** — which features are being merged, the gain from unifying (more cohesive UX, less duplicate code, consolidated data). List the original PRDs that become "superseded" (or under review).
- **If NEW** — why the product needs this capability now, where it connects to existing features (even new features are rarely fully isolated), and which gap it fills.

## Recommended Direction

[The recommended approach, 1 paragraph, in **product language**:
- User journey (who does what, when, why)
- Value delivered
- Boundary (what this idea covers and what's explicitly out)

**DO NOT write technical architecture here** — that's the techspec's job.]

## MVP Scope

[The smallest version that delivers real value. Thought in **user stories**, not technical tasks.

- As a [persona], I can [action] so that [benefit]
- As a [persona], I can [action] so that [benefit]

Ideally 2-4 stories. If it's more than 5, it's probably not MVP.]

## Not Doing (explicit)

[Tempting items that landed OUT of scope — and why. Forces scope discipline:]

- **[tempting item 1]** — reason: [out of scope because...]
- **[tempting item 2]** — reason: [could become v2 if hypothesis X validates]

## Key Assumptions to Validate

[What must be true for this direction to work. Each assumption with a test — ideally **with a user**, not with code.]

- **[assumption 1]** — test: [interview 5 users in segment X / market research / low-fidelity prototype]
- **[assumption 2]** — test: [metric Y rises by Z% within 2 weeks of release]

## Open Questions

[Questions that don't yet have an answer and that the user (or stakeholder) must answer before the PRD:]

- [Question 1 affecting scope]
- [Question 2 affecting priority]

## Next Step

Pick ONE:

- **`/dw-plan prd`** using this one-pager as input — when the direction is clear but we need to detail user stories, acceptance criteria, and hand off to techspec
- **`/dw-run`** — when it's an IMPROVES so small that it fits in a single task (up to 3 files, no new endpoint/screen) — write a quick PRD first
- **Stop here** — if any "Open Question" is blocking, stop and resolve with the stakeholder before advancing

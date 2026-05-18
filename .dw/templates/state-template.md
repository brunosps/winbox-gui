---
schema_version: "1.0"
last_paused: ""
last_resumed: ""
---

# Session State

Cross-session working memory. Lightweight index of what is in flight, what was decided, what is parked. Updated by `/dw-pause` (consolidates) and read by `/dw-resume` (orients).

Unlike per-PRD `MEMORY.md` (workflow memory for one feature) or ADRs (durable architectural records), this file lives at the project level and survives across PRDs, branches, and sessions. Edit it freely between pauses.

## Open Loops

What is currently in flight — work started but not finished. Each entry: short label + path/target + next concrete action.

- _none_

## Decisions

Cross-cutting decisions that haven't been promoted to an ADR yet (because they don't warrant one, or because the formalization is deferred). Format: `YYYY-MM-DD — decision — context (1 line)`.

- _none_

## Blockers

What's preventing forward motion. External (waiting on someone), internal (knowledge gap), or technical (broken tooling). Each entry: short label + what's blocked + owner / unblock condition.

- _none_

## Todos

Small follow-ups that don't justify a full PRD or task file. One line each. Cleared as they get done or migrated to a PRD.

- _none_

## Deferred Ideas

Ideas you considered but parked. Capture so they aren't lost; revisit when scope changes. Each entry: idea + reason it was parked + revisit trigger (if known).

- _none_

## Lessons

Small lessons learned during recent work — patterns that worked, gotchas, "next time I'll …". Not architectural (those go to ADRs); operational.

- _none_

## Preferences

Conventions agreed during work that affect how the agent should behave going forward. Examples: "always run `pnpm typecheck` before commit", "prefer named exports over default exports for utils".

- _none_

## Notes

Free-form scratchpad. Optional.

- _none_

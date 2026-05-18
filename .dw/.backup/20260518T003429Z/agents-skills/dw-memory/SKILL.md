---
name: dw-memory
description: |
  Maintains workflow-scoped memory for dev-workflow PRD runs using
  .dw/spec/prd-<name>/MEMORY.md (shared) and .dw/spec/prd-<name>/tasks/<N>_memory.md
  (task-local). Use when a dev-workflow command needs to persist or recall
  durable cross-task context (decisions, constraints, risks) across task
  executions. Invoked from dw-run-task, dw-run-plan, dw-autopilot, and
  dw-resume. Do not use for PR review remediation, user preferences, or
  event-log summarization.
allowed-tools:
  - Read
  - Write
  - Edit
  - Grep
  - Glob
---

# dw-memory — Workflow Memory

Two-tier memory for a PRD workflow. Callers pass the PRD slug; this skill manages the files.

## Required Inputs (from caller)

- PRD slug (e.g., `prd-user-auth`) → resolves to `.dw/spec/<slug>/`.
- Current task number (1-based), e.g., `3` → task-local file is `.dw/spec/<slug>/tasks/3_memory.md`.
- Optional flag: `compact: true|false` indicating whether either file must be compacted before proceeding.

If the PRD directory or `tasks/` subdirectory does not exist, stop and report — do not guess paths.

## File Layout

```
.dw/spec/<prd-slug>/
  prd.md                          # PRD (authoritative, not memory)
  techspec.md                     # TechSpec (authoritative)
  tasks.md                        # task index (authoritative)
  MEMORY.md                       # shared workflow memory (this skill)
  tasks/
    1_task.md
    1_memory.md                   # task-local memory for task 1
    2_task.md
    2_memory.md
    ...
```

Create `MEMORY.md` and `<N>_memory.md` on first use with the template below. Never create any other memory files.

## Templates

### MEMORY.md (shared, cross-task)

```
# Workflow Memory — <PRD slug>

## Current State
- Last task completed: <N> — <one-line summary>
- Active task: <N+1>
- Branch: <branch-name>

## Durable Decisions
- <decision 1> — <one-line rationale>

## Cross-Task Constraints
- <constraint discovered during implementation that affects multiple tasks>

## Open Risks
- <risk> — <what would trigger it, what to watch for>

## Handoff Notes
- <what the next task or agent needs to know that is not in the PRD/TechSpec/code>
```

### <N>_memory.md (task-local)

```
# Task <N> Memory

## Objective Snapshot
<current understanding of the task objective in 1-3 lines>

## Files Touched
- path/to/file.ext — <why>

## Debug Notes
- <observation that was non-obvious to arrive at>

## Workarounds Applied (task-local only)
- <workaround> — <why justified here, not elsewhere>

## Next Step
<what to do next if interrupted>
```

## Workflow

### 1. Load before editing code
- Read `MEMORY.md` and the current task's `<N>_memory.md` **before** any code change.
- Treat these as mandatory context for the run, not optional notes.
- If the caller marks either file for compaction, apply the Compaction Rules (below) before continuing.

### 2. Keep memory current while the task runs
- Update `<N>_memory.md` whenever:
  - the objective understanding changes,
  - a non-obvious decision is made,
  - a learning appears that the next step needs,
  - an error reshapes the plan.
- Promote to `MEMORY.md` only facts that pass the Promotion Test.
- Keep operational details (files touched, debug steps, local workarounds) in `<N>_memory.md`.

### 3. Close out cleanly
- Update memory **before any completion claim, handoff, or commit** (this pairs with `dw-verify`).
- Record only facts that help the next task start faster and with fewer mistakes.
- If `MEMORY.md` has grown noisy or repetitive, compact it using the Compaction Rules.

## Critical Rules

- Do not invent history, decisions, or status.
- Do not copy large code blocks, stack traces, or full PRD/TechSpec sections into memory files.
- Do not duplicate facts that are obvious from the repository, `git diff`, task file, PRD, or TechSpec.
- Do not read unrelated task memory files unless `MEMORY.md` or the caller explicitly points to them.
- Shared memory is durable and cross-task. Task memory is local and operational.

## Promotion Decision Test

Before promoting an item from `<N>_memory.md` to `MEMORY.md`, ask:

1. Will another task need this to avoid a mistake or rediscovery?
2. Is this fact durable across multiple runs, not just the current execution?
3. Is this information NOT already obvious from the PRD, TechSpec, task files, or the repository itself?

All three must be "yes" to promote. If any is "no", the item stays in task memory.

### Belongs in shared memory
- A discovered constraint affecting multiple tasks ("the Stripe API rate-limits to 100 req/s — batch operations must respect this")
- A cross-cutting architectural decision made during implementation ("chose React Server Components for data fetching across the whole feature")
- An open risk future tasks must account for ("migration depends on schema v3 which is not yet deployed to staging")

### Stays in task memory
- Files touched during this task's implementation
- Debugging steps taken to resolve a task-specific error
- The current task's objective and acceptance criteria snapshot
- A workaround applied only to the current task's scope

## Compaction Rules

When flagged for compaction, apply inline:

1. If both files need compaction, compact `MEMORY.md` first, then `<N>_memory.md`. The shared file sets the cross-task context that the task file should not duplicate.
2. **Preserve:** current state, durable decisions, reusable learnings, open risks, handoff notes.
3. **Remove:** repetition, stale notes, long command transcripts, facts derivable from the repo/PRD/task files.
4. **Rewrite** retained items as short factual bullets. No narrative logs, no chronological play-by-play.
5. Keep the default section headings intact. Remove empty sections only if truly unused.

## Error Handling

- If any caller-provided memory path is missing, stop and report the mismatch instead of guessing another path.
- If memory conflicts with the repository, PRD, or task spec, trust the repo and docs — correct the memory.
- If compaction would remove active risks, decisions, or handoff context, keep those items and remove lower-value repetition first.

## Integration With Other dev-workflow Commands

- `/dw-run-task` — reads memory before coding; updates `<N>_memory.md` during; runs promotion test + updates `MEMORY.md` at the end.
- `/dw-run-plan` — runs promotion + compaction between tasks, so each task starts with clean shared state.
- `/dw-autopilot` — threads memory through every phase (brainstorm → PRD → techspec → tasks → execution).
- `/dw-resume` — reads `MEMORY.md` first to reconstitute cross-session context, then the active task's memory.

Callers should mention this skill in their "Skills Complementares" section.

## Inspired by

Ported from Compozy's `cy-workflow-memory` skill (`/tmp/compozy/.agents/skills/cy-workflow-memory/SKILL.md`). Adapted for dev-workflow:

- Paths are `.dw/spec/<prd-slug>/` instead of `.compozy/tasks/<name>/`.
- Task-local file is `<N>_memory.md` next to `<N>_task.md` (mirrors the existing dev-workflow task layout).
- Inline Compaction Rules (Compozy keeps them in `references/memory-guidelines.md`); if the rule set grows, extract a `references/` directory later.

Credit: Compozy project (https://github.com/compozy/compozy).

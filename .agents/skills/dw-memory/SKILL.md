---
name: dw-memory
description: Two-tier workflow memory (shared + per-task) with promotion test, so cross-task context survives without bloating files.
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
- <decision 1> — <one-line rationale> — [confidence: 0.7; seen in tasks 1,3]

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

### Confidence signal

Tag each durable decision with a confidence in `[0.3–0.9]` plus the tasks that confirmed it — `… — [confidence: 0.7; seen in tasks 1,3,5]`:

- Confirmed in **≥2 of the last 3 tasks** with no contradiction → **≥0.7** (trust it; safe to act on without re-deriving).
- Confirmed once, or inferred but not yet reused → **0.3–0.5** (tentative; gather more signal before relying).
- Contradicted by a later task or the repo → lower it or drop the decision (see Error Handling).

Confidence makes cross-task learning explicit and is the signal `/dw-learn` reads to promote high-confidence decisions into durable instincts, constitution principles, or rules.

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

## Context Budget

Memory is part of a broader **context budget** the agent must respect during execution. A blown budget degrades reasoning before any output is produced — the model starts missing requirements, drops constraints, and reverts to averaged-over-training answers. Full guidance: [`references/context-budget.md`](references/context-budget.md).

**Targets:**

- **Total active context:** under **40k tokens** (rough working budget across PRD + TechSpec + tasks + MEMORY.md + per-task memory + open files).
- **Reserve:** at least **120k tokens** of headroom for actual reasoning, tool output, and the model's response stream.
- **Hard ceiling per memory file:** `MEMORY.md` ≤ 6KB; `<N>_memory.md` ≤ 3KB. Past those, compact instead of growing.

**Anti-co-load rules** (apply on every load):

1. Never load two PRD specs in the same context. If switching PRDs, drop the previous PRD's spec/techspec/tasks references from the active set.
2. Never load multiple archived `.dw/bugfixes/` SUMMARY.md files together — load only what's needed for the active fix or query.
3. Never load `.dw/intel/files.json` and `.dw/intel/deps.json` simultaneously when answering a single question — pick the primary per query shape (see `dw-codebase-intel/references/query-patterns.md`).
4. Never load a design proposal AND the prior design's full text — if comparing, summarize the prior into 5-10 lines.

**Monitoring signal:**

If the agent finds itself reading large files repeatedly or summarizing the same fact across multiple turns, that's a budget signal. Compact memory and explicitly drop unrelated loaded context before proceeding. Note this in `MEMORY.md` under Handoff Notes so the next task starts lean.

This budget is doctrine, not a hard gate. No command currently rejects work for exceeding 40k. The discipline lives here because future sessions read this skill first.

## Error Handling

- If any caller-provided memory path is missing, stop and report the mismatch instead of guessing another path.
- If memory conflicts with the repository, PRD, or task spec, trust the repo and docs — correct the memory.
- If compaction would remove active risks, decisions, or handoff context, keep those items and remove lower-value repetition first.

## Integration With Other dev-workflow Commands

- `/dw-run` — reads memory before coding; updates `<N>_memory.md` during; runs promotion test + updates `MEMORY.md` at the end.
- `/dw-run` — runs promotion + compaction between tasks, so each task starts with clean shared state.
- `/dw-autopilot` — threads memory through every phase (brainstorm → PRD → techspec → tasks → execution); on re-invocation reads `MEMORY.md` first to reconstitute cross-session context.
- `/dw-learn` — synthesizes confidence-tagged decisions here (plus bugfixes, deviations, git history) into atomic **instincts** in `.dw/memory/instincts/`; format, confidence rules, and promotion path in `references/instincts.md`.

Callers should mention this skill in their "Skills Complementares" section.

## Inspired by

Ported from Compozy's `cy-workflow-memory` skill (`/tmp/compozy/.agents/skills/cy-workflow-memory/SKILL.md`). Adapted for dev-workflow:

- Paths are `.dw/spec/<prd-slug>/` instead of `.compozy/tasks/<name>/`.
- Task-local file is `<N>_memory.md` next to `<N>_task.md` (mirrors the existing dev-workflow task layout).
- Inline Compaction Rules (Compozy keeps them in `references/memory-guidelines.md`); the budget discipline was extracted to `references/context-budget.md` because it's about model behavior, not file management.

Credit: Compozy project (https://github.com/compozy/compozy).

The Context Budget section adapts the context-loading discipline from [`tech-leads-club/agent-skills/tlc-spec-driven`](https://github.com/tech-leads-club/agent-skills/tree/main/packages/skills-catalog/skills/(development)/tlc-spec-driven) (CC-BY-4.0, Felipe Rodrigues). The target, the anti-co-load rules, and the "monitoring signal" framing come from there; the integration with two-tier memory and the specific file ceilings are dev-workflow-specific.

## Structured Return

When invoked directly or by a harness, return or merge this block:

- **Status:** `PASS` when memory is current and scoped correctly, `FINDINGS` when memory is stale/overloaded, `BLOCKED` when required task/project context is missing, `NOT_APPLICABLE` when no session memory is needed.
- **Scope:** project memory, task memory, checkpoint, and files considered.
- **Evidence:** memory entries read/written, stale facts removed, and promotion decision.
- **Artifacts:** `.dw/STATE.md`, task `MEMORY.md`, checkpoint, or compaction note.
- **Decisions:** promote, keep local, discard, or compact.
- **Risks:** stale instructions, duplicated facts, leaked task-local context, or oversized memory.
- **Next Step:** exact memory write, compaction, or checkpoint action.

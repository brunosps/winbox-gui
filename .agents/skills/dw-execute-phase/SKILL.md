---
name: dw-execute-phase
description: "Use for task execution. Two agents: executor (wave-based parallel dispatch + deviation handling) and plan-checker (goal-backward verification). Invoked by /dw-run and /dw-autopilot."
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - Grep
  - Glob
---

# dw-execute-phase

Bundled skill providing **phase-level execution discipline** for dev-workflow: parallel task dispatch in waves, atomic commit per task, deviation handling mid-execution, and goal-backward plan verification before any execution burns context.

## Why a skill (not inline)

- The execution discipline (wave coordination, deviation rules, checkpoint protocol) is a separate concern from the commands that invoke it. Bundling it as a skill lets multiple commands (`/dw-run`, `/dw-autopilot`, `/dw-execute-phase` itself) reuse the same discipline.
- The plan-checker is a verification GATE — it must run before `/dw-run`/`/dw-execute-phase` mutate code, and bundling it makes that contract visible.
- The agents own the protocol; the orchestrating commands just wire them up.

## When to Use

Read this skill when:

- `/dw-execute-phase` is invoked to run a batch of tasks in parallel waves.
- `/dw-plan-checker` is invoked to verify a `tasks.md` file will achieve its PRD goal before execution.
- `/dw-run` is invoked (it spawns the executor agent for each wave).
- `/dw-autopilot` enters the execution stage (it gates on plan-checker before invoking the executor).

Do NOT use when:

- A single one-off change is being made (use `/dw-run` directly — no waves needed).
- The user is exploring/brainstorming, not executing (use `/dw-brainstorm`).
- The plan hasn't been created yet (use `/dw-plan tasks` first).

## Agents

| Agent | Responsibility | Spawn from |
|-------|----------------|------------|
| `agents/executor.md` | Runs tasks in waves, atomic commit per task, handles deviations (3 deviation rules), respects checkpoint markers, writes `SUMMARY.md` per phase | `/dw-execute-phase`, `/dw-run` |
| `agents/plan-checker.md` | Goal-backward verification of `tasks.md` before execution. Checks: requirement coverage, task completeness, dependency soundness, artifact wiring, context budget. Returns PASS / REVISE / BLOCK. | `/dw-plan-checker`, `/dw-plan tasks` (auto-gate before declaring tasks ready) |

## How the Two Agents Compose

The expected flow:

1. `/dw-plan tasks` produces `.dw/spec/prd-<slug>/tasks.md` from PRD + TechSpec.
2. **Plan-checker GATE** — `/dw-plan-checker .dw/spec/prd-<slug>/` spawns the plan-checker agent. The agent reads PRD/TechSpec/tasks.md and verifies tasks WILL achieve the goal. Returns one of: `PASS` (proceed), `REVISE` (issues found, planner re-runs), `BLOCK` (fundamental gap, abort).
3. `/dw-execute-phase` spawns the executor agent ONLY if plan-checker returned `PASS`. The executor runs tasks in waves, commits atomically, handles deviations.
4. `/dw-qa` runs after all waves complete to validate the implementation against PRD.

`/dw-autopilot` orchestrates this entire flow with hard gates between stages.

## Wave Concept

Tasks in `tasks.md` are grouped into **waves** by their `Depends on:` frontmatter:

- Wave 1: tasks with no dependencies → run in parallel
- Wave 2: tasks that depend on Wave 1 → run after Wave 1 commits land
- Wave N: ...

Within a wave, tasks run in parallel via subagent dispatch. Across waves, sequential.

The executor calculates waves automatically by topologically sorting task dependencies. See `references/wave-coordination.md`.

## Deviation Rules (during execution)

Mid-task, the executor may discover the plan is incomplete or contradicted by reality. Three rules:

1. **Auto-add missing critical functionality** — if a task says "create endpoint" but no validation is specified and the project's CLAUDE.md/rules require it, add the validation as part of the same task. Note in the task's commit message.
2. **Surface ambiguity, don't guess** — if the plan says "use the existing service" and 3 services match, STOP, write a deviation entry to `.dw/spec/prd-<slug>/deviations.md`, and ask the user.
3. **Block on architectural conflicts** — if the task as planned would violate a locked decision in CONTEXT.md / project rules, abort the task and surface the conflict for re-planning.

Detail in `references/atomic-commits.md` (deviation entry format) and `references/plan-verification.md` (what plan-checker should catch BEFORE execution to prevent deviations).

## Atomic Commit Protocol

Every task ends with exactly one git commit:

```
feat(<scope>): <task title> (RF-XX)

<one-line summary of what this commit delivers>

- Files added: <list>
- Files modified: <list>
- Tests added/updated: <list>
- Deviations: <link to deviations.md entry, if any>

Closes RF-XX (partial — see tasks.md).
```

The commit message format is consistent across waves so `/dw-generate-pr` can build a clean PR body. See `references/atomic-commits.md`.

## Checkpoint Protocol

If the executor exhausts its context budget mid-phase OR the user signals stop:

- Write current progress to `.dw/spec/prd-<slug>/active-session.md` (last completed task, next task, blockers).
- Exit cleanly with status `CHECKPOINT`.
- Next invocation of `/dw-resume` reads this file and resumes from the last completed task.

## Files in `.dw/spec/prd-<slug>/`

| File | Read by | Written by |
|------|---------|------------|
| `prd.md` | plan-checker, executor | `/dw-plan prd` |
| `techspec.md` | plan-checker, executor | `/dw-plan techspec` |
| `tasks.md` | plan-checker (verifies), executor (executes) | `/dw-plan tasks` |
| `<NN>_task.md` | executor (per-task detail) | `/dw-plan tasks` |
| `deviations.md` | plan-checker (next iteration), executor | executor (rule 1/2 deviations) |
| `active-session.md` | `/dw-resume`, executor (continuation) | executor (checkpoint) |
| `SUMMARY.md` | `/dw-generate-pr` | executor (after final wave) |

## References

- `references/wave-coordination.md` — how the executor groups tasks into waves and dispatches them in parallel.
- `references/plan-verification.md` — the 6-dimension goal-backward analysis the plan-checker performs.
- `references/atomic-commits.md` — commit message format, deviation entry format, when to use Edit vs Write.

## Rules

- **No execution without plan-checker PASS.** `/dw-execute-phase` and `/dw-run` must call plan-checker first; if it returns REVISE or BLOCK, abort.
- **One commit per task, no exceptions.** Even trivial tasks commit. This drives traceability and revert safety.
- **Deviations are recorded, not silenced.** Every adjustment beyond the plan goes in `deviations.md` with reason.
- **Checkpoint > timeout.** When context budget is low, checkpoint cleanly rather than running tasks half-way.
- **Wave order is topological, not user-defined.** The executor computes wave boundaries from `Depends on:` fields; users can't override.

## Inspired by

Adapted from [`get-shit-done-cc`](https://github.com/gsd-build/get-shit-done) (`gsd-executor`, `gsd-plan-checker`) by gsd-build (MIT license). Core protocols (goal-backward verification, atomic commits, deviation handling, checkpoint resume) preserved. Path conventions changed from `.planning/<phase>/` to `.dw/spec/prd-<slug>/`. SDK CLI calls (`gsd-sdk query init.execute-phase`) replaced by inline operations. The companion `gsd-debugger` agent (1452 lines) was NOT ported — its scope overlaps with the existing `/dw-bugfix` and `/dw-qa --fix` commands.

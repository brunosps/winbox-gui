# Wave coordination — how the executor parallelizes tasks

Tasks within a phase have dependencies. Some can run in parallel; others must wait. The executor groups tasks into **waves** and runs each wave as parallel subagent dispatches, with sequential ordering between waves.

## Wave computation

Input: `tasks.md` with each task carrying a `Depends on:` field (none, or comma-separated task numbers).

Algorithm: topological sort.

1. Build dependency graph: node per task, edge from each `Depends on:` to the dependent task.
2. Cycle check: if a cycle exists → abort with `EXEC-FAILED: dependency cycle`.
3. Wave assignment:
   - Wave 1 = tasks with zero dependencies
   - Wave N = tasks whose all dependencies are in waves 1..N-1
4. Output: ordered list of waves, each wave a list of task numbers.

## Example

```
tasks.md:
- 01 Create user schema       Depends on: none
- 02 Create user model        Depends on: 01
- 03 Create order schema      Depends on: none
- 04 Wire auth middleware     Depends on: 02
- 05 Add login endpoint       Depends on: 04
- 06 Add order endpoint       Depends on: 02, 03
- 07 Wire validation rules    Depends on: 04, 06

Computed waves:
Wave 1: [01, 03]            (no deps; can run in parallel)
Wave 2: [02]                (depends on 01)
Wave 3: [04, 06]            (depend on wave 2)
Wave 4: [05, 07]            (depend on wave 3)
```

## Parallel execution within a wave

Within a wave, the executor dispatches tasks in parallel via subagent calls (one subagent per task). Each subagent:

1. Reads its `<NN>_task.md`
2. Implements
3. Verifies (lint/tests/build)
4. Commits atomically (per `atomic-commits.md`)
5. Marks `[x]` in `tasks.md`
6. Returns success or deviation status to the orchestrating executor

The orchestrating executor:

- Spawns N subagents in parallel (single message, N tool calls)
- Waits for all to return
- If all PASS → proceed to next wave
- If ANY return deviation/blocked → resolve before next wave (see deviation rules in `agents/executor.md`)

## Wave width limits

Practical caps to keep context budget sane:

- **Soft cap: 5 tasks per wave** — tested as the upper bound where parallel execution stays efficient
- **Hard cap: 8 tasks per wave** — beyond this, the orchestrator's context fills with subagent results faster than tasks complete

If a wave exceeds the hard cap, the planner should split: introduce a synthetic dependency to bisect the wave. Example: if Wave 3 has 10 independent tasks, force tasks 06-10 to depend on a "wave 3a barrier" so they go to Wave 3.5.

The plan-checker (Dimension 5) flags wide waves before execution.

## Cross-wave atomicity

Each wave is a checkpoint:

- After all tasks in a wave commit, run a `git status` check — should be clean (everything committed).
- If any task in a wave failed permanently (Rule 3 deviation), abort BEFORE starting the next wave. Leave the partial work committed; surface to user via `EXEC-BLOCKED`.
- Don't run waves N+1 with broken commits in wave N. The dependencies aren't satisfied.

## Order within a wave

Within a wave, order doesn't matter logically (no deps between same-wave tasks). But for **commit history readability**, the executor should commit in numeric order of task number (01 commit before 02 even if both are wave 1). The parallel subagent results may arrive out of order; the executor collects and commits sequentially.

This means: subagents return their changes (files written, but NOT committed). The executor commits them in numeric order.

(Alternative: subagents commit independently and accept commit interleaving. Pick this if commit-order doesn't matter for the project; the orchestrator sets a flag.)

## When to NOT use waves

Single-task changes (`/dw-quick`, `/dw-run`) bypass waves entirely. Waves are for `/dw-run` and `/dw-execute-phase` — phase-scale execution.

## Verification of wave structure (pre-execution)

The plan-checker (Dimension 3 in `plan-checker.md`) verifies:
- Topological sort succeeds (no cycles)
- All `Depends on:` references point to existing tasks
- No wave exceeds the hard cap (8 tasks)

If these fail, plan-checker returns REVISE before any code is touched.

## Resume after checkpoint

If the executor checkpoints mid-phase, `active-session.md` records:
- `last_completed_task`
- `last_wave_completed`
- `remaining_tasks`

`/dw-resume` reads this, recomputes waves from `tasks.md`, skips already-committed tasks, and resumes from the wave containing `last_completed_task + 1`.

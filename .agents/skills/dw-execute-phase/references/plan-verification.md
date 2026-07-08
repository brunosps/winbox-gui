# Plan verification — the 6-dimension goal-backward analysis

The `dw-plan-checker` agent verifies that a `tasks.md` will achieve the PRD goal BEFORE execution starts. This document details each dimension's checks, examples, and pass/fail criteria.

## Why pre-execution verification

Mid-execution discovery of plan flaws is expensive: context burned, partial commits to revert, deviation logs to resolve. Pre-execution verification catches the same flaws at zero implementation cost. The trade-off: 1-2 minutes of plan-checker time vs. potentially 30+ minutes of mid-execution rework.

## The 6 dimensions

### 1. Requirement Coverage

**Goal:** every PRD requirement (RF-XX) has at least one task addressing it.

**Steps:**
1. Extract numbered requirements from `prd.md`. Pattern: `### RF-NN` or `**RF-NN:**` or numbered list under "Functional Requirements".
2. Build `Set<RF>` of expected requirements.
3. Scan `tasks.md` and `<NN>_task.md` files for `Closes RF-XX` / `RF: RF-XX` / `Requirement: RF-XX` markers.
4. Compute `uncovered = expected - addressed`.

**Pass:** `uncovered` is empty.
**Revise:** uncovered non-empty AND fixable by adding tasks.
**Block:** PRD has no numbered requirements (`expected` is empty) — verifying intent is impossible.

**Common failures:**
- Planner forgot a non-obvious requirement (e.g., audit logging) buried in PRD prose
- Compound requirement ("user can sign up AND verify email") only addressed for one half
- Requirement covered indirectly but no task explicitly marks the closure

### 2. Task Completeness

**Goal:** each task has all four parts: files, action, verification, done criteria.

**Steps:**
For each task in `tasks.md`:
- Open `<NN>_task.md` (or inline section)
- Check for required sections:
  - `## Files` (or `Files to create/modify:`) — explicit paths
  - `## Action` (or `Implement:`) — what to do, concrete
  - `## Verification` (or `Verify:`) — how to know it worked (linter, tests, build, manual)
  - `## Done` (or `Done when:`) — criteria for marking `[x]`

**Pass:** all four sections present in every task.
**Revise:** ≥1 task missing one section.

**Common failures:**
- "Files: TBD" or "Files: see techspec" — too vague
- No verification → executor can't decide when to commit
- Done criteria absent → executor commits but can't mark `[x]` confidently

### 3. Dependency Soundness

**Goal:** `Depends on:` graph is acyclic, references valid, waves are reasonable.

**Steps:**
1. Parse every task's `Depends on:` field (none / comma-separated task numbers).
2. Build directed graph (node = task, edge = depends-on relation).
3. Topological sort:
   - Cycle → BLOCK with the cycle path
   - Reference to non-existent task number → REVISE with the dangling ref
4. Compute wave widths. Any wave > 8 tasks → REVISE (split with synthetic barrier).

**Pass:** topological sort succeeds, all refs valid, waves ≤ 8 wide.

**Common failures:**
- Bidirectional dependency (`02 depends on 03; 03 depends on 02`) — likely planner confusion; needs re-think
- Typo in `Depends on: 02` when the task is actually numbered `2.5` or `03`
- 10 independent test files all in wave 1 → split into smaller waves

### 4. Artifact Wiring

**Goal:** every artifact produced by a task is consumed by a downstream task OR is a leaf deliverable referenced in PRD's user stories.

**Steps:**
1. For each task, identify what it PRODUCES:
   - New files (from `Files: + src/foo.ts`)
   - New exports (from `Implement: export function bar()`)
   - New endpoints (from `Action: add POST /api/baz`)
2. For each downstream task (later in topo order), identify what it CONSUMES:
   - Imports (from `Files: src/quux.ts (modify) — imports bar`)
   - References (from `Action: call /api/baz`)
3. Cross-reference: every produced artifact should be either consumed downstream OR explicitly mentioned in PRD as a user-facing deliverable.

**Pass:** zero orphan artifacts.
**Revise:** ≥1 artifact created without consumer or PRD reference — likely incomplete plan.

**Common failures:**
- Task creates a service but no task wires it into the router
- Task creates a migration file but no task runs it (`prisma migrate deploy`)
- Task creates a config but no task reads it

### 5. Context Budget

**Goal:** the phase fits in a practical execution window.

**Steps:**
1. Count tasks. Practical limit: 12 tasks per phase before quality degrades.
2. Estimate aggregate file changes: sum of files mentioned in `Files:` across all tasks. Limit: 30 files per phase.
3. Check parallelism setup: are wave-1 tasks running in parallel? (frontmatter `parallel: true` or default).

**Pass:** ≤12 tasks, ≤30 aggregate files, wave 1 has parallel execution if multi-task.

**Revise:** > 12 tasks → suggest splitting into 2 phases. > 30 files → reduce scope or split.

**Common failures:**
- Mega-phase with 20 tasks because the planner couldn't decompose
- Single-task waves where parallelism was forgotten

### 6. Constraint Compliance

**Goal:** tasks honor locked decisions and project conventions.

**Steps:**
1. Read `.dw/rules/index.md` and `.dw/rules/<module>.md` for relevant modules.
2. Read `CONTEXT.md` if exists; extract `## Decisions` (LOCKED) section.
3. Read `./CLAUDE.md` for project hard constraints.
4. For each task, check if its `Action` or `Files` violate:
   - A locked decision
   - A project rule (forbidden pattern, mandated tool)
   - A CLAUDE.md directive

**Pass:** zero violations.
**Block:** ≥1 hard violation. The plan must change before execution.

**Common failures:**
- Locked decision says "framework: Fastify" but task says "use Express"
- Project rules forbid raw SQL; task uses `pg.query()` directly
- CLAUDE.md says "no env files committed"; task adds `.env.production`

## Verdict resolution

After running all 6 dimensions:

| Outcome | Verdict |
|---------|---------|
| All 6 PASS | PASS — proceed to execution |
| Any REVISE, no BLOCK | REVISE — re-plan |
| Any BLOCK | BLOCK — surface to user, no auto-replan |

`PASS` is the only state that allows `/dw-execute-phase` to proceed.

## Bounded revision loop

The plan-checker is part of a bounded quality loop:

1. `/dw-plan tasks` produces v1 of `tasks.md`
2. `/dw-plan-checker` runs → REVISE
3. `/dw-plan tasks --revise` produces v2 (consumes plan-checker's issues as input)
4. `/dw-plan-checker` runs → PASS or REVISE again
5. After 3 revisions without reaching PASS → escalate to user (something fundamental is wrong)

The escalation cap prevents infinite loops where the planner can't satisfy the verifier. At 3 strikes, the user sees the diff between PRD and the failing tasks.md and decides next step.

## Time budget

Plan-checker target: 1-2 minutes. The 6 dimensions are mostly file reads and grep operations; no heavy analysis. If plan-checker exceeds 5 minutes, it's a sign the agent is over-thinking — bias toward issuing REVISE for ambiguous cases rather than analyzing them deeply.

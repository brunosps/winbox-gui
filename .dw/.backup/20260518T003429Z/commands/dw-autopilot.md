<system_instructions>
You are a full pipeline orchestrator. This command receives a user's wish and automatically executes the entire development flow, from research to commit, stopping only at critical gates.

<critical>This command MUST execute ALL applicable pipeline steps. Do NOT skip any step. If a step is conditional, evaluate the condition and execute if applicable.</critical>
<critical>The ONLY pause moments are the 3 gates defined below. Between gates, execute everything automatically without asking for confirmation.</critical>
<critical>Each step MUST follow the complete instructions from the corresponding command in `.dw/commands/`. Read and execute the full command, not a summarized version.</critical>

<critical>FORMAL EXECUTION IS MANDATORY — READ BEFORE STARTING:
A step that invokes a `/dw-xxx` command is ONLY considered complete when the artifacts produced by that command exist on disk. Manual validations (running tests, opening Playwright ad-hoc, eyeballing the code, writing a short qa-report by hand) DO NOT replace formal execution of the command.
- BEFORE each step: announce to the user `→ Invoking /dw-[name] — executing full instructions`.
- DURING: follow the instructions in `.dw/commands/[name].md` in full, without summarizing or substituting.
- AFTER: run `ls` on the artifact paths listed in that step and confirm they exist before updating `autopilot-state.json`. If any artifact is missing, the step did NOT run — re-execute the command formally.</critical>

<critical>FORBIDDEN RATIONALIZATIONS — if you think any of these, STOP and execute the command formally:
| Thought | Reality |
|---------|---------|
| "I already ran the tests manually" | The command produces structured artifacts. Run the command. |
| "I validated via ad-hoc Playwright" | `/dw-run-qa` requires RF matrix, bugs.md, screenshots, scripts, logs, checklist. Run the command. |
| "The implementation is obviously correct" | `/dw-review-implementation` requires a compliance matrix per RF/endpoint/task. Run the command. |
| "A strong manual validation is enough" | NO. Technical equivalence DOES NOT replace formal execution. |
| "I already checked build and lint, that's enough" | Build/lint DO NOT replace review nor QA. Run the commands. |
| "I wrote a summarized qa-report.md by hand" | A loose file IS NOT execution of `/dw-run-qa`. The full `QA/` tree is mandatory. |
| "The autopilot already advanced, I don't need to go back" | If the artifact doesn't exist, the step didn't run. Go back and execute. |
| "I fixed bugs along the way, so QA is already ok" | Fixing bugs does not replace running formal QA. Run `/dw-run-qa`. |</critical>

## When to Use
- Use when you want to go from an idea to a PR with minimal manual intervention
- Use for complete features that go through the entire pipeline (research, planning, execution, quality)
- Do NOT use for one-off changes (use `/dw-quick`)
- Do NOT use to fix bugs (use `/dw-bugfix`)
- Do NOT use when you want manual control between each phase (use individual commands)

## Pipeline Position
**Predecessor:** (user's wish) | **Successor:** (PR merge)

## Complementary Skills

| Skill | Trigger |
|-------|---------|
| `dw-memory` | **ALWAYS** — the memory thread runs through every phase (brainstorm -> PRD -> techspec -> tasks -> execution -> QA -> review -> PR). Decisions at one gate feed context into the next. |
| `dw-verify` | **ALWAYS** — invoked at each gate (PRD, Tasks, PR) before asking user approval; and before the final commit + push. |

## Input Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `{{WISH}}` | Description of what the user wants to build | "push notification system with per-channel preferences" |

## Approval Gates

The autopilot stops ONLY at these 3 moments:

1. **GATE 1 — PRD**: Presents the generated PRD and awaits user approval before generating techspec/tasks
2. **GATE 2 — Tasks**: Presents the task list and awaits approval before starting execution
3. **GATE 3 — PR**: After automatic commit, asks if the user wants to generate the Pull Request

## Session Resumption

If this command is invoked to resume an interrupted autopilot (via `/dw-resume`):

<critical>Read the `autopilot-state.json` file in the PRD directory. Skip ALL steps listed in `completed_steps`. Resume execution from `current_step`. Gates already passed (listed in `gates_passed`) MUST NOT be re-presented.</critical>

1. Read `.dw/spec/prd-[name]/autopilot-state.json`
2. Report: "Resuming autopilot from step [N] ([name]). Steps 1-[N-1] already completed."
3. Continue execution normally from the indicated step

## Full Pipeline

### Step 1: Codebase Intelligence

<critical>If `.planning/intel/` exists, querying it is MANDATORY before starting.</critical>

- Query `.planning/intel/` via `/gsd-intel` (if available) or `.dw/rules/` to understand project context
- Identify: tech stack, existing patterns, related features

### Step 2: Research (Conditional)

Evaluate whether the topic requires deep research:
- **YES** (run `/dw-deep-research`): new technology for the project, unknown domain, external API integrations, critical architectural decisions
- **NO** (skip to step 3): simple feature in an already mapped domain, refactoring existing code, basic CRUD
  - If skipping, DOCUMENT the reason in the progress block. E.g.: "Research skipped — domain already mapped in .dw/rules/[file].md". The user must see the justification.

If executed, use `standard` mode by default. Incorporate findings into subsequent steps.

### Step 3: Brainstorm (Interactive)

Run `/dw-brainstorm` with accumulated context (intel + research).
- Generate 3 directions
- Present the 3 directions to the user with your recommendation highlighted and justified
- Wait for user confirmation on which direction to follow before proceeding

### Step 4: PRD (Interactive — 7+ Questions)

<critical>The PRD MUST include an interactive interview with the user. Ask AT LEAST 7 clarification questions BEFORE writing the PRD. Do NOT answer questions automatically based on context — the user MUST respond.</critical>

Run `/dw-create-prd` using brainstorm findings.
- Follow ALL command instructions, especially the clarification questions section
- Ask at least 7 questions about: problem, target users, critical features, scope, constraints, design, integration
- In each question, present a recommendation grounded in brainstorm and deep-research findings (if executed). E.g.: "Based on the research, I recommend X because [evidence]. Do you agree or prefer a different direction?"
- Wait for user responses to each question
- This step is BLOCKING — the command STOPS until a response is received from the user for EACH question. If the user does not respond, do NOT proceed. Do NOT assume answers based on context.
- Only after receiving all responses, write the complete PRD in `.dw/spec/prd-[name]/prd.md`

### === GATE 1: PRD Approval ===

Present to the user:
- Summary of functional requirements
- Decisions made automatically
- Open questions (if any)

**Wait for explicit approval.** If the user requests changes, adjust and re-present.

### Step 5: TechSpec (Interactive — 7+ Questions)

<critical>The TechSpec MUST include an interactive interview with the user. Ask AT LEAST 7 technical clarification questions BEFORE writing the TechSpec. Do NOT answer questions automatically — the user MUST respond.</critical>

Run `/dw-create-techspec` from the approved PRD.
- Follow ALL command instructions, especially the clarification questions section
- Ask at least 7 questions about: preferred architecture, existing vs new libs, testing strategy, integration with existing systems, infrastructure constraints, performance, security
- In each question, present a technical recommendation grounded in brainstorm, deep-research, and approved PRD findings. E.g.: "Research indicated lib X has better performance for this case [source]. Want to use X or have another preference?"
- Wait for user responses to each question
- This step is BLOCKING — the command STOPS until a response is received from the user for EACH question. If the user does not respond, do NOT proceed. Do NOT assume answers based on context.
- Only after receiving all responses, generate in `.dw/spec/prd-[name]/techspec.md`

### Step 6: Tasks

Run `/dw-create-tasks` from PRD + TechSpec.
- Follow all command instructions
- Generate individual tasks in `.dw/spec/prd-[name]/`

### === GATE 2: Tasks Approval ===

Present to the user:
- Task list with brief descriptions
- Dependencies between tasks
- Total effort estimate

**Wait for explicit approval.** If the user requests changes, adjust and re-present.

### Step 7: Design Contract (Conditional)

Evaluate whether tasks involve frontend:
- **YES** (run `/dw-redesign-ui`): if there are tasks with visual components AND the `ui-ux-pro-max` skill is available
  - Generate the design contract in `.dw/spec/prd-[name]/design-contract.md`
  - Present a summary of the design contract to the user (palette, typography, mobile/desktop layout) as a visual checkpoint before proceeding
- **NO** (skip to step 8): purely backend/infra tasks

### Step 8: Execution

Run `/dw-run-plan` with the PRD path.
- Follow ALL command instructions, including GSD integration (plan verification, parallel execution)
- Each task follows `/dw-run-task` with Level 1 validation

### Step 9: Implementation Review (Loop)

<critical>BEFORE the PRD compliance review, run the project's build and lint. If they fail, fix and re-run until they pass. The implementation review CANNOT start with broken build or lint.</critical>

Run the project's build and lint:
1. Identify build and lint commands in `package.json` (scripts `build`, `lint`, `lint:fix`, `type-check`, etc.)
2. Run lint with `--fix` enabled (e.g., `npm run lint -- --fix` or `npx eslint . --fix`) to auto-correct what's possible
3. Run build (e.g., `npm run build` or `npx tsc --noEmit`)
4. If any fail after `--fix`: analyze errors, fix manually, and re-run
5. Repeat until both build AND lint pass without errors
6. Only then proceed to the review

Run `/dw-review-implementation` to verify PRD compliance (Level 2).
- If gaps found: fix automatically and re-run the review
- Maximum 3 correction cycles
- Do NOT advance to QA until the review passes

<critical>Required artifacts for this step (verify BEFORE marking complete):
- Formatted output with compliance matrix shown to the user in the session
- Explicit verdict (PASS / GAP / FAIL) for EACH RF in the PRD, EACH endpoint in the TechSpec, and EACH task
- If gaps: correction commits before re-running the review
A short text review, a "looks good", or a conclusion "implementation is correct" WITHOUT the structured RF-by-RF matrix DOES NOT count. A mental or eyeball review DOES NOT count. If the matrix didn't appear in the output, the command didn't run — re-execute.</critical>

### Step 10: Visual QA

Run `/dw-run-qa` with Playwright MCP.
- Test happy paths, edge cases, negative flows, accessibility
- Document bugs with screenshots

<critical>Required artifacts for this step (run `ls` on EACH path BEFORE marking complete):
- `{{PRD_PATH}}/QA/qa-report.md` — exists and contains a per-RF section with PASS/FAIL
- `{{PRD_PATH}}/QA/bugs.md` — exists (may be empty if no bugs, but the file must exist)
- `{{PRD_PATH}}/QA/checklist.md` — exists and fully covered
- `{{PRD_PATH}}/QA/screenshots/` — directory exists and contains at least 1 PNG per RF tested (format `RF-XX-[slug]-PASS.png` or `-FAIL.png`)
- `{{PRD_PATH}}/QA/scripts/` — directory exists and contains Playwright `.spec.ts`/`.spec.js` scripts per RF
- `{{PRD_PATH}}/QA/logs/` — directory exists with captured console/network logs
Running Playwright ad-hoc, taking a few loose screenshots, or writing a short qa-report.md by hand DOES NOT replace this structure. If any artifact is missing or incomplete, the command did NOT run — invoke `/dw-run-qa` formally and follow its flow to completion.</critical>

### Step 11: Fix QA (Conditional)

If QA found bugs:
- Run `/dw-fix-qa` to fix and retest
- Loop until stable (maximum 5 cycles). After 5 cycles, STOP and ask the user how to proceed.

### Step 12: Implementation Review (Post-QA)

<critical>BEFORE the post-QA review, run build and lint again with --fix. QA fixes may have introduced new issues.</critical>

Run the project's build and lint (same sequence as Step 9):
1. Lint with `--fix` enabled
2. Build
3. If any fail: fix and re-run until they pass

Run `/dw-review-implementation` again to confirm QA fixes did not break PRD compliance.
- If gaps found: fix and re-run
- Maximum 3 cycles

<critical>Required artifacts (same rules as Step 9): explicit RF-by-RF matrix in the output. No matrix = command didn't run = re-execute.</critical>

### Step 13: Code Review

Run `/dw-code-review` (Level 3) for formal review.
- Generate persisted report

### Step 14: Commit

<critical>MANDATORY PRE-COMMIT AUDIT — execute BEFORE invoking `/dw-commit`:

Run `ls` on each path below and confirm existence. If ANY is missing, DO NOT commit:
- `{{PRD_PATH}}/QA/qa-report.md`
- `{{PRD_PATH}}/QA/bugs.md`
- `{{PRD_PATH}}/QA/checklist.md`
- `{{PRD_PATH}}/QA/screenshots/` (non-empty)
- `{{PRD_PATH}}/QA/scripts/` (non-empty with `.spec.*` files)
- `{{PRD_PATH}}/QA/logs/`
- Evidence of the last `/dw-review-implementation` run with RF-by-RF matrix (session output or reference in `autopilot-state.json`)

Also verify `autopilot-state.json`:
- Every step 1 through 13 that is NOT in `skipped_steps` must be in `completed_steps`
- Each completed step must have its artifacts listed in `step_artifacts`

If any artifact or step is missing: STOP immediately. Report to the user: `Step N did not produce artifact X — re-running /dw-[command]`. Re-execute the command. Verify again. Only then proceed to `/dw-commit`.

DO NOT make a partial commit. DO NOT justify the absence with manual validation. DO NOT mark a step as complete without the formal artifact.</critical>

Run `/dw-commit` automatically.
- Semantic commits following Conventional Commits
- Do NOT wait for approval

### === GATE 3: Pull Request ===

Ask the user: **"Commits completed. Do you want to generate the Pull Request?"**

- **YES**: run `/dw-generate-pr` with the target branch
- **NO**: inform that commits are ready and the user can generate the PR manually later

## GSD Integration

<critical>When GSD is installed, ALL GSD integrations from each individual command MUST be executed. The autopilot is not an excuse to skip GSD steps.</critical>

If GSD (get-shit-done-cc) is installed:
- Step 1: use `/gsd-intel` for querying
- Step 8: use plan verification + parallel execution
- All commands: follow their individual GSD sections

If GSD is NOT installed:
- All commands work normally without GSD

## State Persistence

<critical>The autopilot MUST save its state after each completed step to allow resumption via `/dw-resume` in case of interruption.</critical>

Save the file `.dw/spec/prd-[name]/autopilot-state.json` with the following format:

```json
{
  "mode": "autopilot",
  "wish": "original user description",
  "prd_path": ".dw/spec/prd-[name]",
  "current_step": 8,
  "completed_steps": [1, 2, 3, 4, 5, 6, 7],
  "skipped_steps": [2],
  "gates_passed": ["prd", "tasks"],
  "step_artifacts": {
    "9": ["review-matrix-shown-in-session"],
    "10": [
      ".dw/spec/prd-[name]/QA/qa-report.md",
      ".dw/spec/prd-[name]/QA/bugs.md",
      ".dw/spec/prd-[name]/QA/checklist.md",
      ".dw/spec/prd-[name]/QA/screenshots/",
      ".dw/spec/prd-[name]/QA/scripts/",
      ".dw/spec/prd-[name]/QA/logs/"
    ],
    "12": ["review-matrix-post-qa-shown-in-session"]
  },
  "started_at": "2026-04-10T14:30:00Z",
  "last_updated": "2026-04-10T15:45:00Z"
}
```

- Update `current_step`, `completed_steps`, and `step_artifacts` BEFORE moving to the next step
- A step ONLY moves to `completed_steps` after verifying its artifacts exist on disk
- If the session drops, `/dw-resume` will read this file and continue from the correct step — and revalidate artifacts before trusting `completed_steps`
- When the pipeline finishes (after commit or PR), remove the file or mark `"status": "completed"`

<critical>After EACH completed step, display the updated progress block to the user. This is MANDATORY — the user MUST see what was done and what comes next. If a step was skipped, the reason MUST appear in the progress block.</critical>

## Progress Format

During execution, report progress in this format:

```
=== AUTOPILOT =====================================
  OK [1/14] Codebase Intelligence
  OK [2/14] Research (skipped — known domain)
  OK [3/14] Brainstorm
  OK [4/14] PRD
  PAUSE [GATE 1] Awaiting PRD approval...
===================================================
```

## Closing

At the end, present:
- PR link (if generated)
- Summary: steps executed, steps skipped, estimated time saved
- Suggested next steps (merge, deploy, etc.)

</system_instructions>

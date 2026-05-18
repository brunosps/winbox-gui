---
name: dw-verify
description: |
  Enforces fresh verification evidence before any completion, fix, or passing
  claim — and before commits or PR creation. Use when a dev-workflow command
  is about to report success, hand off work, or commit code. Invoked from
  dw-run-task, dw-run-plan, dw-fix-qa, dw-bugfix, dw-code-review, dw-generate-pr,
  and dw-quick. Do not use for early planning or brainstorming.
allowed-tools:
  - Bash
  - Read
  - Grep
---

# dw-verify — Verification Before Completion

## Overview

Claiming work is complete without verification is dishonesty, not efficiency.

**Core principle:** Evidence before claims, always.

**Violating the letter of this rule is violating the spirit of this rule.**

## The Iron Law

```
NO COMPLETION CLAIMS WITHOUT FRESH VERIFICATION EVIDENCE
```

If the verification command has not been run in the current message, the result cannot be claimed.

## The Gate Function

```
BEFORE claiming any status or expressing satisfaction:

1. IDENTIFY: What command proves this claim?
2. RUN: Execute the FULL command (fresh, complete)
3. READ: Full output, check exit code, count failures
4. VERIFY: Does output confirm the claim?
   - If NO: State actual status with evidence
   - If YES: State claim WITH evidence
5. ONLY THEN: Make the claim

Skip any step = lying, not verifying
```

## Scope of Verification

Match the verification scope to the claim scope.

- **Narrow claim** (e.g., "this test passes"): run the specific test.
- **Broad claim** (e.g., "task complete", "ready to commit"): run the full verification pipeline — formatting, linting, all tests, and build. If the project defines a single gate command (e.g., `npm test`, `make verify`, `pnpm check`), run that.

A narrow verification does not support a broad claim. Running one test alone does not justify "task complete." The verification scope must be equal to or broader than the claim scope.

**If in doubt, run the full pipeline.** Over-verification wastes minutes. Under-verification wastes hours.

**Passing pipeline ≠ meeting requirements.** A green build proves the code compiles, lints, and passes existing tests. It does not prove the implementation matches the PRD. For "task complete" or "requirements met" claims, also verify deliverables against the task's acceptance criteria and the PRD — line by line, not by assumption.

## Common Failures

| Claim                 | Requires                         | Not Sufficient                  |
| --------------------- | -------------------------------- | ------------------------------- |
| Tests pass            | Test command output: 0 failures  | Previous run, "should pass"     |
| Linter clean          | Linter output: 0 errors          | Partial check, extrapolation    |
| Build succeeds        | Build command: exit 0            | Linter passing, logs look good  |
| Bug fixed             | Test original symptom: passes    | Code changed, assumed fixed     |
| Regression test works | Red-green cycle verified         | Test passes once                |
| Task complete         | Acceptance criteria cross-check  | Tests passing                   |
| Requirements met      | Line-by-line PRD checklist       | Tests passing                   |

## Red Flags

If you catch yourself using any of these, STOP and run verification:

- "should", "probably", "seems to", "I'm confident"
- Expressing satisfaction before verification
- About to commit, push, or open a PR without verification
- Trusting another agent's success report
- Relying on partial verification
- "Just this once"

## Rationalization Prevention

| Excuse                                  | Reality                |
| --------------------------------------- | ---------------------- |
| "Should work now"                       | Run the verification   |
| "I'm confident"                         | Confidence ≠ evidence  |
| "Just this once"                        | No exceptions          |
| "Linter passed"                         | Linter ≠ compiler      |
| "Agent said success"                    | Verify independently   |
| "I'm tired"                             | Exhaustion ≠ excuse    |
| "Partial check is enough"               | Partial proves nothing |
| "Different words so rule doesn't apply" | Spirit over letter     |

## When To Apply

Apply this skill before:

- any success or completion claim
- any expression of satisfaction with the implementation state
- any commit or PR creation
- any handoff that implies correctness
- moving to the next task based on completion

## Pre-Commit and Pre-PR Gate

Commits and PRs are permanent artifacts. They require the highest verification standard.

**Before `git commit`:**
1. Run the full verification pipeline. Not a subset. The full pipeline.
2. Confirm zero errors, zero warnings, zero test failures in the output.
3. Produce a Verification Report (template below) with verdict PASS.
4. Only then run `git commit`.

**Before creating a PR:**
1. All of the above, plus:
2. Verify the diff matches the intended changes (`git diff` review).
3. Confirm no unrelated files are staged.

If the full pipeline has not passed in this session after the last code change, the commit or PR must not proceed.

## Verification Report Template

Verification is not complete until the agent **cites actual command output** in their response. "I ran it and it passed" is not evidence. If the verification output is not shown, the verification did not happen.

Every verification must be reported using this structure. Do not deviate.

```
VERIFICATION REPORT
-------------------
Claim: [What is being claimed — e.g., "tests pass", "build succeeds", "task complete"]
Command: [Exact command run — e.g., `npm test`, `pnpm verify`]
Executed: [Timestamp or "just now, after all changes"]
Exit code: [0 or non-zero]
Output summary: [Key lines from output — pass count, error count, build result]
Warnings: [Any warnings, or "none"]
Errors: [Any errors, or "none"]
Verdict: PASS or FAIL
```

If the verdict is FAIL, do not use completion language. State what failed and what remains.

If the verdict is PASS, the claim may proceed — but only the specific claim supported by the evidence.

## When Verification Fails

Verification failure is information, not a dead end. Protocol:

1. **Read the failure.** Identify the exact error. Quote the relevant output lines.
2. **Diagnose the root cause.** Read the error. Trace it to the source. If multiple things failed, address them one at a time, starting with the first failure.
3. **Fix the root cause.** Apply the minimal change that addresses the actual error. No workarounds, no suppressions, no skipped checks.
4. **Re-verify from scratch.** Run the full verification command again. Do not assume the fix worked. Do not run only the previously-failing subset.
5. **Report with evidence.** Use the Verification Report Template. If it passes, the claim may proceed. If it fails again, return to step 1.

**Never:**
- Claim partial success ("3 of 4 checks pass, close enough")
- Skip re-verification after a fix
- Blame the tooling without evidence of a false positive
- Move on while verification is still failing

## Project-Specific Verification Commands

dev-workflow does not hardcode a verification command. Discover it from the project:

1. Check `.dw/rules/` for a documented verify command.
2. Read `package.json` scripts: prefer `verify`, `check`, `ci`, `test`, in that order.
3. Check for `Makefile`/`make verify`, `pyproject.toml`/`just verify`, etc.
4. If none is explicit, run the documented test + lint + build sequence.

If no verification command exists for the project, state that explicitly in the Verification Report and avoid completion language.

## Integration With Other dev-workflow Commands

This skill is invoked transparently from:

- `/dw-run-task` — before committing the task's changes
- `/dw-run-plan` — before Level 2 review and before declaring the plan complete
- `/dw-fix-qa` — before marking a bug as resolved in `QA/bugs.md`
- `/dw-bugfix` — before claiming the bug is fixed (original symptom no longer reproduces)
- `/dw-code-review` — before emitting an APPROVED verdict
- `/dw-generate-pr` — blocks PR creation if the session has no passing VERIFICATION REPORT post-last-edit
- `/dw-quick` — before committing the one-off change

Callers should mention this skill in their "Skills Complementares" section so the user sees the dependency.

## Inspired by

Ported from Compozy's `cy-final-verify` skill (`/tmp/compozy/.agents/skills/cy-final-verify/SKILL.md`). Adapted for the dev-workflow context:

- Project-agnostic verification discovery (Compozy assumes `make verify`; dev-workflow scans `package.json`/Makefile/`.dw/rules/`).
- Integration table maps to dev-workflow command names instead of Compozy phases.

Credit: Compozy project (https://github.com/compozy/compozy).

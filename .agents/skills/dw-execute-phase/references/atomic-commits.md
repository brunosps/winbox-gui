# Atomic commits — one commit per task, no exceptions

Every task in a phase commits exactly once. This drives traceability (a task's diff is `git show <sha>`), revert safety (`git revert <sha>` undoes one task without affecting others), and PR clarity (`/dw-generate-pr` builds a clean changelog from the per-task commits).

## Commit message format

Strict format. No deviations.

```
<type>(<scope>): <task title> (RF-XX)

<one-line summary>

- Files added: <comma-separated list, or "none">
- Files modified: <comma-separated list, or "none">
- Tests added/updated: <comma-separated list, or "none">
- Deviations: <link to deviations.md entry, or "none">

Closes RF-XX (partial — full close on tasks.md completion).
```

### Field rules

**`<type>`** — Conventional Commits:

| Type | Use |
|------|-----|
| `feat` | New user-facing capability (default for most PRD tasks) |
| `fix` | Bug fix discovered during the phase (rare in `/dw-execute-phase`; common in `/dw-qa --fix`) |
| `refactor` | Code reshape without behavior change |
| `test` | Tests-only task |
| `docs` | Docs-only task |
| `chore` | Tooling, config, build (rare in PRD-driven phases) |

**`<scope>`** — module name from project rules. If the task touches `src/auth/`, scope is `auth`. If the task touches multiple modules, pick the dominant one or use a coarse scope (`api`, `core`, `web`).

**`<task title>`** — copy from `tasks.md` task line. Imperative, concise. "Add login endpoint", not "Added login endpoint" or "Adding login endpoint".

**`(RF-XX)`** — the requirement this task closes. If the task contributes to multiple, list the primary; mention the others in the body.

**`<one-line summary>`** — one sentence answering "what does this commit deliver?". Different from the title (which is the task title); this is the OUTCOME.

**File lists** — explicit. `src/routes/users.ts, src/services/users.ts, src/schemas/user.ts`, not "user-related files".

**Deviations link** — `.dw/spec/prd-<slug>/deviations.md#deviation-03-1` if the task triggered a Rule 1 or 2 deviation.

**Closes line** — `Closes RF-XX (partial — full close on tasks.md completion)` because one task usually doesn't fully close a requirement; the whole phase does. Final task in the phase changes "(partial — ...)" to "(full)".

## Examples

```
feat(auth): wire JWT middleware to all /api/* routes (RF-04)

Authenticated routes now reject requests without valid Bearer tokens.

- Files added: src/middleware/auth.ts, src/middleware/auth.test.ts
- Files modified: src/server.ts, src/routes/index.ts
- Tests added/updated: src/middleware/auth.test.ts (12 cases — happy path, expired, malformed, missing)
- Deviations: none

Closes RF-04 (partial — full close on tasks.md completion).
```

```
test(orders): add integration tests for order creation flow (RF-07)

Covers happy path, payment failure, inventory mismatch.

- Files added: tests/integration/orders.test.ts
- Files modified: none
- Tests added/updated: tests/integration/orders.test.ts (8 cases)
- Deviations: .dw/spec/prd-checkout-v2/deviations.md#deviation-08-1

Closes RF-07 (partial — full close on tasks.md completion).
```

## Verification before commit

The executor MUST run, in order:

1. **Linter** — project's lint command (`pnpm lint`, `ruff check`, `dotnet format --verify-no-changes`, `cargo clippy`).
2. **Tests** — at minimum the tests touched by this task. Full suite if practical.
3. **Build** — typecheck/compile (`pnpm tsc --noEmit`, `mypy`, `dotnet build`, `cargo check`).

All three must pass. If any fails:
- If the failure is in a test the task added → fix and retry (1 retry, then deviation)
- If the failure is in unrelated code → deviation Rule 2 (ambiguity: does this task own the regression?)

The executor does NOT commit unverified code. Period.

## Edit vs Write

When implementing the task:

| Use Edit when | Use Write when |
|---------------|----------------|
| Modifying an existing file | Creating a new file |
| Changing 1-30 lines | Replacing a file completely |
| You have line context (the file is in your context) | The file is small and a Write is cleaner than 5 Edits |

Never use `cat <<'EOF' > file` heredocs from Bash to create files. Always use Write tool.

## Multi-file tasks

If a task touches 5+ files, that's still one commit. Stage all files (`git add <list>`) then `git commit`. The body's `Files added:` / `Files modified:` lists must include every file.

If a task should logically be split into separate commits (e.g., "create schema then wire it"), that's a sign the planner under-decomposed. The executor flags as Rule 2 deviation, not silently split.

## Commit signing

If `git config commit.gpgsign true` is set, signing is on by default — let it run. Do NOT pass `--no-gpg-sign` from the executor. If signing fails (key missing), surface the error; do NOT bypass.

## What NOT to commit

- `.env` files (even if the task touched them — they should be gitignored already; if not, that's a separate task)
- IDE artifacts (`.vscode/settings.json` user prefs, `.idea/`)
- OS junk (`.DS_Store`, `Thumbs.db`)
- Unrelated changes accidentally in the working tree (executor should `git status` before adding to confirm only the task's files)

If unrelated changes are present, deviation Rule 2: pause and ask — the executor doesn't know if those are user's WIP or expected from a prior task.

## Deviation entry format (referenced from commits)

`.dw/spec/prd-<slug>/deviations.md`:

```markdown
# Deviations — <prd-slug>

## DEVIATION-<TASK_NN>-<RULE_NUMBER>: <title>

- **Task:** <NN> — <task title>
- **Rule:** 1 (auto-add) | 2 (ambiguity) | 3 (architectural conflict)
- **Description:** <1-3 sentences>
- **Files affected:** <list>
- **Resolution:** <what was done; "PAUSED awaiting input" for Rule 2; "BLOCKED — re-plan" for Rule 3>
- **Commit:** <SHA, filled when task commits; empty if Rule 2/3>
```

The plan-checker reads `deviations.md` from the previous run when re-verifying after revision — patterns of recurring Rule 1 deviations indicate the planner is missing a project convention that should be in `.dw/rules/`.

## Final phase commit (handled by `/dw-commit`, not executor)

After all per-task commits, `/dw-generate-pr` (NOT the executor) reads them and builds the PR body. The executor never makes a "wrap-up" commit. If the phase needs a final commit (e.g., updating CHANGELOG.md), that should be the LAST task in `tasks.md` — atomic like every other.

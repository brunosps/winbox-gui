---
name: dw-git-discipline
description: Use when committing or opening a PR. Atomic commits (one intent), Conventional Commits, trunk-based pattern, branch hygiene. Triggers before /dw-commit, /dw-generate-pr, or any git operation.
allowed-tools:
  - Read
  - Bash
---

# Git Discipline

> **Inspired by** [`addyosmani/agent-skills/git-workflow-and-versioning`](https://github.com/addyosmani/agent-skills/tree/main/git-workflow-and-versioning) (MIT). Trunk-based pattern, atomic commit principles, and branch-hygiene patterns adapted from Addy Osmani's work; specifics rewritten for dev-workflow's commit and PR commands.

History is documentation. Bad commit hygiene corrupts the documentation; good hygiene makes future debugging cheap. This skill encodes the rules.

## Three core principles

### 1. One intent per commit

A commit answers ONE question: "what did I change and why?" If your commit message has the word "and" connecting two unrelated changes — split it.

Common splits:
- Refactor + feature → two commits (refactor first, feature second).
- Fix + style cleanup → two commits.
- Type fix + behavior change → two commits.
- Multiple bug fixes → one commit per bug.

**Why this matters:** When something breaks 6 weeks later, `git bisect` returns the commit that introduced it. If that commit also did a rename, a refactor, and a feature, you've gained nothing. If it did one thing, you know what to revert.

See `references/atomic-commits-discipline.md` for the discipline in detail.

### 2. Trunk-based, short-lived branches

Long-lived branches diverge from `main` and become merge nightmares. The discipline:

- Branches live 1-3 days, max a week.
- Daily merge or rebase from `main` to keep close to trunk.
- Incomplete work behind feature flags, not behind a multi-week branch.
- Small PRs (under ~400 lines diff). If bigger, ask: can this be split into independently-mergeable pieces?

See `references/trunk-based-pattern.md` for when this bends and when it doesn't.

### 3. Branch + commit message hygiene

- **Branch names:** `feat/<scope>`, `fix/<scope>`, `refactor/<scope>`, `chore/<scope>`. Lowercase, kebab-case, ≤40 chars.
- **Commit messages:** Conventional Commits (`type(scope): subject`) — `feat`, `fix`, `refactor`, `docs`, `chore`, `test`, `style`, `build`, `ci`, `perf`.
- **Subject line:** ≤72 chars, imperative mood ("add" not "added"), no trailing period.
- **Body (when needed):** explain WHY, not WHAT. The diff already shows what.
- **Footer:** breaking-change marker, issue references, co-author lines.

See `references/branch-hygiene.md` for naming conventions and rebase-vs-merge guidance.

## What this skill enforces

When wired into `/dw-commit`, every commit must:

1. Have a single logical intent (one feature, one fix, one refactor — not mixed).
2. Pass lint + tests + build BEFORE the commit is created.
3. Use Conventional Commits format with correct type/scope.
4. Have a body that explains WHY for non-trivial changes.
5. NOT skip pre-commit hooks (`--no-verify` is forbidden unless user explicitly authorizes).
6. NOT amend an already-pushed commit (history rewrites on shared branches break collaborators).

When wired into `/dw-generate-pr`, every PR must:

1. Have an explanatory description with summary + test plan, not just commit-list dump.
2. Stay reasonably scoped (large PRs flagged with split suggestion).
3. Have a branch that follows naming conventions.
4. Reference an issue / PRD / spec when applicable.

## Quick reference: when to do what

| Situation | Action |
|-----------|--------|
| Mixed refactor + feature in working dir | Stage refactor → commit → stage feature → commit |
| Pre-commit hook fails | Investigate. Fix root cause. NEVER `--no-verify` |
| Want to "tidy up" before PR | `git rebase -i` BEFORE pushing — never after |
| Already pushed and need to fix message | `git revert` + new commit. Don't force-push to shared. |
| Branch is 5 days old, tons of conflicts | Rebase from main daily; don't let drift compound |
| PR has 2,000 lines | Split. Identify natural seams: schema, backend, frontend, tests |
| Stuck mid-merge with conflicts | Resolve, don't `git checkout --theirs` blindly. The conflict is information |

## What this skill does NOT do

- It does not push, force-push, or create branches without explicit user request.
- It does not amend commits without explicit user request.
- It does not collapse multiple commits via `git rebase -i` on already-pushed branches.
- It does not enforce a specific commit count per PR — only that each commit is atomic.

## Integration with dev-workflow commands

- `/dw-commit` runs this skill — verifies lint/tests/build green, drafts a Conventional Commits message, splits commits if multi-intent detected.
- `/dw-generate-pr` uses this skill to validate branch naming, PR body structure, and scope.
- `/dw-run` and `/dw-run` follow the atomic-commit discipline when their executor commits work — one task = one commit (or one logical sub-task).

## Anti-patterns this skill prevents

- "WIP" commits getting merged to `main` (pre-PR cleanup expected).
- "Fix typo" follow-up commits that should have been amended in feature branch.
- Commits with hooks bypassed (`--no-verify`) — root cause should be fixed instead.
- Long-lived branches that drift from main.
- PR descriptions that are just `git log` dumps.
- Force-pushes to shared branches.
- Commits that mix unrelated changes.

## When discipline bends

Real-world software can't always be perfect:

- **Hotfix to production:** atomic still applies; hygiene bends only on subject-line conciseness if needed for clarity.
- **Massive auto-generated change** (lockfile, codemod): one commit is fine; mark `chore(deps)` or `refactor(codemod)` clearly.
- **Reverts:** use `git revert <sha>` (preserves history) over `git reset --hard` (rewrites history). Both create one commit; only revert is safe on shared branches.

## Verification before committing

- [ ] Lint passes
- [ ] Tests pass
- [ ] Build passes
- [ ] One logical intent
- [ ] Conventional Commits subject
- [ ] Body explains WHY (when non-trivial)
- [ ] No `--no-verify`
- [ ] No amend of pushed commit
- [ ] Branch name follows convention

# Atomic commits — one intent per commit

A commit is atomic if it represents exactly one logical change. Atomic commits are the foundation of `git bisect`, `git revert`, code review quality, and reading history six months later.

## The single-intent rule

If your commit message naturally contains "and" connecting two things — that's two commits.

| Bad subject | Better |
|-------------|--------|
| `feat: add login form and fix navbar bug` | Two commits: `feat(login): add form`, `fix(nav): correct mobile menu` |
| `refactor: extract user service and add caching` | Two commits: `refactor(user): extract service`, `feat(user): add result cache` |
| `chore: update deps and reformat` | Two commits: `chore(deps): bump foo to 2.0`, `style: apply prettier sweep` |
| `fix: handle null and add tests` | Two commits: `test(payment): cover null amount case`, `fix(payment): handle null amount` (or merge tests + fix when they're the same logical change — see below) |

**Exception:** if you fix a bug AND add the regression test for it, those belong in ONE commit. The test proves the bug and proves the fix; separating them loses the link.

## Refactor vs feature: always separate

The most common atomicity violation: refactoring "while I'm in here" alongside a feature.

Bad:
```
feat(orders): add bulk-order export AND extract OrderRepository
```

Good:
```
refactor(orders): extract OrderRepository (no behavior change)
feat(orders): add bulk-order export endpoint
```

Why: when bulk-order export turns out to have a bug 2 months later, `git bisect` lands on the second commit. The first commit is a clean prior baseline you can compare against. If they were combined, the refactor's surface area dilutes the diagnosis.

## Practical: how to make commits atomic

When you find yourself with mixed changes in the working directory:

```bash
# 1. See what's changed
git status
git diff

# 2. Stage just the refactor (no behavior change)
git add -p   # interactive — pick hunks one by one
# Or stage specific files:
git add src/orders/repository.ts

# 3. Verify what's staged matches one logical intent
git diff --cached

# 4. Commit
git commit -m "refactor(orders): extract OrderRepository (no behavior change)"

# 5. Stage the feature
git add src/orders/bulk-export.ts src/orders/__tests__/bulk-export.test.ts

# 6. Commit
git commit -m "feat(orders): add bulk-order export endpoint"
```

`git add -p` is the workhorse for unmixing changes. Practice it.

## Commit message structure

Conventional Commits format:

```
type(scope): subject

body — explains WHY, not what (the diff already shows what)

Footer: BREAKING CHANGE, Refs, Co-Authored-By
```

**Type vocabulary:**

| Type | Use for |
|------|---------|
| `feat` | New user-facing capability |
| `fix` | Bug fix |
| `refactor` | Code change with NO behavior change |
| `perf` | Behavior unchanged but faster |
| `docs` | Docs-only |
| `test` | Test additions/changes only |
| `style` | Formatting, no semantic change |
| `chore` | Build, deps, config, no source change |
| `ci` | CI pipeline changes only |
| `build` | Build system changes |

**Subject line rules:**
- ≤72 chars
- Imperative ("add" not "added", "fix" not "fixes")
- No trailing period
- Lowercase after the colon

**Body rules:**
- Wrap at 72-100 chars per line
- Blank line between subject and body
- WHY > WHAT
- Reference issues, PRDs, ADRs when relevant

## When to write a body

You don't need a body for trivial changes. You need one when:

- The reason for the change isn't obvious from the diff (e.g., a workaround for a bug in a dependency).
- The change addresses an issue with non-obvious consequences (e.g., performance, security).
- The change is a deliberate design choice over alternatives (briefly note the rejected option).
- The change has a non-obvious blast radius.

A good rule: if a future maintainer would say "huh, why?" reading the diff, write a body.

## What goes in the footer

- `BREAKING CHANGE: <explanation>` — for breaking-change commits.
- `Refs #123` — issue reference.
- `Co-Authored-By: Name <email>` — for pair work or AI assistance.

## Bisect-friendly commits

For `git bisect` to work, every commit must:
- Build
- Pass tests
- Be a coherent state (not "WIP, half-implemented")

If you can't bisect through your branch, the commits aren't atomic enough. Squash before merging is acceptable; intra-branch WIP commits get rebased away before push.

## What to do with WIP / "save point" commits

Local WIP is fine. Pushed WIP is not.

Workflow:
1. Make many small commits as you work (saves your progress).
2. Before pushing OR before opening PR, `git rebase -i origin/main` to reorganize:
   - Squash WIP commits into their logical parent.
   - Drop accidental commits.
   - Reorder so refactor comes before feature.
   - Edit messages.
3. Push the cleaned history.

After pushing, no rewrites unless explicitly authorized — others may have pulled.

## Common atomic-commit mistakes

- **"Tiny extra fix" piggy-backed.** "While I was in there" → separate commit.
- **Reformat sweep mixed with logic change.** Apply formatter as its own commit BEFORE making logic changes.
- **Test fixture changes mixed with code changes.** If the fixture change is just to support the new code, fine — keep together. If the fixture was wrong before and you're fixing it, separate commit.
- **Lockfile churn separate from intent.** Sometimes lockfile updates accompany intentional dep bumps; that's fine. But noisy lockfile changes from running `npm install` for unrelated reasons → revert.
- **Generated file diffs mixed in.** Generated files (build artifacts, generated types) should regenerate from source automatically; don't commit them by hand alongside source changes if you can avoid it.

## What atomic commits unlock

- `git bisect` finds the exact commit that broke X — and that commit is small enough to inspect quickly.
- `git revert <sha>` removes ONE feature without unwinding others.
- Code review focuses: each commit reviewed in isolation makes a tight discussion possible.
- Cherry-picking to other branches works cleanly when the change is one thing.
- Commit messages become reliable changelog material.

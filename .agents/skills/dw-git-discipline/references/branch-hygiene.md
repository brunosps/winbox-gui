# Branch hygiene — naming, lifetime, rebase vs merge

Branches are conversations between developers. Names, lifetimes, and integration choices shape that conversation.

## Branch naming conventions

Format: `<type>/<scope>` or `<type>/<scope>-<short-description>`

| Prefix | When |
|--------|------|
| `feat/` | New user-facing capability |
| `fix/` | Bug fix |
| `refactor/` | No behavior change, internal restructure |
| `perf/` | Performance improvement |
| `chore/` | Build, deps, config |
| `docs/` | Documentation only |
| `test/` | Test additions or fixes |
| `experiment/` | Spike / exploration; usually NOT merged |
| `hotfix/` | Urgent production fix |
| `release/` | Release preparation (if your team uses release branches) |

Rules:
- Lowercase, kebab-case.
- ≤40 characters total.
- Match the prefix to the dominant change type if mixed.
- Include a PRD/issue reference when one exists: `feat/prd-user-onboarding`, `fix/issue-1234-login-loop`.

Examples:

| Good | Bad |
|------|-----|
| `feat/user-onboarding` | `feature/UserOnboarding` (capitalized) |
| `fix/login-redirect-loop` | `bug-fix-stuff` (no prefix, vague) |
| `refactor/extract-user-service` | `mybranch` (meaningless) |
| `chore/bump-react-19` | `update-deps-and-tests` (multi-intent in name) |

## Branch lifetime

| Branch type | Target lifetime |
|-------------|-----------------|
| `feat/` for normal feature | 1-3 days |
| `fix/` for normal bug | hours to 1 day |
| `hotfix/` | hours, ship same day |
| `refactor/` for small refactor | hours to 1 day |
| `refactor/` for major refactor | break into multiple `refactor/`+ branches, each 1-3 days |
| `experiment/` | as long as needed, but NOT merged to main without conversion |
| `release/` | until release ships |

A branch older than a week is a smell. Either:
- It's blocked on review (fix the review process).
- It's too big (split it).
- It should have been a feature flag on `main` (kill the branch, do it on main).

## Daily integration with `main`

While on a branch, integrate from `main` daily:

```bash
git fetch origin
git rebase origin/main   # preferred for personal branches
# OR
git merge origin/main    # acceptable for shared branches
```

Why daily:
- 1 day of drift = trivial conflicts.
- 7 days of drift = cascading conflicts; same files changed on both sides multiple times.
- Conflicts caught daily are individually small; conflicts caught at PR time are an avalanche.

## Rebase vs merge

| Situation | Choice |
|-----------|--------|
| Personal branch, never shared | Rebase. Linear history. |
| Shared branch (others have pulled) | Merge. Rebase rewrites history; collaborators' branches diverge. |
| Pulling latest `main` into your active feature branch | Rebase, IF nobody else has pulled your branch yet. |
| Merging your feature into `main` via PR | Whatever the team standard is. Squash-merge is fine if commits weren't atomic; preserve commits if they were. |

Rule of thumb: **rebase what's yours; merge what's shared.**

## When NOT to force-push

After a `git rebase`, the branch's history is rewritten — you must force-push to update the remote. This is FINE on a branch only YOU touch. It is BAD on shared branches because:

- Collaborators have pulled the old history; their next pull will conflict in confusing ways.
- Open PRs against the old SHAs may show garbled diffs.
- Reviewers lose context if they linked to a specific SHA.

Discipline:
- `git push --force-with-lease` on personal branches: OK.
- `git push --force` (without `with-lease`): never — too dangerous.
- Force-push to `main` / `master` / `production` / shared release branches: only with explicit authorization, ideally never.

## Cleanup: deleting merged branches

After a branch merges:

```bash
# Delete locally
git branch -d feat/user-onboarding

# Delete remotely (most platforms auto-delete on PR merge)
git push origin --delete feat/user-onboarding
```

A repo with hundreds of stale branches is noise. Most platforms (GitHub, GitLab) auto-delete on merge — enable that setting if you can.

## Stale branch audit

Once a quarter (or whenever the branch list gets noisy):

```bash
# List branches not merged to main, sorted by date
git for-each-ref --sort=-committerdate refs/remotes/origin --format='%(committerdate:short) %(refname:short)' | head -50
```

Anything older than 60 days that hasn't merged: ask the owner — close, finish, or delete.

## Hotfix branches

When prod breaks and you need to ship fast:

1. Branch from `main` (or the production tag, if you have one).
2. Name: `hotfix/<short-description>` or `hotfix/<incident-id>`.
3. Make ONE change. Atomic commit. No "while I'm here" piggybacks.
4. PR to `main` (and to release branch if you have one) — fast review, fast merge, fast deploy.
5. Verify in production.
6. Add a regression test on `main` if hotfix didn't include one.

Hotfix discipline matters because the temptation is to bundle other fixes — and hotfix bundles cause the next outage.

## Stacked branches (advanced)

Sometimes you must stack branches: feature B depends on feature A, A is in review.

Options:

1. **Wait for A to merge.** Best, when possible. Start B from updated `main`.
2. **Stack:** branch B from A. PR B against A. When A merges, rebase B onto `main`, update PR target.
3. **Feature flag both A and B on `main`.** A merges first behind flag, B merges next behind flag, both flip on together.

Option 3 is the cleanest at scale. Stacking causes review and rebase pain.

## Anti-patterns

- Branch named `bruno-changes` — meaningless.
- Branch lives 3 weeks "because we're discussing the design" — branch isn't where design happens. Keep designing in docs/PRDs; branch when you're ready to code.
- `git pull` instead of `git pull --rebase` on a personal feature branch — creates merge commits in your history that you'll then need to clean up.
- Force-pushing to fix "a typo in the commit message" on a shared branch — use `git revert` + new commit instead.
- Multiple developers on the same branch without coordinating push order — non-fast-forward errors and conflicts. Pick one driver per branch or coordinate.

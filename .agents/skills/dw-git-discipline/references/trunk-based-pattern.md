# Trunk-based development — short branches, fast integration

Trunk-based means: `main` is always shippable, and most work lands on `main` within hours-to-days, not weeks. The opposite (long-lived feature branches) creates merge debt that compounds non-linearly.

## The rules

1. **Branches live 1-3 days, max a week.** Past a week, you're carrying risk.
2. **Rebase from `main` daily.** A 1-day rebase is trivial; a 7-day one is a project.
3. **Incomplete work hides behind a feature flag.** It can sit in `main` half-built without users seeing it.
4. **Small PRs (≤400 lines diff).** Larger PRs get worse review quality and longer review cycles.
5. **CI green before merge, always.** Yellow/red CI on `main` blocks everyone.

## Why short branches

A 7-day branch is not 7× the work of a 1-day branch. It's much worse:

- **Compound merge cost.** Every day, the gap between your branch and `main` grows. Conflicts multiply non-linearly because the same files keep moving on both sides.
- **Stale assumptions.** You wrote code against APIs/types from a week ago. The current version differs in ways you don't know.
- **Lost context.** By day 7, you've forgotten why you wrote line 200 of file X. Reviews suffer.
- **Coordination drag.** Other developers can't ship work that depends on your branch finishing.

## Feature flags vs feature branches

The classic question: "I'm building feature X but it's not ready to ship — won't merging it break things?"

Two options:

| Long branch | Feature flag |
|-------------|--------------|
| Code lives on a branch until ready | Code lives on `main` behind a `if (flag)` check |
| Merge debt accumulates daily | No merge debt; integrates as you go |
| Other devs can't see your changes | Other devs can read/review/depend on contracts |
| Single big risky merge at the end | Many small low-risk merges |
| Hard to demo / preview | Can demo to stakeholders by toggling flag |

Feature flags win for almost every non-trivial feature. Costs:

- Flag system needs to exist (LaunchDarkly, GrowthBook, env-based, or just a constant).
- Flags have a lifecycle: introduce → ship code behind flag → enable → remove flag. The remove step is critical and often skipped.

## When trunk-based bends

- **Open-source / external contributors:** they can't push directly to `main`; long branches are inherent to fork-and-PR. Mitigation: small PRs, fast review, frequent merges.
- **Compliance / audited environments:** PR + multi-stage review may take days regardless of branch life. Compress what you can.
- **Big migrations / refactors crossing many modules:** can't always be flag-gated. Mitigation: break into many small PRs, each independently shippable, merged sequentially.

In all bend cases, the SPIRIT of trunk-based still applies: minimize the gap between branch and `main`; integrate often; don't let drift compound.

## The "let it sit on a branch" anti-pattern

Common scenario: developer finishes feature X, opens PR, gets review delays, starts feature Y on a new branch from X (because Y depends on X), then X gets review feedback, X is rebased, Y now diverges from rebased X, X merges, Y has to rebase against the merged-X (different from branched-X), conflicts everywhere, Y stalls, X starts to bit-rot in production usage…

Three fixes:

1. **Don't stack branches.** If Y depends on X, wait for X to land. (Or merge X behind a flag and start Y from `main`.)
2. **Cut review delays.** A PR sitting in review for 3 days is the bug; fix the review process, don't work around it.
3. **Tighter scope.** If X is so big it takes 3 days to review, X should have been three PRs.

## CI and trunk-based

Trunk-based assumes CI is fast and reliable. If CI takes 45 minutes and is flaky:

- Developers won't rebase frequently (too painful).
- Branches drift, then conflicts on merge are big.
- Pressure mounts to bypass CI ("just merge it, CI's broken anyway").

Fixes happen in two directions:

- Make CI fast (under 10 minutes ideally; under 20 acceptable).
- Make CI reliable (flaky tests are bugs; fix or remove them).

A team that can't trust CI cannot run trunk-based effectively.

## Trunk-based + feature flags + atomic commits

These three disciplines compound:

- **Atomic commits** make rebases and reverts surgical.
- **Trunk-based** keeps branches short, so rebases are tiny.
- **Feature flags** decouple "code in main" from "feature live for users."

Together: you can ship work daily, revert any single change without unwinding others, and toggle features without redeploys. This is the working norm for high-velocity teams; it's not optional discipline, it's load-bearing.

# Stop the line — when to drop everything for a bug

The Toyota factory metaphor: any worker can pull the cord and halt production when they see a defect. The reason isn't dramatic — it's economic. Defects compound downstream. Fixing one now is cheaper than ten later, on top of all the work that built on the broken state.

In code, this means: when a real bug surfaces, you stop building features on top of unknown-state code.

## What "stop the line" actually requires

1. **Don't keep coding around it.** If a test is red, you don't merge other PRs that touch the same module until it's green.
2. **Don't ignore "flaky" tests.** Flakiness is a bug — usually a race, sometimes a fixture. "Re-run the suite" is not a fix.
3. **Don't downgrade priority without evidence.** "It's edge case" requires evidence the case is rare AND the impact is low. Often only one of those is true.
4. **Document the pause.** A bug discovered → a tracking issue or task created → a clear "what's blocked by this" note.

## When the rule bends

Stop-the-line is not absolute. It bends when:

- **The bug is in a parallel system you don't own.** Report it; continue your work. (Don't sit blocked waiting on someone else.)
- **Production isn't on fire AND a release is shipping in <24h.** Sometimes "ship the working subset, fix the broken one in the next release" is correct. But this requires explicit acknowledgment, not silent slipping.
- **The bug is informational, not blocking.** A console warning that doesn't affect behavior, a log noise, a deprecated-API notice — those go on a backlog, not stop the line.
- **Reproduction needs production data.** Continue feature work in parallel branches; don't merge them until repro is achieved or scope-isolated.

## When the rule does NOT bend

- Tests are red and you don't know why → stop.
- A previous green build went red after a merge → stop and bisect.
- A user report can be reproduced → stop.
- "Build passes locally but fails CI" → stop. CI is closer to truth.
- Coverage drops below threshold without obvious reason → stop. (Removed a test silently?)

## The cost of not stopping

Three failure modes when teams ignore stop-the-line:

1. **Compounding defects.** Bug A masks bug B; you fix A and now B surfaces in production. The user reports it as a regression of A's fix.
2. **Lost reproduction.** A bug seen in commit N is hard to reproduce by commit N+5 because the surrounding code moved. The fix becomes "rewrite this whole area" because nobody can isolate the original issue anymore.
3. **Test debt.** Red tests get marked `.skip` "temporarily" — and never come back. Three months later half the suite is skipped and nobody trusts it.

## How to communicate "I'm stopping the line"

- "Found a bug in X — pausing my current task to investigate. Will report status in 30 minutes."
- "I can reproduce the issue. Estimated fix path: Y. Do you want me to push through, or hand off?"
- "Stuck at localization for 60 minutes. Tried: A, B, C. Need fresh eyes."

Brief, factual, no apology. Stopping is the right call; you're naming it.

## Anti-patterns to avoid

- "Quick fix and we'll come back to it" — almost never come back to it. Either fix root cause now or formally defer with a tracking issue.
- "It might be flaky" — investigate, don't speculate. Flakiness has a cause.
- Silently rolling back instead of investigating — you lost the chance to learn what broke.
- "Worked around it" with a `// TODO: actual fix` — the workaround is now load-bearing in 3 weeks.

---
name: dw-debug-protocol
description: Use when investigating a bug. Stop-the-line discipline + six-step triage (reproduce → localize → reduce → fix root cause → guard → verify). Triggers on every bug report, test failure, or /dw-bugfix.
allowed-tools:
  - Read
---

# Debug Protocol

> **Inspired by** [`addyosmani/agent-skills/debugging-and-error-recovery`](https://github.com/addyosmani/agent-skills/tree/main/debugging-and-error-recovery) (MIT). Patterns and stop-the-line philosophy adapted from Addy Osmani's work; specific guidance and references rewritten to fit dev-workflow's bugfix and QA loops.

The fastest path through a bug is the disciplined one. This skill encodes the discipline.

## When this skill applies

- Any failing test, runtime error, or "it works locally but not in CI" report.
- Reproducing a user-reported issue.
- A regression after a refactor or dependency bump.
- An intermittent / non-reproducible bug (see `references/non-reproducible-strategy.md`).

If you're tempted to "just try a fix and see," stop. Run the protocol below first.

## The four core rules

### 1. Stop the line

The moment a bug is confirmed, stop adjacent work. Do not pile features on top of broken state. Branches stay frozen until the bug is reproduced and a fix path is identified — *or* explicitly downgraded ("not blocking, scheduled for later").

See `references/stop-the-line.md` for when this rule bends.

### 2. Reproduce before fixing

Without a deterministic reproduction, you cannot know whether your "fix" worked. The reproduction is the hypothesis test. Even a flaky bug needs a *reproducible-enough* test (e.g., "fails 8/10 runs") before fixing.

If you cannot reproduce: see `references/non-reproducible-strategy.md`. Don't fix on guess.

### 3. Find the root cause, not the nearest cause

The first place the stack trace lights up is rarely where the bug LIVES. The bug lives wherever the invariant was violated. Fixing the symptom (e.g., catching a `null` you shouldn't have produced) hides the real issue and makes the next bug worse.

### 4. Guard against recurrence before declaring done

Every fix produces an artifact: a regression test that proves the bug exists, then proves it's fixed. Without this artifact, the bug WILL come back during the next refactor.

## The six-step triage

Each bug runs through these six steps, in order. See `references/six-step-triage.md` for detail.

| Step | Question | Output |
|------|----------|--------|
| 1. Reproduce | Can I trigger this on demand? | A failing test or repro script |
| 2. Localize | Where does the invariant break? | A file:line where state goes wrong |
| 3. Reduce | What's the smallest input that triggers it? | Minimal repro (1-3 lines if possible) |
| 4. Fix root cause | Why is the invariant violated? Fix THAT. | Code change at the cause, not the symptom |
| 5. Guard | What test would have caught this? | Regression test added to suite |
| 6. Verify end-to-end | Does the original report scenario now pass? | Manual or scripted E2E proof |

Do not skip steps. Skipping reduce → bigger fix than needed. Skipping guard → repeat bug in 3 weeks.

## Error categorization

Bugs cluster into a small number of categories. Knowing the category narrows where to look — see `references/error-categorization.md`:

| Category | Typical symptom | Where to look first |
|----------|-----------------|---------------------|
| UI / render | Wrong pixels, missing element, click does nothing | Component tree, prop flow, conditional render |
| Network / I/O | Timeout, 500, partial data | Request payload, headers, error path, retry |
| State / data | Wrong value displayed, stale read | State management, cache invalidation, race |
| Concurrency | "Sometimes fails", deadlock | Async order, locks, await placement |
| Configuration | Works dev, fails prod | Env vars, secrets, build flags, infra config |
| Logic | Branch returns wrong result | Guard conditions, off-by-one, boolean polarity |

## Non-reproducible bugs

Some bugs only happen in production, only on certain users, only at certain times. Don't fix them on intuition. The protocol in `references/non-reproducible-strategy.md` covers:

- Timing/race conditions: instrument with logs first, fix second
- Environment-specific: bisect by environment delta
- State-dependent: capture user state at moment of failure
- Frequency-dependent: deploy logging, wait for next occurrence

A "fix" for an unreproduced bug is a guess. Mark it as such in the commit message.

## Verification before declaring done

A bug is fixed when:

1. The repro from step 1 now passes.
2. The regression test from step 5 was committed.
3. Lint + tests + build are all GREEN.
4. The original report scenario (E2E) was verified — by you, or with a screenshot/log/run trace if not directly reproducible.

If any are missing, the bug is "fixed pending verification," not "fixed."

## Integration with dev-workflow commands

- `/dw-bugfix` runs this skill end-to-end. The bug report is decomposed into steps 1-6 and progressed atomically.
- `/dw-qa --fix` uses this skill when QA findings are bug-shaped (failing scenario rather than missing feature). Each finding becomes a six-step run.
- `/dw-review --code-only` flags fixes that skipped step 5 (no regression test) as REJECTED.

## When to escalate / pair

After 60 minutes stuck at step 2 (localize) or step 4 (root cause), surface the situation to the user:

- "I've exhausted hypotheses A/B/C; here's what I tried and what's left."
- Don't silently spin. Fresh eyes find what stuck eyes miss.

This is not failure — it's protocol. Stuck > 1 hour is a signal, not a flaw.

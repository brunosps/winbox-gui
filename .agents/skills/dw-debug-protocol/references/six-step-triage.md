# Six-step debug triage

Six discrete steps. Each has a specific output. Don't move to the next without finishing the current.

## Step 1 — Reproduce

**Question:** Can I trigger this on demand?

**Output:** A failing test, a script, or a sequence of UI clicks that produces the bug. Save it. The reproduction IS the hypothesis test for your fix.

**How:**
- From a user report → ask: what version, what environment, what input, what was expected, what happened. Get all five before guessing.
- From a stack trace → identify the entry point (HTTP route, CLI command, UI event) and the input that triggered it.
- From a log line → find the surrounding context (request ID, user ID, timestamp); reconstruct the scenario.
- Write a failing test FIRST when the bug is in pure logic. The test commits the bug to record before you fix it.

**Common pitfall:** "It happens sometimes." This is not a reproduction. Either:
- Find a deterministic trigger (often there's a state precondition you missed), OR
- Quantify the rate (8/10 runs) and treat the test as flaky-but-progress.
- See `non-reproducible-strategy.md` for state/timing/env-dependent cases.

**Done when:** You have a one-line command (or a series of clicks) that produces the bug ≥80% of the time.

## Step 2 — Localize

**Question:** Where does the invariant break?

**Output:** A file:line reference where state is wrong. Not where the symptom appears — where the cause lives.

**How:**
- Stack trace → walk UPWARDS from the failure point, asking at each frame: "is the input here valid?" The deepest frame with valid input is where the invariant breaks.
- Bisect the change history if a recent commit broke things: `git bisect start && git bisect bad HEAD && git bisect good <known-good-sha>`.
- Logs/instrumentation → add explicit log lines at suspected points; reproduce; narrow the window.
- Debugger → set breakpoints at boundaries (entry/exit of suspect functions); inspect state.

**Common pitfall:** Stopping at the first place that catches the error. The error often surfaces far from the cause.

**Done when:** You can point at one specific function (or 2-3 closely related lines) and say "the violation happens here."

## Step 3 — Reduce

**Question:** What's the smallest input that triggers this?

**Output:** A minimal reproduction — ideally 1-3 lines of input or a 5-10 line test case.

**How:**
- Start with your full reproduction.
- Remove one element at a time (a config flag, a request field, a piece of state). Check if the bug still reproduces.
- Repeat until removing anything else makes the bug disappear.

**Why bother:**
- Smaller repros are faster to test (your fix loop runs in seconds, not minutes).
- Smaller repros surface the EXACT trigger; vague repros let you fool yourself.
- The minimal repro often makes the cause obvious: "oh, it only fails when the array is empty."

**Common pitfall:** Skipping this step because "I already know what's wrong." If you really know, the reduce takes 2 minutes. Do it.

**Done when:** Removing one more thing makes the bug disappear.

## Step 4 — Fix root cause

**Question:** WHY is the invariant violated? Fix THAT.

**Output:** Code change that addresses the cause, not the symptom.

**How:**
- Ask "why?" until you reach a design or assumption that's wrong, not just a bad value.
  - "X is null" — why?
  - "Because Y returned null" — why?
  - "Because the cache was empty when called from this path" — why was the cache empty?
  - "Because the warm-up runs after this path is reachable" — root cause: ordering.
- Fix at the deepest "why" you can change without scope creep.
- If the root cause is too deep ("this whole module assumes synchronous I/O"), fix at the shallowest level that makes the bug not recur, AND open a follow-up issue for the deeper fix.

**Symptoms vs root cause — examples:**

| Symptom fix (don't) | Root cause fix (do) |
|---------------------|---------------------|
| Catch the null and return default | Find why null was produced and prevent it |
| Add a retry loop | Find why the first call failed |
| Increase the timeout | Find why the operation is slow |
| Add `if (process.env.NODE_ENV !== 'production')` guard | Find why prod has different behavior |
| Suppress the warning | Address what the warning is warning about |

**Common pitfall:** Adding `try/catch` around the symptom and moving on. This is hiding the bug, not fixing it.

**Done when:** Reproduction now passes; lint + tests + build all green.

## Step 5 — Guard against recurrence

**Question:** What test would have caught this before deploy?

**Output:** A regression test (or two) added to the suite that fails without your fix and passes with it.

**How:**
- Take the reproduction from step 1 → make it a test.
- The test name should describe the bug ("returns empty list when cache warm-up is delayed", not "fix bug").
- The test should fail clearly if the bug returns: assertion error with a message naming the invariant.
- Place the test where future maintainers will find it (alongside the function with the cause, not in some catch-all "bugs.test.ts").

**When you can't add a test:**
- Bug requires production-only conditions: add a logging guard or a runtime invariant check that would fire on recurrence.
- Bug is in infra/config: add a CI check that validates the config matches expectations.
- Bug is in third-party library: open an upstream issue AND add a defensive wrapper with a test for your defense.

**Common pitfall:** "I'll add the test later." You won't. Add it now or the bug returns.

**Done when:** Test exists, was committed in same commit as fix, fails on `git revert <fix>`.

## Step 6 — Verify end-to-end

**Question:** Does the original report scenario now pass?

**Output:** Proof — manual verification, screenshot, log trace, scripted E2E run.

**How:**
- Re-read the original bug report.
- Run through it as if you're the reporter — same inputs, same path, same environment if possible.
- Capture evidence (screenshot, terminal output, log line) that the issue no longer reproduces.

**Why this step matters:**
- Unit tests pass ≠ user-facing scenario works. The user's path may differ from the test's path.
- Confirms you fixed the user's bug, not just A bug.
- Catches "fixed it for case A but broke case B" regressions.

**Common pitfall:** Stopping at "tests pass" and skipping the user-facing verification. Tests are necessary, not sufficient.

**Done when:** You can confidently say "the original reporter would now see the expected behavior."

## After all six steps

The fix is ready to commit and ship:

- Commit message: describes the bug, root cause, and fix in 1-3 sentences.
- The regression test goes in the same commit as the fix.
- If a deeper fix was deferred, the follow-up issue is linked from the commit.
- E2E verification evidence is in the PR description (screenshot, log, run output).

This is what "fixed" looks like. Anything less is "in progress."

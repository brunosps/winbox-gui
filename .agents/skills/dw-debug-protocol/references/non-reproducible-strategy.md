# Non-reproducible bugs — strategy when you can't trigger it on demand

Some bugs only fire in production, only for some users, only some times. The protocol differs from normal debugging because step 1 (reproduce) is the open question. Don't fix what you haven't seen.

## Don't fix on guess

The strongest urge with a non-reproducible bug is: "I bet I know what it is — let me push a fix and see if reports stop." This fails because:

1. If the fix doesn't work, you wasted a deploy and learned nothing about the cause.
2. If reports stop (correlation), you can't tell whether your fix worked or the trigger condition stopped occurring.
3. You build a habit of guessing; the next bug, you guess wrong.

Instead: instrument first, fix second.

## The four sub-strategies

### A. Timing / race conditions

**Signs:** "Fails 1 out of N runs," "Worked locally, fails in CI," "Started failing after we sped up X."

**Strategy:**
1. Add timing-stamped logs at every async boundary in the suspect path.
2. Push to a branch with extra logging; run the suite multiple times in CI.
3. Compare the timestamps of failure runs vs success runs. Look for: events arriving in different order, gaps that exceed expectations.
4. Once you see the order anomaly, you have the reproduction (forced via test that constrains order).

**Tools:**
- `Date.now()` or `performance.now()` in logs.
- For network: response time logging.
- For tests: `--repeat 100` (Jest, Vitest) to amplify rare failures.
- Chaos: insert random `await sleep(rand)` at suspected boundaries to provoke order issues.

### B. Environment-specific

**Signs:** "Works in dev, fails in staging," "Works for one user, fails for another," "Worked yesterday, fails today, no code changes."

**Strategy:**
1. Diff the environments. Run a script that prints config (env vars, dependencies, OS, locale, feature flags) in both.
2. Bisect by environment delta: change one variable at a time in the failing environment toward the working environment. The change that flips the bug is the cause.
3. If user-specific: capture the user's state (account flags, history, A/B variants, browser, region) — diff against a working user.

**Tools:**
- Per-user replay: instrument enough that you can re-run a single user's session.
- A/B comparison: working vs failing pair.
- Feature flag audit: every flag that differs is a suspect.

### C. State-dependent

**Signs:** "Happens to long-time users, never new ones," "Only after they've done X then Y," "Cache-related," "Database-state-related."

**Strategy:**
1. Identify the state precondition. (Usually the failing user has a value or pattern in their data that the test fixtures don't.)
2. Capture the failing user's state (sanitized DB rows, sanitized client state).
3. Reproduce locally by seeding that exact state. Now you have a deterministic reproduction.
4. Once reproduced, run the standard six-step protocol.

**Tools:**
- DB dump / sanitize / load locally.
- Client state export (localStorage, IndexedDB, in-memory store snapshot).
- Replay system that takes a captured state and replays user actions.

**Privacy:** Sanitize before capturing. Production user data should never sit in dev environments unsanitized.

### D. Frequency-dependent (rare)

**Signs:** "We see this once a week," "Maybe 0.1% of requests," "Customer complained but logs don't show it."

**Strategy:**
1. Add specific logging or telemetry that would catch the bug NEXT time it fires.
2. Deploy the logging.
3. Wait for next occurrence (set up an alert if possible).
4. Use the captured event as your reproduction.

**Tools:**
- Structured logs filterable by error pattern.
- Sentry / Bugsnag / similar with breadcrumbs (action history before failure).
- Custom telemetry on the suspect code path.

**Time discipline:**
- Don't sit waiting forever. Set a deadline ("if no event in 7 days, escalate or reprioritize").
- Continue other work in parallel; this bug is "instrumented and waiting," not "actively in progress."

## When to ship a guess

Sometimes the bug is high-impact and waiting for instrumentation isn't acceptable. In that case:

1. Acknowledge it's a guess in the commit and PR. Don't pretend you reproduced it.
2. Make the fix MINIMAL. Don't rewrite a module to fix an unconfirmed bug — you'll cause new bugs.
3. Add monitoring/alerting that would catch the bug if it persists. The fix has a verification path: "no recurrence reports in N days."
4. Plan a follow-up: if the bug returns, fall back to the instrument-first protocol.

A guess fix shipped with explicit acknowledgement is acceptable. A guess fix shipped pretending it was a real diagnosis is not — that pattern erodes the team's trust in fix quality.

## Anti-patterns

- "Just add a try/catch and log it" → hides the bug; report stops; cause persists.
- "Restart the service / clear the cache" — operational mitigation, not a fix. Document it as such.
- Fix shipped without monitoring → no way to know if it worked.
- Fix shipped touching unrelated code → "while I was in there" expansion that turns one unverified change into many.
- Refusing to ship until reproduced when production is suffering — perfectionism that costs users. Use the "ship a minimal guess" path with clear acknowledgement.

## When to escalate

After 4 hours instrumenting + waiting with no reproduction:

- Surface the situation: what's instrumented, what's expected, what window of time you're waiting.
- Ask whether to (a) keep waiting, (b) ship a minimal guess fix with monitoring, (c) reprioritize.
- The decision is the user/team's, not yours alone. Provide the trade-off.

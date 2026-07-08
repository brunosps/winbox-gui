# Error categorization — symptom → cause matrix

Before debugging, classify the bug. Wrong category = wasted hours. Most bugs sit in one of six categories; each has a typical hunting ground.

## The six categories

### 1. UI / rendering

**Symptoms:** Wrong pixels, missing element, click does nothing, layout shift, hydration mismatch.

**Hunting ground:**
- Component tree: prop flow from root to leaf.
- Conditional render guards: `{cond && <Foo />}` where `cond` is unexpectedly false (or truthy when 0).
- CSS specificity / cascade collisions.
- Server vs client mismatch (Next.js, hydration).
- Event handler binding (lost `this`, stale closure capturing old state).

**First check:** Open browser devtools → Elements panel → confirm the element exists in DOM. If yes, problem is CSS/layout. If no, problem is render path.

**Often misclassified as:** State bug. Check the prop value at render time before chasing state.

### 2. Network / I/O

**Symptoms:** Timeout, 5xx response, partial data, "loading forever," CORS error.

**Hunting ground:**
- Request payload (DevTools Network tab → check what was actually sent).
- Auth headers / cookies (missing, expired, wrong domain).
- Server-side error (check the API logs, not just the client's view).
- Retry/timeout configuration (too short = flaky; too long = perceived hang).
- Idempotency: request retried in a way the server treated as new.

**First check:** Reproduce in DevTools Network tab. Copy as cURL → run from terminal. If terminal request works, client config is wrong. If terminal also fails, server is wrong.

**Often misclassified as:** Logic bug. Verify the request actually went out before assuming bug is in the response handler.

### 3. State / data

**Symptoms:** Wrong value displayed, stale read, "I updated it but it didn't change," off-by-one in collections.

**Hunting ground:**
- State management: where is the source of truth? How is it updated?
- Cache invalidation: did the cache layer return stale data?
- Race condition: did two updates arrive out of order?
- Mutation vs replacement: did a deep mutation fail to trigger a re-render?
- Derived state: is a computed value re-running when it should, or memoized too aggressively?

**First check:** Log the state at three points — when it's set, when it's read, when it's rendered. The discrepancy reveals the layer with the bug.

**Often misclassified as:** UI bug. The pixel is wrong because the data is wrong; fix the data.

### 4. Concurrency

**Symptoms:** "Sometimes fails," deadlock, race ("works locally, fails in CI"), out-of-order writes, lost update.

**Hunting ground:**
- Async boundaries: missed `await`, dangling promise, fire-and-forget that needed completion.
- Locking: missing lock, lock held too long, lock granularity wrong.
- Order assumptions: code assumes A finishes before B without enforcing it.
- Test isolation: shared state between tests creating false dependency.

**First check:** Add timing logs. If the bug rate changes when you add `setTimeout(0)` or `process.nextTick`, it's a concurrency bug.

**Diagnostic patterns:**
- Sleep-and-check: the bug disappears if you add 100ms delay → ordering issue.
- Always-fails-on-fast-machine: missed async wait.
- Always-fails-on-slow-machine: timeout too tight.
- Fails 1/N runs: race condition; the rate gives clues to the window.

**Often misclassified as:** "Just flaky, ignore." Concurrency bugs always have a cause. Re-running until green is hiding, not fixing.

### 5. Configuration

**Symptoms:** "Works in dev, fails in prod," "works on my machine," sudden behavior change after deploy.

**Hunting ground:**
- Environment variables: present? right value? expected name? leading/trailing whitespace?
- Feature flags: toggled differently between environments?
- Build flags: dev mode strips assertions; prod might not include polyfills.
- Infra: different DB version, different Node version, different network topology.
- Secrets: rotated, expired, wrong key.

**First check:** Print the environment at startup (sans secrets) → diff dev vs prod. The diff often contains the bug.

**Diagnostic command:** `env | sort` (or equivalent) at app entry, in both environments. Compare.

**Often misclassified as:** Logic bug. The code is correct; the configuration is wrong.

### 6. Logic

**Symptoms:** Branch returns wrong result given correct input, incorrect calculation, off-by-one in iteration.

**Hunting ground:**
- Guard conditions: `>=` vs `>`, `&&` vs `||`, negation polarity.
- Edge cases: empty array, single element, very large input, exact boundary value.
- Type coercion: `"0"` is truthy in JS; `0 == false` but `0 !== false`.
- Default values: a missing field falls back to `undefined`, not zero.
- Floating point: `0.1 + 0.2 !== 0.3`.

**First check:** Manually trace the function with the failing input. Often the bug is visible at line 3 of the function.

**Often misclassified as:** State bug. The state is correct; the function uses it wrong.

## Multi-category bugs

Some bugs span categories. The clue: fixing only one category fixes the test but not production.

Example: "Prod login fails."
- Network: auth API returns 401 (real).
- Configuration: prod has rotated secret; staging didn't.
- State: client retains stale token in localStorage from before rotation.

Fix all three or the bug returns next rotation.

## Diagnostic shortcut: where did it last work?

If the bug is new (not always existed):

1. `git log` the affected files. What changed in the last 7 days?
2. `git bisect` if the regression window is wide.
3. Look at infra / dep / config changes (often the cause is outside source code).

If the bug always existed:

1. It's likely an edge case the original author didn't consider.
2. Check the test fixtures — they probably hit the happy path only.
3. Add the failing case to the test fixtures BEFORE fixing.

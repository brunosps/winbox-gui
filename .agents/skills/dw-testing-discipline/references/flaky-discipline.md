# Flaky discipline — taxonomy, quarantine, SLOs

A flaky test is one that produces different verdicts (pass/fail) on the same code across runs. They corrode trust in the suite faster than any other category of test debt.

## The four root causes (in order of frequency)

### Cause 1: Race conditions (concurrency)

**Tells:**
- Test passes locally, fails in CI (or vice versa).
- Failure rate correlates with CI machine load.
- Adding `await page.waitForTimeout(100)` "fixes" it.

**Common scenarios:**
- Async operation completes after test moves on (missing `await`).
- Two requests sent simultaneously, response order matters.
- DOM update happens after assertion runs.
- Database write not yet committed when read fires.

**Fix:**
- Replace wall-clock waits with condition-based waits (`waitFor`, `toBeVisible`, `expect.poll`).
- Add proper `await` on every async operation.
- Use transaction boundaries explicitly when test reads its own write.

### Cause 2: Test order dependency

**Tells:**
- Test passes when suite runs in order, fails with `--shuffle`.
- Test fails when run with `.only` in isolation.
- Failures cluster on first run after CI restart but not afterwards.

**Common scenarios:**
- `beforeAll` populates shared state; second test mutates it; third test fails.
- Test A creates a global mock; Test B inherits it unexpectedly.
- Database row persists across tests because cleanup is in `afterEach` but a test threw mid-execution.

**Fix:**
- Move state creation from `beforeAll` to `beforeEach`.
- Reset shared state in `beforeEach` (clean slate every test).
- Avoid global mocks; scope mocks to the test that needs them.
- Run with `--shuffle` in CI to catch new order dependencies.

### Cause 3: Non-deterministic inputs

**Tells:**
- Test fails at month boundary, year boundary, DST change.
- Test fails based on hostname, locale, timezone.
- Test fails when a flaky RNG produces edge values.

**Common scenarios:**
- `new Date()` in production code, tested without clock fake.
- `Math.random()` for IDs, tested without seed.
- `Intl.DateTimeFormat` rendering based on system locale.
- File paths with timestamps, hash IDs based on time.

**Fix:**
- Mock the clock (`vi.useFakeTimers`, `freezegun`).
- Seed RNG explicitly in tests (`Math.random = () => 0.5` or via DI).
- Pin locale and timezone in CI environment AND in test setup.

### Cause 4: External dependencies

**Tells:**
- Test fails when a third-party service has an outage.
- Test fails when CI runs against a real API and hits rate limits.
- Test fails differently for different geographic CI runners.

**Common scenarios:**
- Direct call to external API in unit tests.
- DNS lookup baked into test execution path.
- CDN-hosted resources in E2E tests.

**Fix:**
- Mock external services at unit/integration layers.
- Use contract tests instead of live calls.
- For E2E, use a sandbox account / dedicated test environment.

## Quarantine workflow

When a test flakes:

### Within 1 hour of detection

1. **Quarantine the test.** Add `.skip` or equivalent. Add a comment:

   ```javascript
   test.skip('FLAKY-2026-05-12: race condition in checkout flow — owner: bruno, fix-by: 2026-05-19', () => {
     // ...
   });
   ```

2. **File a tracking issue.** Title: `FLAKY: <test name>`. Body includes:
   - Test name and file
   - Failure mode observed (race? order-dependency? non-determinism?)
   - First detection: CI run URL, timestamp
   - Hypothesis (if any)
   - Owner and fix-by date

3. **Note in CI.** The next CI run shows "1 quarantined" — make this visible on the dashboard.

### Within 24 hours

1. **Named owner assigned.** Not "team X" — a person.
2. **Fix-by date set.** Default 5 business days. Major flake (production-path test): 2 days.

### When fix-by passes without fix

Escalate:
- Pair the owner with someone for a debug session.
- If still unfixed after 2× the fix-by window, the test is removed (not skipped). A failing un-skipped test is better than a perpetually skipped test.

## SLOs

### `flaky_rate` (first-class metric)

- Definition: `(tests that pass on retry but fail on first run) / (total test runs)`.
- Target: < 1–2% per week.
- Alert at: > 5% on any given day.

### `time-to-fix-flaky`

- Definition: hours from quarantine to fix-merged.
- Target: median < 24 hours; p95 < 7 days.

### `quarantine inventory`

- Definition: count of currently-skipped tests with `FLAKY-*` markers.
- Target: < 10 at any time.
- Alert at: > 25 (the quarantine has become a cemetery — emergency cleanup).

## What NOT to do

- **Auto-retry as fix.** `retries: 3` in CI config is hiding flakes, not fixing them. The 4th run that finally passes still validated nothing.
- **Increase timeouts indefinitely.** A timeout that grows from 5s to 30s "to make CI pass" means the test isn't waiting on the right condition.
- **Remove the test without investigation.** "It's been flaky forever, delete it" — sometimes correct, but make sure the underlying invariant is captured elsewhere.
- **Mark skip without owner.** A skip is a debt. An unowned debt is a perpetual liability.

## When a test should be permanently removed

A flaky test should be DELETED (not just skipped) when:

1. The invariant it tests is covered elsewhere (duplicate per A23).
2. The invariant it tests is no longer a real requirement.
3. The test was always probabilistic by design and never had value.

Deletion is acceptable; abandonment-by-skip is not.

## Real-systems-at-final-gate principle

Many flakes come from mocks drifting from reality. The defense:

- **Unit:** mock the world; fast feedback; flake budget tiny here.
- **Integration:** real DB (testcontainers); mock external services with contract validation.
- **Contract:** Pact / schemathesis verifying producer-consumer agreement.
- **E2E:** real services in a preview environment; near-zero mocks.

When CI is wired this way, a flake at unit usually = race or order-dependency (fixable). A flake at E2E usually = real environment issue (fix the environment, not the test).

## Integration with dev-workflow

- `/dw-qa --fix` uses this taxonomy when retest cycles produce inconsistent results: classify the flake, apply the right fix, document.
- `/dw-review --code-only` flags tests being modified that have a `FLAKY-*` marker — review must verify the flake is now actually fixed, not just made less likely.
- `/dw-qa` weekly summary includes the `flaky_rate` metric.

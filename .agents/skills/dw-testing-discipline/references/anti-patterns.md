# Anti-patterns — 25 smells across 4 families

Each anti-pattern names the smell, shows the violation in pseudo-code, gives the fix, and notes how `/dw-review --code-only` detects it. Agent-specific failure modes are covered separately in `agent-guardrails.md`.

---

## Family A: Fragile to refactor (tests bound to internals, not behavior)

### A1. Implementation-detail selectors

**Violation:**
```javascript
await page.click('.btn.btn-primary.checkout-button');
```

**Fix:** Use `getByRole('button', { name: 'Checkout' })`.

**Detection:** Grep for `.click(`, `.querySelector(`, `cy.get('.`, `getByTestId(` with a class-flavored argument.

### A2. Testing internal structure vs observable behavior

**Violation:**
```javascript
expect(component.state.cart.items.length).toBe(3);
```

**Fix:** Assert what the user sees: `expect(await page.getByText('3 items in cart')).toBeVisible()`.

**Detection:** Tests that import/inspect internal state, refs, or private fields. Class-based component tests that read `.state` directly.

### A3. Testing private methods directly

**Violation:**
```javascript
expect(orderService._calculateTax(...)).toBe(8.5);
```

**Fix:** Test the public method that uses tax calculation; verify the result. If the private method is independently complex, extract it to a module and test that module's public API.

**Detection:** Identifiers starting with `_` accessed from tests.

### A4. Snapshot-as-test (snapshot replacing assertion)

**Violation:**
```javascript
expect(rendered).toMatchSnapshot();  // ← only assertion in test
```

**Fix:** Either:
1. Write specific assertions about what the component renders, OR
2. Use a snapshot AS A SECONDARY check after specific assertions, with a comment classifying the snapshot as `PRODUCT_CONTRACT` (UI contract worth pinning) — never `IMPLEMENTATION_DETAIL`.

**Detection:** Tests where the only assertion is `toMatchSnapshot` or equivalent.

### A5. Vague existence assertions

**Violation:**
```javascript
expect(result).toBeTruthy();
expect(element).toBeDefined();
expect(button).should('exist');
```

**Fix:** Assert what you actually want: `expect(result.status).toBe('success')`, `expect(button).toBeEnabled()`, `expect(button).toHaveText('Continue')`.

**Detection:** Tests asserting only existence/truthiness without follow-up semantic check.

### A6. Action without assertion

**Violation:**
```javascript
test('clicking save works', async () => {
  await page.getByRole('button', { name: 'Save' }).click();
  // ← no assertion. What did "works" mean?
});
```

**Fix:** Define what "works" means. Assert the observable outcome: URL changed, modal closed, success message visible, data persisted.

**Detection:** Tests with `await x.click()` or `await x.type()` followed by no `expect(...)`.

---

## Family B: Non-deterministic outcomes (tests that flip verdict on the same code)

### A7. Static sleeps / fixed-timeout waits

**Violation:**
```javascript
await page.waitForTimeout(2000);
```

**Fix:** `await expect(page.getByText(/order confirmed/i)).toBeVisible({ timeout: 5000 })` — wait on the actual condition.

**Detection:** Grep for `waitForTimeout`, `cy.wait(<number>)`, `sleep(`, `Thread.sleep`, `time.sleep` in test files.

### A8. Test order dependency / hidden shared state

**Violation:** Test B passes only after Test A has run because A populates a shared cache or DB row.

**Fix:** Each test sets up its own state in `beforeEach`. Verify by running tests with `--shuffle` or `--randomize`.

**Detection:** Tests fail when run with `.only`. Tests fail with `--shuffle`. Setup in `beforeAll` instead of `beforeEach`.

### A9. Non-deterministic inputs (clock, RNG, locale)

**Violation:**
```javascript
test('today is Monday', () => {
  expect(new Date().getDay()).toBe(1);  // fails 6 days a week
});
```

**Fix:** Mock the clock (`vi.useFakeTimers()`, `jest.useFakeTimers()`, `freezegun` in Python). Seed RNG. Pin locale.

**Detection:** Tests using `new Date()`, `Date.now()`, `Math.random()`, `Intl.DateTimeFormat` without fakes.

---

## Family C: Mock-driven false confidence (tests asserting on their own setup)

### A10. Asserting the mock exists

**Violation:**
```javascript
expect(mockFn).toBeDefined();
```

**Fix:** Don't assert on mock setup. If the mock is wrong, the behavior assertion downstream will fail naturally.

**Detection:** Mock functions referenced in assertions without `toHaveBeenCalled` semantics.

### A11. Mock drift

**Violation:** Mocked API response set up 6 months ago still returns `{ status: 'OK' }` while the real API now returns `{ ok: true }`.

**Fix:** Contract testing (Pact, schemathesis) or periodic recording (msw + real-traffic capture). Re-validate mocks against real APIs quarterly.

**Detection:** Tests with mocks that haven't been touched in >90 days against APIs that have changed. Hard to detect in CI; needs explicit contract checks.

### A12. Over-mocking child components

**Violation:**
```javascript
vi.mock('./UserAvatar');
vi.mock('./UserMenu');
vi.mock('./UserBanner');
// ... testing nothing real
```

**Fix:** Mock at boundaries (HTTP, DB, third-party SDKs). Render real children unless they're genuinely expensive or test-irrelevant.

**Detection:** Test files with >3 module mocks of internal modules.

### A13. Incomplete mocks (missing fields the code reads)

**Violation:**
```javascript
const mockUser = { id: 1 };  // missing email, but code reads user.email
```

**Fix:** Use a factory that supplies sensible defaults for ALL fields the type/contract declares.

**Detection:** Runtime errors like `Cannot read property 'X' of undefined` inside production code under test.

### A14. Mocking wrong level (mocking methods the logic depends on)

**Violation:**
```javascript
// Testing OrderService, but mocking its private calculate() method
const service = new OrderService();
vi.spyOn(service, 'calculate').mockReturnValue(100);
expect(service.processOrder(...)).toBe(/* uses mocked 100 */);
```

You've tested the SCAFFOLD, not the logic.

**Fix:** Mock at the EDGE (DB call, HTTP call, time). Let internal logic run.

**Detection:** Spies on methods of the System Under Test itself.

---

## Family D: Suite hygiene problems (team and suite-level pathologies)

### A15. Coverage as vanity metric

**Violation:** PR comments demanding "you need to hit 90% coverage" with no discussion of what the coverage means.

**Fix:** Coverage is a flashlight. Use it to FIND blind spots. Don't optimize for the number.

**Detection:** Cultural; visible in PR templates that gate on coverage percentage.

### A16. Happy-path-only coverage

**Violation:** Every test exercises the success case. Edge cases, error paths, boundary values uncovered.

**Fix:** For each unit, write at minimum: happy path + 1 boundary + 1 invalid input + 1 failure path.

**Detection:** Tests where every assertion is positive (`toBe`, `toEqual`) and none is negative (`toThrow`, `toReject`).

### A17. Eternal `beforeAll` / shared setup hiding dependencies

**Violation:**
```javascript
beforeAll(async () => {
  await db.users.create([100 users]);
  await db.orders.create([500 orders]);
});
```

Tests now SHARE state. Order matters. Cleanup is fragile.

**Fix:** `beforeEach` with minimal setup specific to each test.

**Detection:** `beforeAll` blocks creating data (vs `beforeAll` blocks doing one-time framework setup like spinning testcontainers).

### A18. Cleanup in `afterEach` (use `beforeEach` instead)

**Violation:**
```javascript
afterEach(async () => {
  await db.users.deleteAll();
});
```

If a test fails mid-run, cleanup might not run; next test starts dirty.

**Fix:** `beforeEach` with explicit setup-from-clean (truncate + seed). Reliable regardless of previous test outcome.

**Detection:** `afterEach` blocks doing state reset.

### A19. Magic strings and logic in tests

**Violation:**
```javascript
const TIMESTAMP = '2024-01-15T10:30:00Z'; // why?
expect(formatted).toBe('a long string with embedded specifics');
```

When the test fails, what was the test's INTENT?

**Fix:** Use factories with named defaults. Extract magic values to constants with documenting names. Use snapshot testing for legitimate snapshot cases (with classification).

**Detection:** Test files with ≥10 string literals not bound to a named variable.

### A20. Testing against third-party sites you don't control

**Violation:**
```javascript
test('Google homepage loads', async ({ page }) => {
  await page.goto('https://google.com');
  expect(await page.title()).toContain('Google');
});
```

You're testing Google's availability, not your code.

**Fix:** Mock the third party. Use a wiremock or msw to fake their responses. If you must call them, do it in a separate "external dependencies up" smoke test, not unit/integration.

**Detection:** External URLs in test code outside designated smoke tests.

### A21. Quarantine-as-cemetery

**Violation:**
```javascript
test.skip('flaky on CI sometimes', () => { /* ... */ });
// commented 8 months ago, no owner, no fix-by date
```

**Fix:** Every skip/quarantine has a NAMED OWNER and a FIX-BY DATE. Tracking issue exists. PR that introduces the skip says exactly when the test gets fixed.

**Detection:** Skipped tests without comments/labels naming owner and date.

### A22. Retry-as-fix (auto-retry hiding real bugs)

**Violation:**
```javascript
// jest.config or playwright.config
retries: 3,
```

A flaky test is a SIGNAL. Retrying until green hides it.

**Fix:** When a test is flaky, FIX IT (probably a race condition or non-deterministic input). Quarantine if you can't fix immediately. Never just retry.

**Detection:** CI config with retry counts. Test runners showing "1 retry succeeded" badges.

### A23. Duplicate tests across pyramid layers

**Violation:** Same scenario tested at unit, integration, AND E2E. Triple maintenance, no triple value.

**Fix:** Apply Law 2 — lowest layer wins. Drop higher-layer duplicates.

**Detection:** Search for the same scenario name across `tests/unit`, `tests/integration`, `tests/e2e`.

### A24. Weakening tests to make them pass

**Violation:**
```diff
- expect(orders.length).toBe(5);
+ expect(orders.length).toBeGreaterThan(0);
```

The "fix" makes the test useless.

**Fix:** Read Law 3. Fix production OR document WHY the assertion is weaker.

**Detection:** PR diff shows assertion relaxation without commit body explanation.

### A25. Mock-driven confidence (test asserts on its own setup)

**Violation:**
```javascript
const mock = vi.fn().mockReturnValue('hello');
expect(mock()).toBe('hello');
```

You wrote `hello` in the mock. You asserted `hello`. You proved nothing.

**Fix:** Assert on the OUTPUT of the production code that consumed the mock — not on the mock itself.

**Detection:** Tests asserting equality between a value the test body created and a value the test body retrieved.

---

## How `/dw-review --code-only` uses this catalog

For each diff hunk under a test path:
1. Run regex scans for the patterns flagged "Detection" above.
2. Each hit becomes a finding with severity from this skill's `dw-review-rigor` integration.
3. Hits classified as Brittleness/Flakiness/Mock-misuse → severity `high`.
4. Hits classified as Process → severity `medium`.
5. Hits where the SAME test has multiple patterns → severity `critical` (suite-health smell, not just one test).

A PR with ≥1 `high` test anti-pattern that lacks ADR justification gets REJECTED.

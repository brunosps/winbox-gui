# Twelve positive patterns — pseudo-code that survives refactors

Each pattern survives across testing frameworks (Jest, Vitest, Playwright, Cypress, pytest, JUnit). Pseudo-code in JavaScript-flavored examples; translate to your stack.

## 1. Query by behavior and accessible role, not CSS selectors

**Pattern:**

```javascript
// GOOD — describes what the user does
const submitBtn = await page.getByRole('button', { name: 'Submit order' });
await submitBtn.click();
expect(await page.getByText(/order #\d+ confirmed/i)).toBeVisible();
```

**Anti-pattern:**

```javascript
// BAD — describes implementation
await page.click('.btn-primary.submit-btn');
expect(page.querySelector('.confirmation-toast')).toBeTruthy();
```

The good version survives a CSS refactor, a className rename, a Tailwind migration. The bad version breaks on each.

## 2. Selector hierarchy

When the role isn't enough, climb DOWN this ladder. Stop at the highest rung that disambiguates:

1. **Role + name** — `getByRole('button', { name: 'Submit' })`
2. **Label / form association** — `getByLabel('Email')`
3. **Text content** — `getByText('Welcome back')`
4. **Test-id** — `getByTestId('user-menu')` (escape hatch; do not start here)
5. **Structural / CSS** — `querySelector('article:nth-child(3)')` (last resort; flags)

Test-id is fine. Test-id as your default is a sign you're not designing accessible UI.

## 3. Wait on observable conditions, never wall-clock

**Pattern:**

```javascript
// GOOD — waits for the actual condition
await expect(page.getByText('Order confirmed')).toBeVisible({ timeout: 5000 });
```

**Anti-pattern:**

```javascript
// BAD — wall-clock hopes
await page.waitForTimeout(3000);
expect(page.getByText('Order confirmed')).toBeVisible();
```

`waitForTimeout` is the #1 source of flakiness. It races with the network, with rendering, with the event loop. Always wait on what you actually need to see.

## 4. Each test independent and order-free

Every test runs cleanly when invoked in isolation (with `.only`) or in a randomized order.

**Pattern:**

```javascript
beforeEach(async () => {
  // Set up state THIS test needs
  await db.users.create({ id: 'test-1', email: 'a@b.c' });
});

afterEach(async () => {
  // Clean up — but if tests are independent, you may not need this
  await db.users.deleteAll();
});
```

**Anti-pattern:** Shared state in `beforeAll`. Tests passing only when run in suite order. Brittle CI runs.

**Healthier:** prefer setup in `beforeEach` over teardown in `afterEach`. A test that sets up its own state from a clean baseline never wonders "did the previous test corrupt me?"

## 5. One behavior per test, as many assertions as that behavior needs

**Pattern:**

```javascript
test('successful login redirects to dashboard and shows user name', async () => {
  await login('user@example.com', 'password');

  expect(await page.url()).toContain('/dashboard');
  expect(await page.getByRole('heading', { name: /welcome/i })).toBeVisible();
  expect(await page.getByText('user@example.com')).toBeVisible();
});
```

ONE behavior ("successful login leads to dashboard with user info"). THREE assertions because that behavior has three observable parts.

**Anti-pattern:** Two unrelated behaviors crammed into one test ("login + then-also-test-search-works"). When it fails, you don't know which broke.

## 6. Names read as specifications

**Pattern:**

```
should reject invalid email when registering given no prior account
should approve refund within SLA given amount under threshold
should sync calendar events when user reconnects given offline edits
```

Form: `should <expected outcome> when <triggering condition> [given <starting state>]`

**Anti-pattern:**

```
test_login
test_email_1
testHappy
```

These tell you nothing on failure. A spec-style name doubles as documentation when failure messages get noisy.

## 7. Table-driven / parameterized when inputs vary

**Pattern:**

```javascript
describe.each([
  { input: '',           expected: 'required' },
  { input: 'a',          expected: 'too short' },
  { input: 'a'.repeat(100), expected: 'too long' },
  { input: 'a@b.c',      expected: 'valid' },
])('validateEmail($input)', ({ input, expected }) => {
  test(`returns ${expected}`, () => {
    expect(validateEmail(input)).toBe(expected);
  });
});
```

Easy to add cases; impossible to forget one input class.

**Anti-pattern:** Five copy-pasted tests with one variable changed. Drift between them; one gets updated, others don't.

## 8. Build test data via factories

**Pattern:**

```javascript
const buildUser = (overrides = {}) => ({
  id: 'u-' + Math.random().toString(36).slice(2),
  email: 'test@example.com',
  role: 'member',
  createdAt: new Date(),
  ...overrides,
});

test('admin can delete users', () => {
  const admin = buildUser({ role: 'admin' });
  const target = buildUser();
  expect(canDelete(admin, target)).toBe(true);
});
```

Tests focus on the FIELDS that matter to the behavior. Default everything else.

**Anti-pattern:** Literal 50-field JSON blobs copy-pasted across tests. When the schema changes, you update them all — or worse, miss some.

## 9. Mock at boundaries you don't control

**Pattern:**

| Boundary | Test treatment |
|----------|---------------|
| Your own modules | Real (don't mock) |
| Your own DB (with testcontainers) | Real |
| Third-party HTTP API | Mock at fetch/axios level |
| Cloud SDK (AWS, GCP, Stripe) | Mock at SDK level OR sandbox account |
| System clock | Mock when test depends on time |
| RNG | Mock when test depends on randomness |
| File system (when external) | Mock; in tests of fs logic, real temp dir |

**Anti-pattern:** Mocking your own modules so the test is fast. You're now testing the mock setup, not the code.

## 10. Real systems gate final merge

**Pattern:**

```
unit (mocks ok)        → every commit, run locally and in CI
integration (real DB)  → every PR, run in CI
contract (boundary)    → every PR
E2E (real services)    → before merge to main, run in CI on preview env
```

No merge to main without a real-system path going green. Mocks are speed, real is truth.

**Anti-pattern:** 100% mocked test suite. "It all passes locally" → first user request fails because the mock didn't match the real API shape.

## 11. Mutation score over coverage percentage

**Pattern:**

Set up mutation testing (Stryker for JS/TS, mutmut for Python, etc.) ONCE per project. Run weekly on critical modules.

```bash
npx stryker run
# Output: 87 mutants, 78 killed, 9 survived → mutation score 89.6%
```

A surviving mutant means: this code path runs in tests, but the tests don't actually assert anything that breaks when the code changes. Investigate each.

**Anti-pattern:** 95% line coverage with assertions like `expect(result).toBeTruthy()` — every line ran, but mutations all survive. The suite is decorative.

## 12. Page Object Model is a tool, not a religion

For small E2E suites (<20 tests, <5 pages), POM is over-engineering — direct queries are clearer.

For large suites (>50 tests, many pages, multi-step flows), POM pays off:

```javascript
class CheckoutPage {
  constructor(page) { this.page = page; }

  async fillShipping(addr) { /* ... */ }
  async selectPayment(method) { /* ... */ }
  async place() { /* ... */ }
  async waitForConfirmation() { return this.page.getByText(/confirmed/i); }
}

test('checkout end-to-end', async ({ page }) => {
  const checkout = new CheckoutPage(page);
  await checkout.fillShipping(defaultAddress);
  await checkout.selectPayment('card');
  await checkout.place();
  await expect(checkout.waitForConfirmation()).toBeVisible();
});
```

The page object hides the selector mess; tests read like specs. But for a 5-test suite, that wrapper is just noise.

**Rule of thumb:** apply POM when you have ≥3 tests sharing the same page interactions. Otherwise inline.

## Applying these patterns

When `/dw-run` generates a new test, it must comply with these 12. `/dw-review --code-only` checks each diff hunk under test paths against these patterns and the anti-patterns in the sibling reference. Violations become findings.

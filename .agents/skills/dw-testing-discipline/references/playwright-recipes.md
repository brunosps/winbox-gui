# Playwright recipes — concrete tactical patterns

Practical Playwright code for the common scenarios. Use this when `/dw-qa` runs in UI mode or when `/dw-functional-doc` needs E2E coverage.

> These recipes ASSUME the doctrine in this skill (core rules, positive patterns) has already been applied. Recipes are the HOW once the WHY is settled.

## Basic Navigation

```javascript
import { test, expect } from '@playwright/test';

test('homepage loads and shows hero', async ({ page }) => {
  await page.goto('http://localhost:3000');
  await expect(page).toHaveTitle(/Welcome/);
  await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
});
```

Key practices:
- Use `expect(page).toHaveTitle(/pattern/)` instead of `await page.title()` + manual assertion (waits for title to settle).
- Use `getByRole`, not selectors (Positive Pattern #1).

## Form interaction

```javascript
test('login redirects to dashboard with user info', async ({ page }) => {
  await page.goto('/login');

  // Use labels, not selectors
  await page.getByLabel('Email').fill('user@example.com');
  await page.getByLabel('Password').fill('secret123');
  await page.getByRole('button', { name: 'Sign in' }).click();

  // Wait on observable outcome, not wall-clock
  await expect(page).toHaveURL(/\/dashboard/);
  await expect(page.getByText('user@example.com')).toBeVisible();
});
```

## Screenshot capture (debugging + visual evidence)

```javascript
// For debugging
test('checkout flow', async ({ page }) => {
  try {
    await page.goto('/checkout');
    // ... interaction
  } catch (err) {
    await page.screenshot({ path: `failure-${Date.now()}.png`, fullPage: true });
    throw err;
  }
});

// For visual regression (only on stable surfaces)
test('header looks correct', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('banner')).toHaveScreenshot('header.png');
});
```

Warning: visual regression tests need a baseline. Apply the snapshot classification gate from `ai-agent-gates.md` (Gate 5).

## Browser console logs (capture for debug)

```javascript
test('no console errors during checkout', async ({ page }) => {
  const errors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') errors.push(msg.text());
  });
  page.on('pageerror', err => errors.push(err.message));

  await page.goto('/checkout');
  await page.getByRole('button', { name: 'Complete order' }).click();
  await expect(page.getByText('Order placed')).toBeVisible();

  expect(errors).toEqual([]);
});
```

## Network interception (mock or inspect)

```javascript
test('handles API failure gracefully', async ({ page }) => {
  // Intercept the orders API and return 500
  await page.route('**/api/orders', route =>
    route.fulfill({ status: 500, body: 'Server error' })
  );

  await page.goto('/orders');

  // The UI should show a recoverable error state, not crash
  await expect(page.getByText('Could not load orders')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Retry' })).toBeVisible();
});
```

```javascript
test('order list calls API with correct params', async ({ page }) => {
  let capturedUrl;
  page.on('request', req => {
    if (req.url().includes('/api/orders')) capturedUrl = req.url();
  });

  await page.goto('/orders?filter=overdue');

  await expect.poll(() => capturedUrl).toContain('filter=overdue');
});
```

## Wait for conditions (NOT wall-clock)

```javascript
// GOOD — wait on observable condition
await expect(page.getByText(/order #\d+ confirmed/i)).toBeVisible({ timeout: 10000 });

// GOOD — wait on URL change
await page.waitForURL(/\/dashboard/);

// GOOD — wait on network response
const responsePromise = page.waitForResponse(resp => resp.url().includes('/api/orders') && resp.status() === 200);
await page.getByRole('button', { name: 'Load orders' }).click();
await responsePromise;

// GOOD — poll a custom condition
await expect.poll(async () => {
  const count = await page.getByRole('listitem').count();
  return count;
}, { timeout: 5000 }).toBeGreaterThan(0);

// BAD — wall-clock
await page.waitForTimeout(3000);  // ← Anti-pattern A7
```

## Mobile viewport testing

```javascript
test('mobile menu works', async ({ browser }) => {
  const context = await browser.newContext({
    viewport: { width: 375, height: 812 },  // iPhone X
    userAgent: 'Mozilla/5.0 (iPhone; ...)',
  });
  const page = await context.newPage();

  await page.goto('/');
  await page.getByRole('button', { name: /menu/i }).click();
  await expect(page.getByRole('navigation')).toBeVisible();
});
```

For dev-workflow's redesign-ui flow, capture screenshots at TWO viewports: 375px (mobile) and 1440px (desktop). This is documented in `/dw-redesign-ui` step 7.

## Multi-step user journey (Page Object Model)

For >3 tests sharing the same flow, POM pays off:

```javascript
// checkout-page.ts
export class CheckoutPage {
  constructor(private page) {}

  async goto() {
    await this.page.goto('/checkout');
  }

  async fillShipping(addr) {
    await this.page.getByLabel('Address').fill(addr.street);
    await this.page.getByLabel('City').fill(addr.city);
    await this.page.getByLabel('ZIP').fill(addr.zip);
  }

  async selectPayment(method) {
    await this.page.getByLabel(method).check();
  }

  async place() {
    await this.page.getByRole('button', { name: 'Place order' }).click();
  }

  async expectConfirmed() {
    await expect(this.page.getByText(/order #\d+ confirmed/i)).toBeVisible();
  }
}

// checkout.spec.ts
test('checkout end-to-end', async ({ page }) => {
  const checkout = new CheckoutPage(page);
  await checkout.goto();
  await checkout.fillShipping(defaultAddress);
  await checkout.selectPayment('Card ending in 4242');
  await checkout.place();
  await checkout.expectConfirmed();
});
```

## Helper utilities

For small projects, a couple of helpers reduce boilerplate. Drop these in `test-helpers.ts`:

```typescript
import type { Page } from '@playwright/test';

/**
 * Capture browser console logs as the page runs.
 * Pass the returned array to assert on logs at the end of the test.
 */
export function captureConsoleLogs(page: Page) {
  const logs: Array<{ type: string; text: string; ts: string }> = [];
  page.on('console', msg => {
    logs.push({ type: msg.type(), text: msg.text(), ts: new Date().toISOString() });
  });
  return logs;
}

/**
 * Take a timestamped screenshot for debugging or evidence collection.
 */
export async function captureScreenshot(page: Page, name: string) {
  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const filename = `${name}-${stamp}.png`;
  await page.screenshot({ path: filename, fullPage: true });
  return filename;
}
```

## Persistent session (auth state)

When tests need to start logged in, save auth state ONCE in setup:

```typescript
// global-setup.ts
import { chromium } from '@playwright/test';

export default async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();

  await page.goto('http://localhost:3000/login');
  await page.getByLabel('Email').fill('e2e-user@example.com');
  await page.getByLabel('Password').fill(process.env.E2E_PASSWORD);
  await page.getByRole('button', { name: 'Sign in' }).click();
  await page.waitForURL(/\/dashboard/);

  await page.context().storageState({ path: '.auth/user.json' });
  await browser.close();
};

// playwright.config.ts
export default defineConfig({
  globalSetup: require.resolve('./global-setup'),
  use: {
    storageState: '.auth/user.json',
  },
});
```

Tests now start authenticated. No login loop in every test.

## Common pitfalls (cross-reference to anti-patterns)

| Pitfall | Anti-pattern | Fix |
|---------|--------------|-----|
| `await page.click('.btn-primary')` | A1 (implementation selectors) | `getByRole('button', { name: ... })` |
| `await page.waitForTimeout(3000)` | A7 (static sleeps) | Wait on observable condition |
| `expect(button).toBeTruthy()` | A5 (vague existence) | Assert what you actually want |
| Tests pass solo, fail in suite | A8 (order dependency) | Setup in beforeEach, not beforeAll |
| `await page.goto('https://www.google.com')` | A20 (third-party site) | Mock or skip |

## Limitations

- Playwright cannot test native mobile apps (use React Native Testing Library or Detox).
- Some authentication flows (Google Sign-In, MFA hardware keys) cannot be automated; use test-mode bypasses with a dedicated test account.
- Visual regression tests are sensitive to font rendering across OSes; pin to a CI runner OS.

## Cross-skill integration

When running these recipes, the doctrine in this skill applies:
- Apply core rules (especially Rule 2: lowest layer first — many E2E tests should be integration tests instead).
- Run the 7 AI Agent Gates if a coding agent is producing this test.
- Check for Anti-Patterns 1, 5, 7, 20 in the diff.
- For browser security scenarios (auth bypass, XSS, CSRF), see `security-boundary.md`.
- For picking which workflow (UI / network / perf) applies, see `three-workflow-patterns.md`.

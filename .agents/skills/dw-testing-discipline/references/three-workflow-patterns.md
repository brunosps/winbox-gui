# Three workflow patterns for browser-based testing

> Adapted from [`addyosmani/agent-skills/browser-devtools`](https://github.com/addyosmani/agent-skills/tree/main/browser-devtools) (MIT). The three workflows below organize webapp testing tasks by what's actually being verified.

Most webapp testing tasks fall into one of three workflows. Each has different goals, different signals, and different failure modes. Don't conflate them.

## Workflow 1 — UI bugs

**Goal:** verify what the user sees matches what's expected.

**Signals:**
- Screenshot diff vs reference.
- Element exists / does not exist in DOM.
- Element has expected text / attributes.
- Element is visible / styled correctly.
- Click triggers expected navigation or state change.

**Tools:**
- Playwright / Puppeteer for navigation and interaction.
- Visual regression (Percy, Chromatic, Playwright's `toHaveScreenshot`).
- Accessibility checks (`axe-core`, Playwright's accessibility snapshot).

**Common bugs caught:**
- Layout shift after image loads.
- Text wrapping that overflows containers.
- Missing focus styles on interactive elements.
- Hover/active states broken on touch devices.
- Hydration mismatch (server-rendered ≠ client-rendered DOM).

**Common bugs missed:**
- Behavior bugs (click works but state is wrong).
- Race conditions (UI state stable; network race underneath).
- Security bugs (UI hides the action; server still accepts it).

**When to use this workflow:**
- After a CSS / component refactor.
- Before / after design system migrations.
- Smoke testing critical pages on every release.

## Workflow 2 — Network issues

**Goal:** verify the client-server contract is honored under various network conditions.

**Signals:**
- Request was sent with expected payload, headers, method, URL.
- Response was received with expected status, body, headers.
- Retries occurred when expected (and didn't when not).
- Errors are surfaced to UI vs swallowed silently.
- Requests don't fire when they shouldn't (e.g., debounced search).

**Tools:**
- Playwright `page.route()` to intercept and inspect / modify requests.
- DevTools Network panel via MCP / inspection.
- Mock server / MSW for controlled scenarios.
- Network throttling (slow 3G, offline) for resilience tests.

**Common bugs caught:**
- N+1 requests on page load.
- Missing error handling (200 success path tested; 500 path crashes UI).
- Auth headers missing on retried requests.
- Stale data shown after offline reconnect.
- Race conditions when multiple requests resolve out of order.

**Common bugs missed:**
- Visual bugs that don't affect network (CSS issues).
- Server-side bugs (test only checks request/response shape, not server logic).
- Performance bugs at scale (single request looks fine; thousands per second don't).

**When to use this workflow:**
- After auth / API client refactor.
- Verifying offline / connectivity-loss behavior.
- Validating against contract tests / API mocks.
- Reproducing user-reported "loading forever" bugs.

## Workflow 3 — Performance investigation

**Goal:** find why a page is slow and verify a fix.

**Signals:**
- Lighthouse scores (LCP, FID/INP, CLS, TTFB).
- DevTools Performance flame graph timing.
- Bundle analyzer output.
- `web-vitals` library captures from real browser sessions.
- Frame rate during interactions (`requestAnimationFrame` timing).

**Tools:**
- Playwright `tracing.start({ snapshots: true, screenshots: true })`.
- Lighthouse CI for automated runs.
- DevTools Performance tab via MCP.
- WebPageTest for repeatable third-party measurement.
- Bundle analyzer (`next-bundle-analyzer`, `webpack-bundle-analyzer`).

**Common bugs caught:**
- Render-blocking third-party scripts.
- Unintended re-renders amplifying click handlers.
- Large bundle from accidental lib import (e.g., `import _ from 'lodash'` instead of specific function).
- Images larger than displayed size.
- N+1 client-side renders (list of 1000 items each fetching).

**Common bugs missed:**
- Network correctness (perf can pass even when results are wrong).
- Visual issues unrelated to render time.
- Backend perf (this workflow looks at client-side; backend traces are needed too).

**When to use this workflow:**
- Performance regression alerts firing.
- User-reported slowness.
- Before / after a perf-targeted refactor.
- Pre-launch validation against budget targets.

## Choosing a workflow

The first question for any test: what am I trying to verify?

| Concern | Workflow |
|---------|----------|
| "Does the page look right?" | UI bugs |
| "Does clicking this button do the right thing visually?" | UI bugs |
| "Does the API get called correctly?" | Network issues |
| "Does the UI handle errors gracefully?" | Network issues |
| "Why is this page slow?" | Performance |
| "Does this hit our perf budget?" | Performance |
| "Does an attacker get blocked here?" | Network issues + security boundary (`security-boundary.md`) |

Mixing workflows in a single test produces flaky, slow, or incomplete coverage. A test that asserts BOTH "the button is styled correctly" AND "Lighthouse score is >90" runs slowly and fails for unrelated reasons.

## Anti-patterns

- One mega-test that "checks everything" — fails fragilely, hard to debug.
- UI tests that assert pixel-perfect layout in CI (CI rendering differs from local).
- Network tests that mock the entire API (you stop testing the contract; you test the mock).
- Performance tests run once during dev, never in CI (regressions land silently).
- Skipping security workflow because "we have unit tests for auth" — unit tests don't catch UI/server-disagreement bugs.

## How these compose

A real webapp test suite has all three workflows running:

- **Per commit (CI):** UI smoke tests + critical-path network tests.
- **Per PR:** above + visual regression on changed components.
- **Per release:** above + Lighthouse CI + extended network resilience tests.
- **Periodically (nightly / weekly):** full perf baseline + security boundary checks.

Different cadences match different cost profiles. UI smoke is cheap; full perf + security is expensive. Balance accordingly.

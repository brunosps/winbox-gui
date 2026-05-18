---
name: dw-testing-discipline
description: Use when authoring, reviewing, or debugging tests. Six core rules, 25 anti-patterns, 6 agent guardrails, flaky discipline. Triggers on every test diff, /dw-qa, or test-writing session.
allowed-tools:
  - Read
---

# Testing Discipline

## Founding principle

> Tests exist to expose defects, not to keep CI green.
> A test that fails has done its job.
> A test that passes for the wrong reason is worse than no test.

Everything else in this skill follows from that.

## The six core rules

```
1. Test the behavior, never the mock.
2. Push each test to the lowest layer that can detect the defect.
3. When a test fails, read production first — change the test only with documented justification.
4. Real systems gate the merge. Mocks isolate; they do not validate.
5. Coverage is a flashlight; mutation score is a quality probe. Neither is a target.
6. No test-only methods, branches, or flags leak into production code.
```

Each rule has nuance read `references/core-rules.md` for the long version with examples.

## When to use

- Authoring any test (unit, integration, contract, E2E).
- Reviewing a PR diff under test paths.
- Debugging a flaky test (or considering retry-as-fix — read `references/flaky-discipline.md` first).
- Generating tests via an AI agent → invokes `references/agent-guardrails.md` automatically.
- Browser-based E2E with Playwright → recipes in `references/playwright-recipes.md`.
- Verifying browser-side trust boundaries (auth, CSRF, headers) → `references/security-boundary.md`.
- Picking which test workflow applies (UI / network / perf) → `references/three-workflow-patterns.md`.

## Reference router

| Doing what | Read |
|------------|------|
| Placing a new test (which layer?) | `references/core-rules.md` (Rule 2 deep-dive) |
| Writing new tests | `references/patterns.md` |
| Reviewing tests / spotting smells | `references/anti-patterns.md` |
| Agent-generated tests | `references/agent-guardrails.md` + `references/anti-patterns.md` |
| Flaky tests | `references/flaky-discipline.md` |
| Playwright E2E | `references/playwright-recipes.md` |
| Browser trust boundary | `references/security-boundary.md` |
| Picking the right workflow | `references/three-workflow-patterns.md` |

## Patterns that produce reliable tests (one-liners; full in `references/patterns.md`)

1. Query by behavior and accessible role; never CSS selectors or DOM indices.
2. Selector ladder: role → label → text → test-id → structural. Stop at the highest rung that disambiguates.
3. Wait on observable conditions; never wall-clock sleeps.
4. Each test independent and order-free; lean on `beforeEach`, not `beforeAll`.
5. One behavior per test; as many assertions as that behavior requires.
6. Names read as specifications: `should <outcome> when <condition> given <state>`.
7. Table-driven / parameterized when inputs vary.
8. Build test data via factories; literal blobs only for fields under test.
9. Mock at boundaries you don't control; real wiring for the systems you own.
10. Real systems gate the final merge; contract tests bridge unit and E2E.
11. Mutation score, not coverage percentage, measures suite strength.
12. Page Object Model is a tool; collapse it for small suites where it adds noise.

## Anti-pattern catalog (four families, full in `references/anti-patterns.md`)

The four kinds of smell that produce most test debt:

**A. Fragile to refactor** — tests bound to internals, not behavior:
- Implementation-detail selectors.
- Asserting internal structure instead of observable outcome.
- Testing private methods directly.
- Snapshots replacing real assertions.
- Vague existence assertions (`toBeTruthy`, `should('exist')`).
- Actions with no assertion ("clicking save works").

**B. Non-deterministic outcomes** — tests that flip verdict on the same code:
- Static sleeps / fixed-timeout waits.
- Test order dependency / hidden shared state.
- Non-deterministic inputs (clock, RNG, locale).

**C. Mock-driven false confidence** — tests testing the test setup:
- Asserting the mock exists.
- Mock drift (mocked response no longer matches real API).
- Over-mocking child components.
- Incomplete mocks (missing fields the code reads).
- Mocking the wrong level (mocking methods of the SUT itself).
- Asserting on a value the test body fed into a mock.

**D. Suite hygiene problems** — team and suite-level pathologies:
- Coverage as vanity metric.
- Happy-path-only coverage.
- Eternal `beforeAll` hiding dependencies.
- Cleanup in `afterEach` (move to `beforeEach`).
- Magic strings and logic in tests.
- Testing against third-party sites.
- Quarantine-as-cemetery (skip without owner or deadline).
- Retry-as-fix (auto-retry hiding real bugs).
- Duplicate tests across pyramid layers.
- Weakening assertions to make tests pass.

Total: 25 specific patterns across the four families.

## Agent-authoring guardrails (mandatory when an LLM writes tests)

Six guardrails block the most common failure modes when an LLM produces test code. Each is a pre-condition before the diff goes to review. Full prompts and verification in `references/agent-guardrails.md`:

1. **State the invariant first** — agent prints `INVARIANT`, `OWNING_LAYER`, `EXISTING_SUITE` before writing code.
2. **Extend, don't sprawl** — agent extends an existing suite; new files require a named invariant.
3. **Real execution somewhere** — at least one test path runs against real systems before merge.
4. **Red? Read production** — on failure, the agent reads production code first and prints `ANALYSIS:` before changing tests.
5. **Classify before snapshot** — snapshots only with explicit `PRODUCT_CONTRACT` classification; `IMPLEMENTATION_DETAIL` forbids them.
6. **Negative companion** — every positive assertion ships with a negative test for invalid input or failure mode.

## Placement doctrine (tripwires)

Before writing test code:

- Name the invariant in **one sentence**. Fuzzy language signals unclear requirements — stop and clarify.
- Place the test at the **lowest layer** capable of detecting the defect when the invariant breaks.
- Reject tests where (`likelihood-of-bug` × `blast-radius`) falls below a ten-minute-maintenance threshold (the test is more expensive to maintain than the bug would be to fix).

## Flaky discipline (tripwires)

- Quarantine flaky tests within ONE HOUR of detection. Assign a named owner within 24 hours with a fix-by date.
- Track `flaky_rate` as a first-class metric: SLO under 1–2%; alert at >5%.
- Real systems at the final gate: mock at unit; contract-test boundaries; real DB/queue/route at integration; near-zero mocks at E2E.

Full taxonomy in `references/flaky-discipline.md`.

## Cross-cutting red flags

Any of these in a PR is enough to REJECT a verdict:

- Mock setup larger than the test logic.
- Test breaks when an internal method is renamed (not the public contract).
- Removing the assertion body leaves the test green.
- Test fails when run with `.only` in isolation.
- `sleep`, `Thread.sleep`, or `cy.wait(<number>)` appears.
- Selector contains CSS class, index, or `xpath`.
- Test asserts a third-party site is reachable.
- Snapshot diffs accepted without reading.
- Coverage percentage is the only metric quoted.
- Failing tests auto-retried until green; no investigation.
- Skipped/quarantined tests without named owner and fix-by date.
- Test depends on `new Date()`, `Math.random()`, or system locale.
- `afterEach` resets database state.
- Agent-written test has 6+ assertions and zero edge cases.
- The diff contains the phrase "I'll mock this to be safe."

## When NOT to use this skill

- General code review unrelated to tests.
- Library-specific debugging where the test is just a reproduction.
- Non-testing CI pipeline design (deploys, artifacts, secrets).
- Production observability and alerting.
- Single-line typo fixes in existing tests.

## Integration with dev-workflow commands

- `/dw-plan tasks` applies the placement doctrine — each test-adding task names the invariant.
- `/dw-run` runs the 6 agent guardrails when generating tests during implementation.
- `/dw-review --code-only` runs the anti-pattern checks on diff hunks under test paths.
- `/dw-qa --fix` applies the flaky-discipline taxonomy in retest cycles.
- `/dw-qa` (UI mode) references `playwright-recipes.md` for concrete recipes.

## Bottom line

> A test that cannot fail is decorative. A test that fails for the wrong reason is misleading. Build tests that fail for exactly one reason — the reason the invariant was violated — and trust them when they do. Mocks isolate. Real systems validate. Coverage shines a light. Mutation score grades the suite. Agents will reach for the mock and the snapshot; the guardrails make them put both down. Tests reveal bugs, not just pass.

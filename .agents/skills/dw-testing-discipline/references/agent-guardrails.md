# Six guardrails — mandatory when an agent writes tests

LLMs have characteristic failure modes when authoring tests. Six guardrails are forcing functions for the most common ones.

Every test produced by an agent (via `/dw-run`, `/dw-bugfix`, `/dw-autopilot`, or any code-generating flow) must clear all six BEFORE the diff goes to review.

## Guardrail 1 — State the invariant, layer, and host suite first

**Failure mode it blocks:** agent writes 200 lines of test code without articulating what the test is supposed to prove or where it belongs.

**What the agent must do:**

Before any test code, print:

```
INVARIANT: <one sentence — what behavior the test verifies>
OWNING_LAYER: <unit | integration | contract | e2e>
EXISTING_SUITE: <path to the existing test file the new test joins, or "NEW: <reason>">
```

If the agent can't fill any line, it stops and asks the user — it does NOT invent an invariant.

**Why it works:**
- "Invariant" forces specific behavior naming.
- "Owning layer" forces Rule 2 (lowest detectable layer).
- "Existing suite" forces extending coverage rather than spawning orphan files.

**Verification:** `/dw-review --code-only` looks for this 3-line preamble in the PR description or commit body. Missing = REJECTED.

## Guardrail 2 — Real execution somewhere

**Failure mode it blocks:** agent writes tests that mock everything. They pass green forever and validate nothing.

**What the agent must do:**

At SOME layer, the test path must run against real systems before merge:

- Pure logic: unit only is sufficient.
- Code touching DB: at least one integration test with real DB (testcontainers, ephemeral container, dedicated test DB).
- Code calling external services: a contract test OR a sandbox-account smoke test.
- UI interactions: at least one E2E run on a real preview environment.

**Verification:** PR description lists the real-system runs covering the touched code. If no real-system path covers the change → REJECTED.

## Guardrail 3 — On red, read production first

**Failure mode it blocks:** agent sees a test go red and modifies the test until green. Bug ships.

**What the agent must do:**

When a test fails (its own or pre-existing):

1. Print: `INVESTIGATING FAILURE: <test name>`.
2. Read production code in the path that produced the observed value.
3. Print: `ANALYSIS: <2-3 sentences — is production wrong, the test wrong, or has the invariant changed?>`.
4. Decide:
   - Production wrong → fix production.
   - Test wrong → fix the test AND document the change in the commit body.
   - Invariant changed → update the test AND open an ADR if it's a public-contract change.

**Verification:** every commit changing a previously-green test must have an `ANALYSIS:` line. Missing = REJECTED.

## Guardrail 4 — No self-confirming assertions

**Failure mode it blocks:** two related shortcuts —
- Agent writes `mockFn.mockReturnValue('X')` then asserts `expect(mockFn()).toBe('X')`. Proves nothing — the test asserts what the test set up.
- Agent reaches for `toMatchSnapshot()` whenever unsure what to assert. The snapshot becomes the assertion; drift goes unnoticed.

**What the agent must do:**

**For mocks:** never assert on a value the test body fed into a mock. Assert on:
- The OUTPUT of production code that consumed the mock.
- The SIDE EFFECTS (DB state, network calls, event emissions) caused by production code.
- The VISIBLE behavior (UI change, log line, response) the user/caller observes.

**For snapshots:** before adding `toMatchSnapshot()`, classify the artifact:
- `PRODUCT_CONTRACT` — a stable contract worth pinning (serialized API output, stored-record schema). Snapshot OK; document the classification in a comment.
- `IMPLEMENTATION_DETAIL` — HTML structure, internal representation, component tree shape. Snapshot is FORBIDDEN; write specific assertions instead.

**Verification:**
- Mock value flowing directly from setup to assertion without passing through production code → REJECTED.
- Snapshot in diff without classification comment → REJECTED.
- Snapshot classified `IMPLEMENTATION_DETAIL` → REJECTED.

## Guardrail 5 — Negative companion

**Failure mode it blocks:** agent writes happy-path-only tests. Edge cases, error paths, boundary inputs uncovered.

**What the agent must do:**

Every positive assertion ships WITH at least one negative companion:

- Asserting `createUser(validInput)` succeeds → also assert `createUser(invalidInput)` fails with a specific error.
- Asserting `parseDate(validString)` returns a Date → also assert `parseDate(invalidString)` throws or returns null.
- Asserting `transferFunds(...)` succeeds with sufficient balance → also assert it fails with insufficient balance.

**Verification:** a PR adding N positive assertions to a public path must add ≥1 negative assertion. Imbalance >3:1 (positive:negative) on a public path → REJECTED.

## Guardrail 6 — Don't expand the surface to test it

**Failure mode it blocks:** agent exports internals, adds `*ForTesting` methods, or introduces `process.env.NODE_ENV === 'test'` branches in production code to make the test possible.

**What the agent must do:**

If the test needs access the production API doesn't grant:
- **Refactor production for testability** via dependency injection or interface seams.
- **Emit an observable side effect** the test can verify (event, log line, metric) that production also benefits from.
- **Use a dedicated test environment** with test credentials, not a backdoor flag.

The agent does NOT:
- Export `_internal` symbols just for tests.
- Add `// for testing only` methods on classes.
- Wrap production logic in `if (process.env.NODE_ENV !== 'test')` branches.

**Verification:** diff includes new production exports, env checks, or `*ForTesting`-style symbols → REJECTED. Refactor the surface or change the test approach.

## How the six guardrails compose

A test that passes all six:
1. States the invariant, layer, and host suite up front (Guardrail 1).
2. Exercises real systems somewhere in the pipeline (Guardrail 2).
3. When red, reads production first and documents the analysis (Guardrail 3).
4. Asserts on production's observable output, not its own setup (Guardrail 4).
5. Covers failures alongside successes (Guardrail 5).
6. Lives inside the production API's existing surface (Guardrail 6).

Tests passing all six are worth running. Tests missing any one are more likely to mislead than to help.

## Override procedure

To skip a guardrail explicitly:
1. State which guardrail is skipped.
2. State why in one sentence.
3. Add a `// SKIP-GUARDRAIL-N: <reason>` comment in the test.
4. Open a follow-up issue tracking the gap.

Without all four, the guardrail is enforced.

## Prompt block injected when an agent writes tests

```
You are about to write tests. Before producing test code, complete the
6-guardrail preamble:

INVARIANT: ___
OWNING_LAYER: ___
EXISTING_SUITE: ___

If you cannot complete these three lines, STOP and ask the user for
the requirement. Do not invent an invariant.

Then, while writing tests:
- Real execution: name the real-system path covering this code.
- On red: read production first; print ANALYSIS: before changing the test.
- Mocks: never assert on values fed into a mock.
- Snapshots: classify PRODUCT_CONTRACT or IMPLEMENTATION_DETAIL — the latter is forbidden.
- Coverage: every positive assertion needs a negative companion.
- Production surface: don't export internals or add test-only branches.

Tests violating guardrails without explicit SKIP-GUARDRAIL-N comments
will be REJECTED at review.
```

`/dw-run` and `/dw-bugfix` inject this prompt block before generating test code.

## Why six and not more

These are the highest-frequency LLM failure modes observed across multiple projects. Other tendencies exist but are either covered by the positive patterns (e.g., wall-clock waits) or are lower-frequency than these six.

If a new failure mode appears that none of the six catches, add a guardrail AND document the failure that motivated it. Don't add guardrails speculatively.

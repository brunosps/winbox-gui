# Six core rules — expanded with examples

The rules are short for memorization. Each carries nuance that matters in practice.

## Rule 1: Test the behavior, never the mock

**What it means:** the test asserts what the system DOES from the caller's perspective. It does not assert that internal call X was made with internal argument Y.

**Why it matters:** a test bound to internal calls breaks the day you refactor — even when behavior didn't change. The "test is red, behavior is fine" experience erodes trust. Soon no one runs the suite.

**Violation:**

```javascript
// BAD — asserting on mock internals
test('createOrder calls inventory.reserve', () => {
  const inventory = { reserve: vi.fn() };
  createOrder({ items: [...] }, inventory);
  expect(inventory.reserve).toHaveBeenCalledWith(items, 'reserve');
});
```

You've asserted that `createOrder` USES the inventory adapter a specific way. The refactor that consolidates `reserve` into `commit-with-reservation` breaks this even though the order still gets created.

**Correct version:**

```javascript
// GOOD — asserting behavior
test('createOrder reserves inventory before confirming', async () => {
  const result = await createOrder({ items: [...] });
  expect(result.status).toBe('confirmed');
  expect(await getInventoryFor(items[0].sku)).toBe(originalStock - 1);
});
```

Now the test cares about the OUTCOME (inventory decremented, order confirmed), not the path.

## Rule 2: Push each test to the lowest layer that can detect the defect

**What it means:** if a unit test can catch the bug, use a unit. If only an integration test can, integration. If only an end-to-end run can, E2E.

**Why it matters:** lower layers run faster, fail more precisely, isolate the cause better. A bug in pure logic caught at unit takes 50ms and points at the exact function. The same bug at E2E takes 30 seconds and tells you "checkout failed."

**The pyramid resolved:**

| Layer | Catches | Speed | Cost |
|-------|---------|-------|------|
| Unit | Pure logic, math, parsing, formatters | <100ms | low |
| Integration | Module composition, DB queries, HTTP handlers | 500ms–5s | medium |
| Contract | Producer/consumer agreement at API boundary | 1–10s | medium |
| E2E | User journey across multiple services | 10s–60s | high |

**Rule of thumb:**
- If you can write a unit test, do so.
- If unit can't reach it (needs DB, queue, real HTTP), write integration.
- E2E only for journeys NO lower layer can detect: browser-renders-correctly, third-party-callback-arrives, multi-step session state.

## Rule 3: When a test fails, read production first — change the test only with documented justification

**What it means:** a red test is a signal. The first question is "what's wrong with production?" — not "why is the test wrong?"

**Why it matters:** tests are weakened to pass far more often than they should be. "The behavior is fine; the test is too strict" is the slope that leaves a green suite full of meaningless assertions.

**Process when a test goes red:**

1. **Read the failure message.** What invariant did the test claim? What did it observe?
2. **Read production code** in the path that produces the observation.
3. **Decide which is wrong.** If production violates the invariant, fix production. If the test mis-states the invariant, document WHY before relaxing.
4. **Commit the analysis** in the test's commit message or PR body. "Relaxed assertion from X to Y because `<reason>`" is auditable; "fix test" is not.

**Anti-pattern:** re-run the test until green. Auto-retry on flake. Add `.only` to skip the rest.

## Rule 4: Real systems gate the merge. Mocks isolate; they do not validate.

**What it means:** before code merges to main, at least ONE test path exercised real systems — real DB, real route, real external integration in a sandbox or test account. Mocks are fine for fast unit feedback; they cannot decide "safe to ship."

**Why it matters:** mock drift is real. The mocked HTTP response from 3 months ago no longer matches the actual API. Tests pass; production fails on first real call.

**Practical pattern:**

- Unit tests: mock the world; run on every keystroke / commit.
- Integration tests: real local DB (testcontainers, in-memory if equivalent); run on every PR.
- Contract tests: real producer/consumer agreement check; run on every PR.
- E2E: real preview environment with real services; run on PRs before merge to main.

The discipline: no merge without a green E2E (or equivalent real-system check) for the touched path.

## Rule 5: Coverage is a flashlight; mutation score is a quality probe. Neither is a target.

**What it means:**
- **Coverage** tells you what lines executed. Useful as a NEGATIVE signal — 30% coverage means lots of dark code. Useless as a positive signal — 95% coverage with weak assertions is decorative.
- **Mutation score** introduces small bugs (mutations) and measures whether tests catch them. A high mutation score means tests actually probe behavior, not just execute lines.
- Neither should be a number you optimize for. They're diagnostics.

**Anti-pattern:** "We need 90% coverage to merge." Coverage as a gate produces tests written to pass the gate, not to find bugs.

**Healthier framing:** "What lines in the touched diff are NOT covered? Why?" Sometimes the answer is "we don't care, it's logging." Sometimes it's "actually that's a critical branch — add a test."

## Rule 6: No test-only methods, branches, or flags leak into production code

**What it means:** production code should not have `if (process.env.NODE_ENV === 'test') { ... }` branches, `// for testing only` methods exposed on classes, or internals exported just for assertions.

**Why it matters:** production code carrying test-only logic is test decorations leaking into the artifact users run. Bug surface grows; the test environment diverges from production.

**Correct patterns:**

- Need to inject a dependency for testing? Use constructor injection / dependency injection.
- Need to assert on internal state? Add a logging hook or event emission that production also benefits from.
- Need to bypass auth in tests? Use a dedicated test environment with test credentials, not a backdoor flag.

**Tells:**
- `// only used in tests` comments.
- `*ForTesting` suffix on methods.
- `vi.spyOn(module, '_internal')` accessing underscore-prefixed members.
- `process.env.E2E_MODE` reaching into production runtime decisions.

If you see these, the test design is wrong. Refactor production to be testable; don't add backdoors.

## Putting the rules together

A healthy test:
1. Asserts behavior visible to a caller (Rule 1).
2. Sits at the lowest layer that can prove that behavior (Rule 2).
3. When red, sends you to read production code (Rule 3).
4. Has a sibling exercising real systems somewhere in the pipeline (Rule 4).
5. Survives a mutation in the code it claims to cover (Rule 5).
6. Has zero footprint in production code (Rule 6).

Any test failing ≥2 of these is technical debt accumulating. `/dw-review --code-only` flags them.

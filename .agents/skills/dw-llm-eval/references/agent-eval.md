# Agent evaluation — outcome vs trajectory

Agent eval has a foundational question: do you grade **what the agent did along the way** (trajectory) or **what state the world is in at the end** (outcome)?

The answer determines what you measure and what failure modes you catch.

## Outcome-only evaluation (recommended default)

**What it checks:** at the end of the agent's run, does the world look the way it should? Was the right tool called? Was the right ticket filed? Was the user's question answered correctly?

**Pattern:**

```javascript
test('agent files refund ticket when user requests refund', async () => {
  await agent.run('I want a refund for order #123');

  // Outcome assertions
  const tickets = await db.tickets.findMany({ where: { order_id: '123' } });
  expect(tickets).toHaveLength(1);
  expect(tickets[0].type).toBe('refund');

  const userMessage = agent.lastMessage();
  expect(userMessage).toMatch(/refund.*processed|filed|submitted/i);
});
```

**Strengths:**
- Permits creative paths — agent solved it via tool A → B → C OR via tool A → C → B; both pass if the outcome is right.
- Robust to internal refactor — restructuring the agent's prompt or tool descriptions doesn't break the test as long as the outcome holds.
- Aligned with what users care about: did the system do the right thing?

**Weaknesses:**
- Misses "ghost actions" — agent claims to have done X but the outcome state shows it didn't.
  - Defense: combine with rung-3 outcome-state assertions (DB writes, API calls). Don't trust the agent's word.
- Misses inefficiency — agent took 17 tool calls to do what should be 3. Outcome OK but cost is bad.
  - Defense: track tool-call count as a separate metric; alert if it exceeds budget.

## Trajectory evaluation (when path matters)

**What it checks:** did the agent take the expected sequence (or set) of tool calls? Match against a reference trajectory.

**Use when:**
- Compliance / audit requires specific actions in specific order (e.g., "ALWAYS verify identity before disclosing balance").
- Safety-critical: a specific tool MUST be called (e.g., "if user mentions self-harm, must invoke `escalate-to-human` BEFORE any other action").
- The path itself is the contract (e.g., a workflow agent that must traverse a specific decision tree).

## Trajectory match modes

Four modes, from strictest to most permissive:

### Strict

**Rule:** actual trajectory contains identical tool calls in identical order with identical arguments.

**Use when:** path AND parameters are both part of the contract. Compliance, deterministic workflow agents.

```javascript
expect(actualToolCalls).toEqual([
  { name: 'verify_identity', args: { user_id: 'u-42' } },
  { name: 'get_balance', args: { account_id: 'a-99' } },
  { name: 'respond_to_user', args: { template: 'balance-inquiry' } },
]);
```

### Unordered

**Rule:** actual contains the same set of tool calls, any order; arguments match.

**Use when:** the agent legitimately may parallelize or reorder calls without affecting correctness.

```javascript
expect(new Set(actualToolNames)).toEqual(new Set(['fetch_user', 'fetch_orders', 'fetch_addresses']));
```

### Subset

**Rule:** actual trajectory is a SUBSET of reference — agent didn't exceed expected tool calls.

**Use when:** frugality / cost discipline — "agent should NOT call expensive tools unnecessarily."

```javascript
// Reference is the maximum allowed set
const referenceToolCalls = ['fetch_user', 'classify_intent', 'respond'];
const allActualInReference = actualToolNames.every(t => referenceToolCalls.includes(t));
expect(allActualInReference).toBe(true);
```

### Superset

**Rule:** actual contains ALL reference tool calls, possibly plus extras.

**Use when:** specific tools are mandatory but extras are acceptable. Often safety-critical ("MUST log audit event") while permitting agent autonomy in other tool choices.

```javascript
// Reference is the minimum required set
const requiredToolCalls = ['log_audit_event', 'verify_user'];
const allRequiredCalled = requiredToolCalls.every(t => actualToolNames.includes(t));
expect(allRequiredCalled).toBe(true);
```

## Argument matching strategies

When trajectory matching, how should tool ARGUMENTS be compared?

| Strategy | Behavior | Use |
|----------|---------|-----|
| **Exact** | Arguments must match byte-for-byte | Deterministic args (IDs, fixed strings) |
| **Ignore** | Any call to the right tool counts | When the call ITSELF is what matters, not args |
| **Subset** | Actual args contain at least the reference args | Required fields enforced; extras OK |
| **Superset** | Actual args are within reference args set | Frugality — agent didn't add unexpected fields |
| **Custom comparator** | Per-tool comparison function | Domain-specific equivalence (case-insensitive, semantic match) |

Example with custom comparator:

```javascript
const matchers = {
  'search_cities': (actualArgs, refArgs) => {
    // City name comparison: case-insensitive, trimmed
    return actualArgs.name.toLowerCase().trim() === refArgs.name.toLowerCase().trim();
  },
  'fetch_user': 'exact',
};
```

## Decision tree: outcome vs trajectory

```
Is correctness defined by the FINAL STATE or by the PATH?

├── Final state only — agent can solve it however it likes
│   → Outcome-only eval. Combine with rung-3 state assertions.
│
├── Specific tool calls MUST happen for compliance/safety
│   → Superset trajectory mode. Outcome too, as a separate check.
│
├── Specific tool calls MUST NOT happen (cost / privacy)
│   → Subset trajectory mode.
│
├── The full workflow path is the contract (legal, audit)
│   → Strict trajectory mode.
│
└── Path is partly fixed, partly free
    → Trajectory with custom comparator OR split into multiple smaller tests.
```

## Cost vs accuracy tracking

Two metrics that trajectory eval naturally enables:

### Tool-call efficiency

```python
def efficiency_score(actual_trajectory, reference_trajectory):
    actual_calls = len(actual_trajectory)
    reference_calls = len(reference_trajectory)
    if reference_calls == 0:
        return 1.0
    return min(1.0, reference_calls / actual_calls)
```

Score 1.0 = matched or beat reference. Score 0.5 = took 2× the expected calls.

Track this over time; agent regressions sometimes show up as efficiency loss before outcome loss.

### Step-count percentile

```
Run 2026-05-12:
  p50 tool calls: 4 (reference 3)
  p95 tool calls: 9 (reference 6)
  p99 tool calls: 18 (reference 12)
```

p99 spikes catch cases where the agent enters a loop or backtracks excessively — outcome may still be correct but cost runaway is real.

## Dataset structure for agents

```json
{
  "id": "agent-case-001",
  "input": {
    "user_message": "Cancel my order #123 and request a refund"
  },
  "expected_outcome": {
    "tickets_created": [
      { "type": "refund", "order_id": "123" }
    ],
    "order_status": "cancelled"
  },
  "expected_trajectory": {
    "mode": "superset",
    "required_calls": [
      { "name": "verify_user_owns_order", "args_match": "exact" },
      { "name": "update_order_status", "args_match": "subset", "args": { "status": "cancelled" } },
      { "name": "create_refund_ticket", "args_match": "subset" }
    ],
    "forbidden_calls": ["delete_user_data"]
  },
  "tool_budget": { "p95_max_calls": 8 }
}
```

The `forbidden_calls` field is powerful — explicitly enumerate tools that MUST NOT fire for this input class. Catches "agent escalated to a dangerous tool that wasn't necessary."

## Combining outcome + trajectory

For serious agent eval, combine both:

```javascript
test('agent handles refund request', async () => {
  const result = await agent.run(case.input);

  // Outcome
  expectOutcomeMatch(result.outcome, case.expected_outcome);

  // Trajectory — superset mode (required tools called)
  expectTrajectoryMatch(result.trajectory, case.expected_trajectory, 'superset');

  // Forbidden — none of these tools fired
  for (const forbidden of case.expected_trajectory.forbidden_calls) {
    expect(result.trajectory.some(t => t.name === forbidden)).toBe(false);
  }

  // Budget — didn't exceed expected tool calls
  expect(result.trajectory.length).toBeLessThanOrEqual(case.tool_budget.p95_max_calls);
});
```

## LLM-as-judge for agent quality

Beyond mechanical trajectory matching, judge for:
- Was the agent's intermediate reasoning sound? (rubric: logical, evidence-based, non-hallucinated)
- Was the final user message appropriate? (rubric: tone, completeness, accuracy)
- Did the agent handle ambiguity well? (rubric: did it ask for clarification when needed?)

These are rung-4 evaluations on top of rung-1/2/3 outcome and trajectory checks.

## Anti-patterns

- **Trajectory-only eval** → punishes creative paths; brittle to refactor; ignores real outcome.
- **Outcome-only eval without state assertion** → trusts the agent's word; misses ghost actions.
- **Strict trajectory mode when subset/superset would do** → false negatives every time the agent legitimately reorders.
- **No tool-budget tracking** → agent regresses to expensive paths; you don't notice until the bill spikes.
- **No `forbidden_calls` enumeration** → agent silently learns to call dangerous tools.

## Tools

- `langchain-ai/agentevals` (MIT) — Python library implementing all four trajectory match modes + LLM-as-judge for trajectories. Source of the taxonomy above.
- `langsmith` — observability + eval orchestration; tracks experiments over time.
- Custom implementation — the modes above are ~50 lines each in any language.

The discipline isn't the library choice; it's choosing outcome-vs-trajectory deliberately, picking the right match mode, and tracking efficiency alongside accuracy.

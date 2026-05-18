# Oracle ladder — climb deliberately

Five rungs ordered by cost (cheap → expensive) and rigor (strict → subjective). Start at the bottom. Every rung up costs an order of magnitude more in latency, money, or calibration effort. Don't reach for an upper rung when a lower one can prove the case.

## Rung 1 — Exact match

**What it checks:** the output equals the expected output, byte-for-byte (or after a normalization step like JSON canonicalization).

**Use when:**
- Output is a structured function call: `expect(toolCalls[0]).toEqual({ name: 'search', args: { q: 'invoices' } })`.
- Output is a classification from a fixed label set: `expect(label).toBe('refund-request')`.
- Output is a parsed value from a JSON contract: `expect(result.user_id).toBe('u-42')`.

**Example:**

```javascript
test('classifier labels refund requests correctly', async () => {
  const cases = await loadDataset('.dw/eval/datasets/classifier/cases.jsonl');
  for (const c of cases.filter(c => c.expected === 'refund-request')) {
    expect(await classify(c.input)).toBe('refund-request');
  }
});
```

**Cost:** ~free.
**Limitation:** can't handle creative outputs (paragraphs, summaries). Don't try to force-fit.

## Rung 2 — Schema validation

**What it checks:** the output matches a structural contract — types, required fields, value ranges. The SHAPE is fixed; specific values can vary.

**Use when:**
- LLM returns structured data with stable schema (JSON, function call args) but variable content.
- You need to detect "agent returned garbage" without asserting on the exact garbage.

**Example:**

```typescript
import { z } from 'zod';

const ResponseSchema = z.object({
  summary: z.string().min(20).max(500),
  citations: z.array(z.object({
    url: z.string().url(),
    page: z.number().int().optional(),
  })).min(1),
  confidence: z.number().min(0).max(1),
});

test('summarizer returns valid shape', async () => {
  const result = await summarize(input);
  expect(() => ResponseSchema.parse(result)).not.toThrow();
});
```

**Cost:** ~free (schema check is cheap).
**Limitation:** doesn't tell you if the CONTENT is correct, only that it's the right shape. Pair with another rung.

## Rung 3 — Outcome state

**What it checks:** a side effect occurred — DB row was created, file was written, tool was called with valid arguments, ticket was opened. The state of the world matches expectations.

**Use when:**
- Agent has tool access and the GOAL is to change state, not produce prose.
- RAG answer is supposed to lead to an action (e.g., "user clicked the suggested invoice and reconciled it").
- The system has observable side effects you can query post-hoc.

**Example:**

```javascript
test('agent files refund request when user asks', async () => {
  await agent.run('I want a refund for order #123');

  const tickets = await db.tickets.findMany({ where: { order_id: '123' } });
  expect(tickets).toHaveLength(1);
  expect(tickets[0].type).toBe('refund');
  expect(tickets[0].status).toBe('pending');
});
```

**Cost:** cheap (1 DB query / API call per assertion).
**Limitation:** doesn't validate the PROSE the agent produced along the way. If the goal was "answer the user politely AND file the refund," rung 3 catches the action but not the politeness — climb to rung 4 for that.

**Key benefit:** catches "ghost actions" — agent claims to have done X but didn't actually do it. Rungs 1-2 trust the agent's word; rung 3 verifies the world.

## Rung 4 — LLM-as-judge

**What it checks:** a different model grades the output against a rubric. Used for genuinely subjective quality — helpfulness, tone, faithfulness, completeness.

**Mandatory before using:**
- Calibrated against ≥20 human-graded cases (Spearman ≥0.80) — see `judge-calibration.md`.
- Different model than the system under test.
- Structured rubric, not free-form "rate 1-10."

**Example:**

```javascript
test('chat response is faithful to retrieved context', async () => {
  const cases = await loadDataset('.dw/eval/datasets/rag-chat/cases.jsonl');
  const scores = [];

  for (const c of cases) {
    const answer = await chat(c.input, c.context);
    const judgment = await llmJudge({
      model: 'claude-opus-4-7', // different from system under test (GPT-4)
      rubric: faithfulnessRubric,
      input: c.input,
      context: c.context,
      output: answer,
    });
    scores.push(judgment.score);
  }

  // 80% of cases must score ≥4 on the 1-5 faithfulness rubric
  const passing = scores.filter(s => s >= 4).length / scores.length;
  expect(passing).toBeGreaterThan(0.8);
});
```

**Cost:** medium-to-high (one judge call per case; pay per case at API rates).
**Limitation:** the judge has bias and drift; without calibration, you're measuring the judge's mood. Re-calibrate every quarter, every model swap, and after rubric changes.

## Rung 5 — Human review

**What it checks:** a domain expert scores. The gold standard for the rubrics rung 4 calibrates against.

**Use when:**
- Calibrating LLM-as-judge (rung 4 setup).
- High-stakes outputs where automation isn't trusted (medical, legal, financial).
- Edge cases that automated rungs flag as borderline.

**Cost:** expensive. Don't scale; sample.

**Pattern:**
- Spot-check 5-10% of LLM-as-judge results randomly each week.
- Whenever LLM-as-judge score is "borderline" (e.g., 2.5-3.5 on 1-5 scale), kick to human.
- Full human review only for the calibration dataset and high-stakes edge cases.

## The climbing decision tree

```
Is the output a fixed-structure value (function call, classification, JSON with stable shape)?
├── YES → Rung 1 (exact match) or Rung 2 (schema)
└── NO → does the output cause an observable side effect (DB write, tool call, ticket opened)?
    ├── YES → Rung 3 (outcome state)
    └── NO → output is subjective (prose, summary, recommendation). Rung 4 required.
        └── Did you calibrate the judge against humans (≥20 cases, Spearman ≥0.80)?
            ├── YES → Rung 4 is valid signal
            └── NO → DO NOT USE Rung 4 yet. Calibrate first via Rung 5.
```

## Anti-patterns

- **Reaching for Rung 4 first** because "everything else seems hard." Climb the ladder; lower rungs catch loud failures cheaply.
- **Pretending Rung 4 is calibrated** by running it without checking against humans. Score numbers without calibration are decorative.
- **Skipping Rung 3 because "we have unit tests"** — unit tests with mocked tools prove the agent CALLED the tool. Rung 3 proves the tool's effect happened.
- **Mixing rungs in one assertion**: `expect(answer).toBe('Yes, your refund is being processed' /* exact */)` — when the exact text doesn't matter, rung 1 is the wrong tool.

## Combining rungs

For a serious AI feature, expect to use 2-3 rungs together:

| Feature | Typical rung mix |
|---------|------------------|
| Classifier | Rung 1 (label correctness) + Rung 4 (rationale quality, if exposed to user) |
| RAG chat | Rung 2 (response shape) + Rung 3 (citations are valid URLs/IDs) + Rung 4 (faithfulness) |
| Agent (filing tickets) | Rung 3 (ticket created with correct fields) + Rung 4 (user-facing message tone) |
| Summarization | Rung 2 (length, structure) + Rung 4 (faithfulness, completeness) |
| Tool-use trajectory | Rung 1 (specific tool calls expected) + Rung 4 (intermediate reasoning quality, optional) |

The rule: cheap rungs catch the failures that scream; expensive rungs catch the failures that whisper. You need both.

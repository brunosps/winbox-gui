# Reference dataset — 20 from failures beats 200 perfect

The dataset is the bedrock. Without one, every "improvement" is anecdote and every regression goes unnoticed until users complain. With one, you can measure change.

## The 20-from-failures principle

> 20 unambiguous cases drawn from real production failures beat 200 synthetic perfect cases.

Why:
- Synthetic cases reflect what the team IMAGINED would happen — they cover the cases the team already knows about.
- Production failures reflect what ACTUALLY happens — they cover blind spots and edge cases.
- 20 well-curated cases at the right level of difficulty discriminate models better than 200 average-difficulty cases.

A 20-case dataset is enough to:
- Detect regressions of >10% accuracy.
- Calibrate LLM-as-judge.
- Run cheaply enough to evaluate on every PR.

Scale up only when you've validated that 20 is producing useful signal.

## Where cases come from

In rough order of priority:

1. **Real production failures.** User reported a bug, support escalated a case, error logs show an unexpected output. Each becomes a case. Sanitize PII before saving.
2. **Edge cases discovered during development.** "Oh, what if the user asks X?" — add it.
3. **Adversarial examples.** Inputs designed to trip the system (prompt injection, ambiguity, contradiction). Especially important for chat/RAG.
4. **Boundary inputs.** Empty, very long, special characters, mixed languages, unusual encodings.
5. **Synthetic — last resort.** Only when no real input exists yet (e.g., pre-launch). Mark them as synthetic; replace as production data arrives.

Target distribution: **80% real production-sourced**, 20% adversarial/boundary. Pure-synthetic datasets give pure-synthetic confidence.

## Case structure

Each case is one line in `cases.jsonl`:

```json
{
  "id": "case-001",
  "input": {
    "user_message": "I want to cancel my subscription",
    "user_context": { "tier": "premium", "tenure_months": 18 }
  },
  "expected": {
    "intent": "cancellation_request",
    "should_offer_retention": true,
    "tone_targets": ["empathetic", "non-manipulative"]
  },
  "rubric_criteria": ["faithfulness", "tone", "completeness"],
  "metadata": {
    "source": "production-2026-04-12-ticket-T-1234",
    "added_at": "2026-04-15",
    "added_by": "@bruno",
    "difficulty": "medium",
    "tags": ["cancellation", "retention", "premium-user"]
  }
}
```

Fields:

- **`id`** — stable identifier (you'll reference it in regression reports).
- **`input`** — what the system receives. Match the production input shape exactly.
- **`expected`** — for rungs 1-3, the deterministic expected output or state change. For rung 4, the rubric-target (what a 5/5 answer would do).
- **`rubric_criteria`** — which rubric dimensions this case exercises.
- **`metadata.source`** — provenance. Production ticket? Synthetic? Adversarial?
- **`metadata.difficulty`** — easy/medium/hard. Track score by difficulty bucket.
- **`metadata.tags`** — for filtering ("show me cases that exercise retention logic").

## Dataset layout

```
.dw/eval/datasets/<feature-name>/
├── README.md                    # provenance, sample size, last review, change log
├── cases.jsonl                  # the cases themselves
├── rubric.md                    # the rubric used for rung-4 scoring
├── runs/
│   ├── 2026-05-01.jsonl         # one line per case with scores from that run
│   ├── 2026-05-08.jsonl
│   └── ...
├── calibration/
│   ├── 2026-05-12-human-scores.jsonl
│   └── spearman-2026-05-12.txt
└── changelog.md                 # when cases were added/removed and why
```

Everything is committed. Datasets evolve with the feature; the git history shows when and why.

## README.md template

```markdown
# Reference dataset — <feature name>

**Purpose:** evaluate <feature> for <quality dimensions>.

**Current size:** N cases (X production-sourced, Y adversarial, Z synthetic).

**Difficulty distribution:** easy: A, medium: B, hard: C.

**Last reviewed:** YYYY-MM-DD.

**Maintainers:** @name1, @name2.

## When to expand

Add a case when:
- A new production failure is observed (always — that's the primary signal).
- A new edge case is identified during development.
- A new adversarial pattern is discovered (security review, red-team session).

Do NOT add cases just to inflate the count. The 20-from-failures principle: quality over quantity.

## When to retire a case

Retire (don't delete) when:
- The behavior the case checked is no longer relevant (feature removed).
- The case became trivially passing across all model versions (it's no longer discriminating).

Move retired cases to `cases-retired.jsonl` with a `retired_reason`.
```

## Adding cases from production

Process:

1. **Capture the failure** — paste the actual input that failed in `cases-pending.jsonl`.
2. **Sanitize PII** — replace names, emails, IDs, account numbers with realistic-but-fake equivalents. NEVER commit real user data.
3. **Define expected behavior** — what SHOULD have happened. Get sign-off from a domain expert if subjective.
4. **Categorize** — difficulty, tags, rubric criteria.
5. **Promote** to `cases.jsonl` after review.

## Sampling for regression runs

You don't need to re-run the entire dataset every time. Smart sampling:

- **PR-time:** random sample of 30% + all "high difficulty" cases + any case added in the last 30 days. Fast feedback.
- **Pre-merge to main:** full dataset.
- **Nightly:** full dataset + judge re-calibration check.
- **Pre-deploy:** full dataset + manual eyeball on 10 random outputs.

## Detecting drift

After each run, compare against the prior run on the SAME cases:

```
Run 2026-05-08 vs 2026-05-01:
  faithfulness: 4.2 → 3.9 (-0.3) ⚠ regression
  completeness: 4.0 → 4.1 (+0.1)
  tone:         4.5 → 4.4 (-0.1)
  outcome accuracy: 95% → 92% ⚠ regression
```

Two ways drift happens:
1. **Code change degraded quality** — your model swap or prompt tweak hurt something. Bisect.
2. **Judge drift** — the LLM-as-judge itself changed (vendor updated the model). Re-calibrate; the "regression" may be the judge, not the system.

## Dataset versioning

When the dataset materially changes (cases added/removed in batch, rubric updated), bump a version in the README:

```
Dataset version: 2.3
- v2.3 (2026-05-12): added 8 cases from production tickets in last 30 days
- v2.2 (2026-04-15): retired 3 cases that became trivially passing
- v2.1 (2026-03-20): rubric updated to add "completeness" criterion
```

Each run logs which dataset version it ran against. You can't compare a v1 score to a v3 score directly.

## Cost discipline

- Keep the dataset SMALL on purpose. 20-50 cases for most features. 100+ only if the feature has many categorically different inputs.
- Cheap evaluations (rungs 1-3) run on every case every time.
- Expensive evaluations (rung 4) run on samples — 20-50 random cases — except for full pre-deploy runs.

## Anti-patterns

- **Synthetic-only dataset.** No connection to real production. Confidence isn't real.
- **Dataset grew to 500 cases nobody re-reads.** Half are duplicates; half are no longer discriminating. Audit and prune.
- **Cases without expected behavior.** "Just look at the output." No measurement possible.
- **Dataset not committed.** Lives in a notebook; ephemeral; lost when person leaves.
- **No metadata tracking source.** Can't tell synthetic from real; can't audit dataset quality.
- **Dataset reused across features.** Each feature has its own dataset; one-size-fits-all is one-size-fits-none.

## Cross-reference

- `oracle-ladder.md` — what to assert per case.
- `judge-calibration.md` — how to make rung-4 judgments meaningful.
- `rag-metrics.md` — RAG-specific extras for the dataset structure.
- `agent-eval.md` — agent-specific extras (trajectory matching).

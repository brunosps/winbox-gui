---
name: dw-llm-eval
description: Use when authoring or reviewing AI/LLM features (chat, RAG, agents, classifiers). Oracle ladder, reference dataset, judge calibration (Spearman ≥0.80), trajectory eval. Triggers on every AI code path.
allowed-tools:
  - Read
---

# LLM Evaluation

> Adapted patterns from [`langchain-ai/agentevals`](https://github.com/langchain-ai/agentevals) (MIT) for trajectory-match modes, plus general LLM-eval discipline from OpenAI evals cookbook, Anthropic's evals guidance, and the broader open evaluations literature. Material rewritten in our voice.

## When this skill applies

- Any feature that uses an LLM in production: chat, summarization, classification, RAG (retrieval-augmented generation), agents, tool-use, structured extraction, code generation.
- `/dw-plan tasks` when the PRD mentions an AI feature — eval planning becomes a mandatory subtask.
- `/dw-review --code-only` when the diff touches AI feature code paths.
- `/dw-qa --ai` when validating an AI feature against its reference dataset.

If the feature is fully deterministic (no LLM in the loop), use `dw-testing-discipline` instead — Iron rules and 25 anti-patterns. This skill is specifically for entropy-tolerant systems.

## First principle

> Tests for deterministic code assert exact outputs.
> Tests for LLM features assert behaviors within tolerance.
> The discipline is choosing the right tolerance — and proving it's not "anything passes."

## The oracle ladder

Five rungs, climb from CHEAPEST/STRICTEST to MOST EXPENSIVE/SUBJECTIVE. Always start at the bottom; only climb when the lower rung can't cover the case.

| Rung | What it checks | Cost | When to use |
|------|----------------|------|-------------|
| 1. **Exact match** | `output === expected` | ~free | Structured outputs (function calls, JSON with stable shape, classifications) |
| 2. **Schema validation** | Output matches JSON schema / type contract | ~free | Output shape matters; specific values vary |
| 3. **Outcome state** | Side effect produced the expected change (DB row, file written, tool called) | cheap | Agents, tool-use, RAG with concrete answers |
| 4. **LLM-as-judge** | A different model grades the output against a rubric | medium ($$$) | Subjective quality (helpfulness, tone, faithfulness) where no rule can decide |
| 5. **Human review** | Domain expert scores | expensive | Calibration of rung 4; high-stakes outputs; edge cases |

**Rule:** never reach for rung 4 before checking if rungs 1-3 can cover the case. Every rung up costs an order of magnitude more (latency, money, calibration effort) — and adds entropy.

See `references/oracle-ladder.md` for examples per rung and the climbing decision tree.

## LLM-as-judge discipline (when rung 4 is needed)

Without calibration, LLM-as-judge produces noise dressed as signal. Three non-negotiables:

1. **Calibrate against humans** — ≥20 human-graded cases, compute Spearman correlation against LLM-as-judge. Target ≥0.80. Below that, reject the judge configuration.
2. **Use a different model than the system under test** — same model judging itself produces false positives. Pair: GPT-4 generates → Claude judges. Or vice versa.
3. **Rubric, not free-form** — provide the judge a structured rubric (criteria + scale + examples) instead of "rate quality 1-10."

See `references/judge-calibration.md` for the full calibration recipe, rubric templates, and the "judge drift" monitoring pattern.

## Reference dataset principle

> 20 unambiguous cases drawn from real production failures beat 200 synthetic perfect cases.

The dataset is the bedrock. Without a reference set, every "improvement" is anecdote.

Structure:
```
.dw/eval/datasets/<feature-name>/
├── cases.jsonl           # input + expected (or rubric reference) per line
├── README.md             # provenance, sample size, when last reviewed
└── runs/<YYYY-MM-DD>.jsonl  # results of each eval run
```

See `references/reference-dataset.md` for case-design principles, sampling from production, and when to expand the set.

## RAG evaluation

Three orthogonal metrics — measure all three, not just one:

| Metric | What it measures | Tool |
|--------|-----------------|------|
| **Retrieval precision@k** | Of the top-K retrieved chunks, how many were relevant | Exact match against labeled ground-truth |
| **Answer faithfulness** | Does the answer cite only what the retrieved context supports? | LLM-as-judge with rubric |
| **Context utilization** | Did the answer USE the retrieved context, or hallucinate around it? | Heuristic + LLM-as-judge |

Precision alone misses hallucination. Faithfulness alone misses retrieval failure. Context utilization alone misses both. See `references/rag-metrics.md` for the full implementation.

## Agent / tool-use evaluation

Two questions distinguish good agent eval from bad:

### Question 1: outcome or trajectory?

| Approach | What it checks | Failure mode |
|----------|---------------|--------------|
| **Outcome-only** | Did the agent achieve the goal? Was the final state correct? | Misses "ghost actions" — agent did the right thing for the wrong reasons |
| **Trajectory** | Did the agent take the expected sequence of steps / tool calls? | Punishes legitimate creativity — agent solved it via a different valid path |

**Recommendation:** outcome-only with side-effect assertion as default. Trajectory match for cases where the path matters (e.g., "must call `get-user` before `update-user`").

### Question 2: which trajectory match mode?

When trajectory matching IS the right call, four modes are available:

- **Strict** — same tool calls, same order, same arguments. Use when both sequence and parameters are part of the contract.
- **Unordered** — same tool calls, any order. Use when concurrent calls are valid.
- **Subset** — actual trajectory contains a subset of reference calls. Use to enforce "don't exceed expected tool use" (frugality / cost).
- **Superset** — actual contains all reference calls plus possibly more. Use when specific tools are mandatory but extras are acceptable.

See `references/agent-eval.md` for examples and the decision tree.

## Required reading by context

| Doing what | Read |
|------------|------|
| Designing an eval suite for an AI feature | `references/oracle-ladder.md` (climb the ladder) |
| Using LLM-as-judge | `references/judge-calibration.md` (mandatory before relying on it) |
| Building / curating a reference dataset | `references/reference-dataset.md` |
| RAG-specific feature | `references/rag-metrics.md` |
| Agent / tool-use feature | `references/agent-eval.md` |

## Anti-patterns (will block in `/dw-review --code-only`)

- **LLM-as-judge without calibration evidence.** PR adds LLM-as-judge but the calibration Spearman score is missing or < 0.80. REJECTED.
- **Same-model judge.** Judge model is the same as the system under test. REJECTED unless explicitly documented (and even then, results are suspect).
- **Single-rung eval.** Feature ships with only LLM-as-judge; no rung 1-3 grounding. REJECTED — the cheap rungs catch the loud failures.
- **Synthetic-only dataset.** No traceable production-failure source for any case. REJECTED — confirm at least 20% of cases come from real user inputs.
- **"Looks good to me" QA.** No reference dataset, no metric, no rubric — just sampling output and calling it good. REJECTED.
- **Coverage as metric.** Quoting "we tested 50 prompts" without saying what was measured. The number is meaningless without the metric.

## Integration with dev-workflow commands

- `/dw-plan tasks`: when the PRD has an AI feature requirement, an eval-plan subtask is mandatory. The task references this skill's oracle ladder.
- `/dw-review --code-only`: AI feature PRs require a reference dataset + ≥2 oracle rungs (lower rungs FIRST). The constitution gate also applies — if the project has principles about AI feature reliability, they're enforced here.
- `/dw-qa --ai`: new mode (when this skill is bundled) — runs the reference dataset against the current implementation, logs to `QA/logs/ai/<feature>-<date>.jsonl`, computes precision@k / faithfulness / outcome accuracy per the feature type.
- `/dw-bugfix` when the bug is an AI failure mode (hallucination, tool misuse, classification error): adds the failing case to the reference dataset BEFORE fixing — the case is now a regression test forever.

## When the discipline bends

- **Prototype / spike phase**: skip calibration; document as "spike — eval added before merge to main."
- **Internal-only AI feature with low blast radius** (e.g., classifier for internal CRM tags): rung 1-3 only is fine; LLM-as-judge may be overkill.
- **Real-time features where eval can't run synchronously**: shadow-eval pattern — run the eval async on a sample of production traffic; alert on regression.

In all bend cases, document the deviation in the techspec / PR. "Skipped judge calibration because internal-only feature affecting <100 users" is fine; just say it.

## Why this approach

Two failure modes drive most AI feature regressions:

1. **No measurement** — team ships, suspects it's worse, can't prove it, debate.
2. **Wrong measurement** — team measures LLM-as-judge only, judge drifts with the model, scores rise while real quality falls.

The oracle ladder fixes both: forces measurement, forces ANCHORED measurement (lower rungs are deterministic; upper rungs are calibrated against them).

## Bottom line

> An AI feature without an eval suite is a feature you can't ship safely. An eval suite without calibration is a number you can't trust. Build the dataset from real failures, climb the ladder from cheap to expensive, calibrate the judge against humans, and re-run before every model swap. The discipline is small; the absence of it is one of the largest sources of "we shipped and don't know if it's worse" experiences in the industry.

# RAG evaluation — three orthogonal metrics

Retrieval-augmented generation (RAG) has three failure modes, each requiring its own metric. Measure all three. Measuring only one creates blindspots.

## The three metrics

### 1. Retrieval precision@k

**What it measures:** of the top-K chunks retrieved, how many were RELEVANT to the user's query?

**How to compute:**

```python
def precision_at_k(retrieved_chunk_ids, relevant_chunk_ids, k=5):
    top_k = retrieved_chunk_ids[:k]
    relevant_in_top_k = sum(1 for cid in top_k if cid in relevant_chunk_ids)
    return relevant_in_top_k / k
```

**Reference data needed:** for each test case, the human-labeled set of "chunks that should have been retrieved" — the ground truth.

**Target:** depends on K. For k=5, target precision >0.6 (3 of 5 chunks relevant). For k=10, target >0.5.

**What it catches:** retrieval is bringing back junk. Chunk embeddings are wrong, the index is stale, the query rewriting is broken.

**What it misses:** the LLM may still produce a great answer even from imperfect retrieval — or a hallucinated answer despite perfect retrieval. Pair with metrics #2 and #3.

### 2. Answer faithfulness

**What it measures:** does the answer make claims that are SUPPORTED by the retrieved context? Or does it fabricate?

**How to compute (rung-4 LLM-as-judge with rubric):**

The judge sees: user question + retrieved context + generated answer. Scores 1-5 per the faithfulness rubric (see `judge-calibration.md` for an example).

**Reference data needed:** the retrieved context (saved from the run) and the answer. No ground-truth answer required — the judge checks claim-by-claim against the context.

**Target:** 80% of cases score ≥4 on the 1-5 scale.

**What it catches:** hallucination — the answer says things the context didn't support. This is the #1 failure mode in production RAG.

**What it misses:** the answer might be faithful to the retrieved context but the retrieved context might be WRONG. Pair with metric #1.

### 3. Context utilization

**What it measures:** did the answer USE the retrieved context, or ignore it and produce a generic / parametric-memory response?

**How to compute (heuristic + LLM-as-judge hybrid):**

Heuristic part — n-gram overlap or semantic similarity:
```python
def context_overlap(answer, context, n=3):
    answer_ngrams = set(ngrams(answer, n))
    context_ngrams = set(ngrams(context, n))
    if not answer_ngrams:
        return 0
    return len(answer_ngrams & context_ngrams) / len(answer_ngrams)
```

Judge part — ask if the answer would change materially without the context:
> "If the retrieved context were removed, would the answer be substantially different? 1 = same as without context (didn't use it), 5 = fully context-grounded."

**Target:** 70%+ overlap on substantive answers; judge score ≥4 on 80% of cases.

**What it catches:** the answer is faithful to the context (metric #2 passes) but ignores it — the model used its parametric memory instead. This means retrieval is doing nothing.

**What it misses:** the answer might use the context but cite it incorrectly. Pair with metric #2.

## Why all three are needed

| Metric | Detects | Misses |
|--------|---------|--------|
| Retrieval precision@k | Junk in retrieval | Faithfulness; utilization |
| Answer faithfulness | Hallucination | Retrieval quality; whether context was used |
| Context utilization | Ignoring retrieval | Hallucination beyond context; retrieval quality |

A RAG system can fail in all three independent ways. Measuring only one creates blind spots in the other two.

## Combined metric example

```python
def evaluate_rag(case):
    retrieved = retrieve(case.query)
    answer = generate(case.query, retrieved)

    return {
        'precision_at_5': precision_at_k(
            [c.id for c in retrieved],
            case.relevant_chunk_ids,
            k=5
        ),
        'faithfulness': llm_judge_faithfulness(
            query=case.query,
            context=retrieved,
            answer=answer
        ),
        'context_utilization_overlap': context_overlap(answer, retrieved),
        'context_utilization_judge': llm_judge_utilization(
            query=case.query,
            context=retrieved,
            answer=answer
        ),
    }
```

Aggregate per-case scores into the per-run summary:

```
Run 2026-05-12:
  precision@5:          0.68  (target >0.6) ✓
  faithfulness ≥4:      83%   (target >80%) ✓
  context utilization:  72%   (target >70%) ✓
  Overall: PASS
```

## Common RAG failure modes

| Symptom | Likely metric that catches it |
|---------|------------------------------|
| User says "the bot is making stuff up" | Faithfulness |
| User says "the bot didn't see my documents" | Context utilization (or retrieval precision) |
| User says "the bot is bad at finding things" | Retrieval precision@k |
| User says "the answer is correct but ignores recent updates" | Retrieval recall (precision's partner — different metric) |
| User says "the bot gives the same generic answer no matter what I ask" | Context utilization |
| User says "the bot says the doc says X but it doesn't" | Faithfulness |

The metric points at the layer to fix. Without it, debugging is guesswork.

## Retrieval recall (the fourth metric, conditional)

Precision asks "of what we retrieved, how much was good?" Recall asks "of what was good, how much did we retrieve?"

In production RAG with many candidate chunks, recall is often the limiting factor — the right chunk exists in the index but doesn't surface.

Compute:
```python
def recall_at_k(retrieved_chunk_ids, relevant_chunk_ids, k=5):
    top_k = set(retrieved_chunk_ids[:k])
    return len(top_k & set(relevant_chunk_ids)) / len(relevant_chunk_ids)
```

Track recall when:
- The corpus is large (>1000 chunks per query domain).
- Users report "the bot can't find things that exist in our docs."
- You're tuning the retrieval pipeline (chunking strategy, embedding model, search algorithm).

Skip recall when:
- The corpus is small (top-K = ~10% of the corpus; recall is high by default).
- Precision is the dominant problem.

## Dataset structure for RAG

```json
{
  "id": "rag-case-001",
  "query": "What's our PTO policy for sabbatical years?",
  "expected": {
    "relevant_chunk_ids": ["chunk-policy-pto-2024", "chunk-policy-sabbatical"],
    "expected_answer_themes": ["accrual rate", "carryover limits", "sabbatical exception"],
    "should_cite": true
  },
  "metadata": {
    "source": "production-2026-04-12-support-thread-S-892",
    "difficulty": "medium",
    "tags": ["pto-policy", "sabbatical", "rare-query"]
  }
}
```

The `relevant_chunk_ids` field requires human labeling — domain expert reviews the corpus, identifies which chunks SHOULD surface for that query.

## Anti-patterns

- **Measuring only one metric** (usually faithfulness via LLM-as-judge) → blind to retrieval and utilization failures.
- **No human-labeled relevance** → can't compute precision/recall.
- **Treating retrieval and generation as one black box** → can't tell which layer regressed.
- **Eval set drawn only from "easy" queries** → metrics are good in test, terrible in production.
- **Ignoring recent-information bias** (RAG must use retrieval; parametric memory is stale) → context utilization metric catches this.

## Tooling

- **ragas** (open source) implements precision, recall, faithfulness, and other RAG metrics with LLM judges. Use as reference implementation.
- **Custom implementation** is straightforward — the metrics above are <100 lines of Python each.
- **LangSmith / Weights & Biases** wrap eval runs with tracking but don't replace the core metrics.

The discipline isn't tool choice; it's measuring all three orthogonal dimensions every run.

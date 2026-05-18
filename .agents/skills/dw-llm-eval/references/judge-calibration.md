# LLM-as-judge calibration — how to make rung 4 mean something

LLM-as-judge sounds simple: a model grades the output. In practice, without calibration it produces NUMBERS WITHOUT SIGNAL — judge scores drift with the model, with rubric phrasing, with prompt minutiae. You read "judge says 4.2 average" and have no idea if that means the system is good.

Calibration anchors the judge to human assessment. After calibration, a judge score has meaning. Before, it doesn't.

## The three non-negotiables

### 1. Calibrate against ≥20 human-graded cases

Process:

1. Sample ≥20 cases from the reference dataset (or representative production traffic).
2. Have ≥1 domain expert grade each case using the same rubric the judge will use. Multiple humans per case is better (inter-rater agreement is useful signal).
3. Run the judge against the same cases.
4. Compute Spearman rank correlation between human scores and judge scores.

**Target:** Spearman ≥0.80.
**Acceptable:** 0.70-0.80 with documented rationale (e.g., "subjective tone judgments inherently noisy").
**Reject:** <0.70. The judge is not measuring what you think it's measuring.

### 2. Use a different model than the system under test

A model judging its own output produces false positives. The judge agrees with itself even when wrong because it shares the same biases and blind spots.

Pairing examples:
- System: GPT-4 → Judge: Claude Opus.
- System: Claude Sonnet → Judge: GPT-4o.
- System: Gemini → Judge: Claude.

If both system and judge MUST be from the same provider, at minimum use different model sizes (Sonnet judges Opus output, not vice versa).

### 3. Structured rubric, not free-form scoring

"Rate this answer 1-10" → noise. Different runs give different scores; different humans disagree wildly; the score has no anchor.

Structured rubric: ≥3 criteria, each with a defined scale and an example per score point.

Example rubric for FAITHFULNESS (RAG):

```markdown
# Faithfulness rubric (1-5 scale)

Score each answer against the retrieved context. A faithful answer makes claims supported by the context; an unfaithful one fabricates or extrapolates.

## 1 — Severely unfaithful
The answer contains claims that contradict the context, or fabricates facts not present in any chunk. Example: context says "Q3 revenue was $1.2M"; answer says "Q3 revenue exceeded $5M."

## 2 — Mostly unfaithful
The answer mixes context-supported and fabricated claims, where the fabrication is meaningful. Example: cites a study that wasn't in the context.

## 3 — Mixed
Half the answer is grounded; half is reasonable inference or generalization beyond the context. Example: context describes the API; answer adds advice not derivable from context.

## 4 — Mostly faithful
All claims are supported by context; minor paraphrasing or summarization without distortion. Example: rewords a passage accurately.

## 5 — Strictly faithful
Every claim is directly traceable to a specific chunk; no information added beyond what context contains. Example: quotes-with-attribution style.
```

Provide this rubric INSIDE the judge prompt. Free-form is forbidden.

## The calibration loop

```
1. Sample 20-30 cases for calibration set.
2. Human-grade them blind (without seeing other graders or judge).
3. Run judge with rubric.
4. Compute Spearman vs human scores.
5. If <0.70:
   - Examine disagreements: where does judge consistently miss?
   - Refine rubric: more specific scale, more examples, narrower scope.
   - OR switch judge model: try a different vendor/size.
   - Re-run step 3-4.
6. If 0.70-0.80: document the noise floor; accept with caveats.
7. If ≥0.80: judge is calibrated. Save the rubric + judge config in version control.
```

Calibration is one-time-per-config but RECURRING-PER-MODEL-CHANGE. Every model swap (you upgrade GPT-4 to GPT-5; vendor deprecates Opus 4.7) invalidates the calibration. Re-calibrate.

## Judge drift monitoring

After deployment:
- Re-run calibration set monthly.
- Plot Spearman over time.
- Alert if Spearman drops below 0.75 between calibration runs — the judge has drifted (model update, rubric got stale, traffic distribution shifted).

```
.dw/eval/judges/<feature>/
├── rubric.md                       # the rubric, version-controlled
├── calibration-2026-05-12.jsonl    # 20+ cases with human + judge scores
├── spearman-2026-05-12.txt         # 0.84
├── calibration-2026-08-12.jsonl    # quarterly re-calibration
└── spearman-2026-08-12.txt         # 0.81
```

## Rubric design patterns

### DO

- **3-5 criteria** per rubric (one for each dimension you care about: faithfulness, completeness, tone, format, ...).
- **1-5 scale** with anchored descriptions per point (not 1-10 — too granular for reliable agreement).
- **Example per score point** showing the kind of output that earns that score.
- **Explicit "what to ignore"** — e.g., "ignore minor grammar; score on substance."

### DON'T

- Single-criterion "quality" score — too vague to calibrate.
- 1-100 scales — humans can't reliably distinguish 73 from 76.
- Rubrics longer than 500 words — the judge skips and lazy-scores.
- "Holistic" scoring without breakdown — opaque to debug.

## Multi-criterion rubrics

For complex outputs (RAG, agents), one number rarely captures quality. Use per-criterion scores:

```json
{
  "faithfulness": 4,
  "completeness": 3,
  "tone": 5,
  "format": 5,
  "overall": null
}
```

Aggregate as needed downstream (weighted average, minimum, "all must be ≥3"). Don't have the judge compute the aggregate — bias compounds.

## Anti-patterns

- **Judge with no rubric.** "Rate this 1-10." Numbers, no signal.
- **Judge is the system being tested.** False positives baked in.
- **No calibration evidence in PR.** "We added LLM-as-judge" — okay, what's the Spearman?
- **Rubric stuffed with all criteria in one prompt** → judge lazy-scores. Split into criterion-per-call if needed.
- **Calibration done once, never revisited.** Model upgrades silently break it. Re-calibrate monthly or per model swap.
- **Judge scoring its own scoring.** Recursive trust collapse.

## Bias to watch

LLM judges have characteristic biases:

- **Length bias** — longer outputs score higher even when shorter is better. Normalize length in the rubric.
- **Self-similarity bias** — judges rate outputs that resemble their own writing higher. Cross-model pairing helps.
- **Position bias** (in comparative judging) — first item often wins. Randomize order, run both A/B and B/A.
- **Recency bias** — last item in context is overweighted. Vary order.
- **Sycophancy** — judges agree with strongly-stated input even when wrong. Frame the judge prompt neutrally.

Document which biases you tested for in the calibration write-up.

## Cost discipline

LLM-as-judge can dominate eval costs. At $0.01-$0.10 per judgment, 100 cases × 4 rubric criteria × monthly = real money.

Optimizations (in order of impact):
1. Run judge against SAMPLES, not the whole dataset every time. 50 random cases weekly catches regression.
2. Use the cheapest model that maintains Spearman ≥0.80. GPT-4 mini may calibrate as well as GPT-4 for your rubric.
3. Batch judge calls when the API supports it.
4. Cache judge results per (input, output, rubric-version) tuple — same eval run shouldn't pay twice.
5. Skip judge for cases where rungs 1-3 already failed — they're broken; no point asking subjective quality.

## When NOT to use LLM-as-judge

- The output has a deterministic correct answer. Use rung 1 or 2.
- The output has a measurable side effect. Use rung 3.
- The team won't budget for calibration. The judge will produce noise.
- The rubric can't be written in <500 words. The criterion is too vague.

A poorly-calibrated judge is worse than no judge: it gives false confidence. Better to ship with "tested manually by domain expert on 20 cases" than with "judge score 4.1" that means nothing.

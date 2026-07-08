# Self-Evaluation Rubric — five axes, evidence required

A structured way for an agent to rate its own output **before** handing it over, without waiting for user
feedback. Used by `dw-review-rigor` (score the review itself) and cited by `dw-cli-run` (the 0–10 delivery
gate applies the same evidence discipline). Adapted from the ECC `agent-self-evaluation` skill.

## The five axes (score each 1–5)

| Axis | Question | What a low score catches |
|---|---|---|
| **Accuracy** | Are the facts, claims, and code correct? | Hallucinated APIs, wrong file paths, syntax errors, unverified assertions |
| **Completeness** | Did it cover everything the request asked? | Missed edge cases, skipped subtasks, unhandled error paths |
| **Clarity** | Is it understandable and well-structured? | Confusing explanations, jargon, rambling, missing context |
| **Actionability** | Can the user act on it immediately? | Vague suggestions, missing steps, "you should X" without showing how |
| **Conciseness** | Is it the minimum needed to be complete? | Redundancy, over-explanation, filler, restating the question |

Scale: **5** = no reasonable improvement · **4** = minor nits · **3** = one notable weakness · **2** = a clear
gap that hurts usability · **1** = fundamentally misses the request.

## The one hard rule: evidence for every score below 5

A score under 5 **must cite the specific gap** — quote the line, name the missing case, point at the
redundancy. "Completeness: 3 — doesn't handle the empty-array case in `parse()`" is a real score;
"Completeness: 3 — could be more thorough" is hand-waving and not allowed.

## What to do with the scores

1. Score each axis independently (don't average away a real weakness).
2. If any axis is **≤ 3**: fix it now when the fix is under ~30 seconds; otherwise flag it explicitly in the
   output so the user knows the gap exists.
3. Keep the self-score short — a few lines, not a second report.

## Anti-patterns

- **"Everything is a 5"** with no evidence — the most common failure; it means you didn't actually evaluate.
- **Over-penalizing scope** — grade only what was requested, not scope you invented.
- **Re-litigating decisions** — evaluate execution quality, not whether a settled decision was right.
- **Personal preference as a gap** — a nit the linter doesn't enforce and no rule backs is not a real deduction.

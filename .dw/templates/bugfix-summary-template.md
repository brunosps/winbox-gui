---
schema_version: "1.0"
slug: ""
created: ""
status: "Fixed | In Review | QA Pending | Reverted"
severity: "Low | Medium | High"
related_concerns: []
---

# Bugfix Summary — {{NNN}}-{{slug}}

One-page record of a bugfix. Sibling files in this directory:

- `TASK.md` — the original triage, clarification answers, and the fix plan that ran
- `fix-report.md` — verification evidence (`dw-verify` PASS output, reproduction proof, regression test run)
- `review/` — populated by `/dw-review --bugfix {{NNN}}-{{slug}}`
- `QA/` — populated by `/dw-qa --bugfix {{NNN}}-{{slug}}` (when applicable)

## Symptom

What the user observed. Quote the original bug description verbatim; do not paraphrase.

> _"…"_

## Root Cause

What was actually broken, in a single sentence. Not the symptom — the cause.

_…_

## Resolution

What changed, in 2-4 bullets. File paths, not snippets.

- _change 1_
- _change 2_

## Files Touched

Full list, including tests. ≤5 — if more, the safety valve should have escalated to `/dw-plan`.

| Path | Change |
|------|--------|
| `src/foo/bar.ts` | _surgical fix to X_ |
| `src/foo/bar.test.ts` | _regression test added_ |

## Verification

How the fix was proven, beyond "tests pass".

- **Reproduction before fix:** _step that triggered the bug, captured_
- **Reproduction after fix:** _same step, now passes_
- **Regression test:** _name + path_
- **Verify report:** `fix-report.md`

## Related

- **Concerns touched:** _refs from `.dw/rules/concerns.md` if the fix landed in a flagged area_
- **Adjacent bugfixes:** _slugs of prior fixes in the same module, if any_
- **PRD context:** _if the bug surfaced inside a feature in flight, link to its PRD path_

## Followups

Open loops this fix surfaced but did not resolve. Add to `.dw/STATE.md` Open-Loops on close.

- _none_

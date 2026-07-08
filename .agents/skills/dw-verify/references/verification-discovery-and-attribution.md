## Project-Specific Verification Commands

dev-workflow does not hardcode a verification command. Discover it from the project:

1. Check `.dw/rules/` for a documented verify command.
2. Read `package.json` scripts: prefer `verify`, `check`, `ci`, `test`, in that order.
3. Check for `Makefile`/`make verify`, `pyproject.toml`/`just verify`, etc.
4. If none is explicit, run the documented test + lint + build sequence.

If no verification command exists for the project, state that explicitly in the Verification Report and avoid completion language.

## Integration With Other dev-workflow Commands

This skill is invoked transparently from:

- `/dw-run` — before committing the task's changes
- `/dw-run` — before Level 2 review and before declaring the plan complete
- `/dw-qa --fix` — before marking a bug as resolved in `QA/bugs.md`
- `/dw-bugfix` — before claiming the bug is fixed (original symptom no longer reproduces)
- `/dw-review --code-only` — before emitting an APPROVED verdict
- `/dw-generate-pr` — blocks PR creation if the session has no passing VERIFICATION REPORT post-last-edit

Callers should mention this skill in their "Skills Complementares" section so the user sees the dependency.

## Inspired by

Ported from Compozy's `cy-final-verify` skill (`/tmp/compozy/.agents/skills/cy-final-verify/SKILL.md`). Adapted for the dev-workflow context:

- Project-agnostic verification discovery (Compozy assumes `make verify`; dev-workflow scans `package.json`/Makefile/`.dw/rules/`).
- Integration table maps to dev-workflow command names instead of Compozy phases.

Credit: Compozy project (https://github.com/compozy/compozy).

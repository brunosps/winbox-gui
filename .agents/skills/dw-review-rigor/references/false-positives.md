# False Positives — patterns LLM reviewers habitually mis-flag

Loaded on demand by the Pre-Report Gate. Skip these unless you have evidence **specific to this codebase**
(a failing test, a real caller that doesn't guard, a project rule that forbids it). Flagging them anyway is
the most common way an AI review drowns real findings in noise.

| Tempting finding | Why it's usually wrong |
|---|---|
| "Add error handling here" | The error path is already handled by the caller or the framework — Express/Koa middleware, a React error boundary, a top-level `catch`, or a job runner that retries. Trace one level up before flagging. |
| "Missing input validation" | The function is internal and every caller already validated at the boundary. Validation belongs at the trust boundary, not on every hop. |
| "Magic number" | Well-known constants are self-documenting: HTTP codes (200/404/500), `1000`/`60`/`24`/`1024`, array index `0`/`-1`, `100` for percent. Only flag opaque domain numbers with no name. |
| "Function too long" | Length ≠ complexity. Exhaustive `switch`, config/lookup objects, test tables, and generated code are long *and* simple. Flag high branching/nesting, not line count. |
| "Missing JSDoc/docstring" | Internal helpers whose name and signature are self-describing don't need docs. Reserve for public API or non-obvious contracts. |
| "Possible null dereference" | A preceding line already narrowed the type or an `if` guard is in scope. Read the few lines above before asserting. |
| "Use `const` / style nit" | The linter/formatter already enforces this (Rule 4). It is a linter task, not a review finding. |
| "This could be a race condition" | Only real if there is actual shared mutable state across concurrent paths. A single-threaded request handler touching a local variable is not a race. |
| "Add a test for this" | Valid only if the behavior is untested AND load-bearing. Don't demand tests for trivial getters or framework glue. |
| "Extract this into a helper (DRY)" | Two similar lines are not duplication worth an abstraction. Premature extraction couples unrelated call sites — see `dw-minimalism`. |

**Rule of thumb:** if the "fix" would be rejected by a competent maintainer as busywork, it is a false
positive. When unsure whether a pattern is intentional, apply Rule 3 (verify intent) before writing it.

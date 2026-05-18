---
name: dw-simplification
description: Use when simplifying code. Chesterton's Fence (WHY first), behavior-preserving refactor, complexity metrics, deep-modules analysis. Triggers from /dw-review and /dw-brainstorm refactor-audit.
allowed-tools:
  - Read
  - Edit
  - Bash
  - Grep
  - Glob
---

# dw-simplification

Behavioral discipline for simplifying code without breaking it. The trap of refactoring is removing something that "looked unused" but was load-bearing for an edge case nobody documented. This skill enforces a protocol that prevents that class of regression.

## When to Use

Read this skill when:

- `/dw-review --code-only` flagged a complexity issue (deep nesting, long function, duplication).
- `/dw-brainstorm` dispatched its **refactor-audit** mode and proposed a simplification target.
- The user explicitly asks to "clean this up" / "simplify X".
- During `/dw-run` if the implementation accidentally produced complex code that wants pre-commit cleanup.

Do NOT use when:

- The complexity is intentional (e.g., performance hot path with optimization comments — leave alone).
- You're simplifying code you didn't just write or read recently — scope creep.
- Tests are missing for the area — without tests, "preserve behavior exactly" is unverifiable. Add tests first, then simplify.

## The Five Rules

### 1. Chesterton's Fence — understand BEFORE changing

Before removing or rewriting any code, answer three questions:

1. **What does it actually do?** Run/trace it; don't guess from the name.
2. **Why was it added?** Check `git log --follow <file> --oneline` and look at the introducing commit's message + PR. Often there's a reason.
3. **What breaks if it's gone?** Search for callers. Run the test suite to find what changes.

If you can't answer all three, you're not ready to simplify. Get answers first.

Concrete cases where Chesterton's Fence saves you:
- A "redundant" early return that handles a race condition.
- A "useless" type cast that fixes a compiler bug on a specific platform.
- A "duplicated" check that exists for clarity at the API boundary.
- A "dead" branch that runs only with a feature flag enabled in production.

### 2. Preserve behavior exactly

The test for "did I simplify or did I change?" is: do the existing tests still pass without modification? If yes, behavior is preserved. If you needed to update tests, you changed behavior — that's a refactor, not a simplification.

When tests are inadequate to confirm behavior preservation, write tests FIRST that document current behavior, then simplify against them. This is the test characterization technique.

### 3. Follow project conventions

Match what the project already does. If files use 2-space indent and arrow functions, your simplified version uses 2-space indent and arrow functions — even if you'd prefer 4-space and `function`. Conventions matter more than personal preference; consistency is itself a form of clarity.

Project conventions live in `.dw/rules/<module>.md` (from `/dw-analyze-project`). Read first.

### 4. Prefer clarity over cleverness

Rules of thumb:

- A line of code that takes 30 seconds to understand is worse than three lines that take 5 seconds each.
- A "neat trick" with comments is uglier than the boring obvious code without comments.
- Optimization that costs readability needs measurement (`dw-performance` skill territory) — without numbers, prefer the readable version.
- "Magic" that requires knowing a language feature most teammates haven't used is anti-clarity. Use it only when the alternative is genuinely worse.

### 5. Scope to recent changes

Don't go simplifying old code while you're "in there". The risk-to-benefit ratio of changing 5-year-old battle-tested code is bad. Limit simplification to:

- Code you just wrote in this session.
- Code your current task touches.
- Code with explicit recent breakage (e.g., a bug just fixed that revealed bad structure).

Out of scope: any other code, no matter how ugly. Open a task / ADR for those instead of fixing in-flight.

## Pattern recognition

Frequent simplification targets — but apply Rules 1-5 before acting:

| Smell | Healthy refactor | When NOT to refactor |
|-------|------------------|----------------------|
| Deep nesting (4+ levels) | Early returns / guard clauses | When the nesting maps to a real domain hierarchy (e.g., visitor pattern) |
| Long function (>50 lines) | Extract method | When the "natural" extraction would produce 3-4 single-call helpers — you're trading complexity for indirection |
| Generic name (`data`, `info`, `helper`) | Rename to specific | When the function genuinely processes generic input (e.g., a serializer middleware) |
| Duplication (same 5 lines twice) | Extract to function/constant | When the duplication is by design (e.g., independent business rules that may diverge) |
| `if/elif/else` chain (5+ branches) | Strategy pattern / lookup table | When the branches are naturally exclusive states with branch-specific logic |

## Rule of 500 — automate large refactors

If a simplification touches >500 lines, **don't do it manually**. Use:

- AST-based codemods (`jscodeshift` for JS/TS, `libcst` for Python, Roslyn for C#, `rust-analyzer` for Rust).
- IDE-driven refactoring tools (Rename Symbol, Extract Method, Inline Variable).
- Search-and-replace ONLY when the pattern is unambiguous and verifiable.

Manual edits across hundreds of lines are how subtle bugs creep in.

## Verification protocol

Before committing the simplification:

```
1. Lint passes:    pnpm lint  / ruff check  / dotnet format --verify-no-changes  / cargo clippy
2. Tests pass:     pnpm test  / pytest      / dotnet test                         / cargo test
3. Build passes:   pnpm tsc --noEmit / mypy / dotnet build                         / cargo check
```

All three GREEN. If any is RED, the simplification broke something — revert or fix.

For changes that altered cyclomatic complexity, optionally run a complexity analyzer to confirm the metric actually improved (some "simplifications" make code more complex by spreading it).

## How `dw-code-review` uses this

In the formal Level 3 review (post-Level 2 chain from `dw-review-implementation`), code-review flags complexity issues using the patterns above. Each flagged issue references this skill: "consider simplifying via guard clauses; apply Chesterton's Fence — verify why the nested check exists before flattening."

## How `/dw-brainstorm` refactor-audit mode uses this

The refactor-audit mode dispatched by `/dw-brainstorm` catalogs code smells (Fowler vocabulary) AND runs the deep-modules analysis (`references/deep-modules.md`) against the target area. For each smell or shallow-module flag, the proposed refactor cites:

1. Which simplification rule applies (early return / extract method / lookup table / etc.).
2. Whether Chesterton's Fence concerns block the refactor (existing tests inadequate? no recent commits explaining the structure? → flag as YELLOW, don't act).
3. Whether the deep-modules test points the other way (some "code smells" are actually deep-module wrappers that absorb complexity; the fix is to make the wrapper deeper, not to flatten it).

## Anti-patterns

1. Simplifying code you didn't read carefully. The "obvious dead code" is often someone's hard-won bug fix.
2. Skipping the test gate "because the change is small". Small changes break things too.
3. Changing whitespace + naming + structure in one commit. Atomic: each commit one kind of change.
4. Refactoring while touching unrelated code (mix-in scope creep). Open a separate task.
5. Personal-style refactors disguised as simplification. Project conventions > your preferences.

## References

- `references/chestertons-fence.md` — the protocol in detail; case studies of "obvious-but-wrong" removals.
- `references/complexity-metrics.md` — when each metric (cyclomatic, cognitive, depth, fanout) actually matters; how to measure cheaply.
- `references/behavior-preserving.md` — characterization tests, refactor with test gate, rollback patterns, codemod tooling per language.
- `references/deep-modules.md` — high-leverage modules behind small interfaces; deletion test, locality, leverage, seam, adapter diagnostic; anti-patterns (shallow wrapper, god-module). Invoked by `/dw-brainstorm` refactor-audit mode.

## Inspired by

Adapted from [`addyosmani/agent-skills/code-simplification`](https://github.com/addyosmani/agent-skills) by Addy Osmani (MIT license). Core principles (Chesterton's Fence, behavior preservation, scope discipline, Rule of 500) preserved. dev-workflow integration: invoked by `dw-code-review` and `/dw-brainstorm` refactor-audit mode via Complementary Skills.

The deep-modules reference is adapted from [`mattpocock/skills/improve-codebase-architecture`](https://github.com/mattpocock/skills/tree/main/improve-codebase-architecture) by Matt Pocock (MIT license). Core framing (deep modules = high leverage at small interface, deletion test, shallow-wrapper anti-pattern) preserved; paths and integration points rebased on dev-workflow conventions.

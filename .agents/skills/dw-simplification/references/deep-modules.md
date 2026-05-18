# Deep modules — high leverage at small interface

A **deep module** absorbs complexity behind a narrow public surface. Its callers see a small, stable API and get a lot of work done per call. A **shallow module** is the opposite — small interface AND small implementation, so it adds an indirection without absorbing any complexity. Or worse: a god-module, deep implementation BUT huge interface, where the surface itself is the complexity.

The refactor-audit mode of `/dw-brainstorm` checks every candidate module against this framing alongside the Fowler smell taxonomy. A "code smell" can mislead if the construct is actually a deep wrapper doing its job correctly.

## The premise

Good abstractions hide complexity. The metric isn't "how short is the code?" — it's "how much complexity does the interface eliminate per unit of public surface area?"

- **Deep module**: 1000 lines of implementation, 3 public functions. Caller does NOT need to know the 1000 lines.
- **Shallow module**: 50 lines of implementation, 8 public functions. Caller has to learn all 8 to use the 50 productively.
- **God-module**: 5000 lines of implementation, 60 public functions. Surface area is itself unmanageable.

The deep version wins not because it's shorter overall (it's longer), but because the cost of using it is concentrated in the implementation, not spread to every caller.

## Diagnostic checklist

When the refactor-audit mode evaluates a module, ask these in order:

### 1. Deletion test

> If this module were deleted, how many call sites would break?

- **0–1 callers**: the module isn't pulling its weight. Inline it; the abstraction is shallower than the call sites' real shape. Mark this as `low` priority candidate for removal.
- **2–3 callers**: borderline. Look at what each caller does with the result — if they immediately wrap/unwrap, the module is shallow. If they consume the result directly, leave it alone.
- **4+ callers**: the module is at least somewhat load-bearing. Move to next checks.

### 2. Locality test

> Does the caller pass everything the module needs, OR does the module reach into globals / shared state to fill gaps?

A deep module is **self-contained**: callers pass in arguments, get back results. A module that reads from `process.env`, a shared singleton, or implicit thread-local state has a deeper coupling than its interface admits — the caller has to know about the hidden inputs too.

If the module reaches into shared state, its **effective interface** is bigger than what the function signature shows. That's a hidden shallowness — the public surface lies about its real cost.

### 3. Leverage test

> LOC saved per caller × number of callers — what's the multiplicative payoff?

Crude formula: if each caller would otherwise write ~N lines of inline code to replace this module's call, and there are M callers, the module's leverage is N × M. A module whose leverage is < 2× its own LOC is shallow — it's not absorbing enough complexity to justify itself.

### 4. Seam test

> How often does the public interface change relative to the implementation?

A stable public interface with frequently-changing implementation is the **ideal seam** — callers stay put while internals evolve. An interface that changes every few months means callers track it; the abstraction is leaking.

Check `git log --oneline -- <module>` against `git log --oneline -- <module-public-surface-file>`. If they move together, the seam is weak.

### 5. Adapter test

> Does this module mediate between two foreign vocabularies?

The best deep modules are **adapters** — they translate between two domains that have different naming conventions, error semantics, or state models. A module that doesn't mediate between domains is harder to justify as deep; it's probably either a passthrough (shallow) or just a piece of the same domain stretched across files (cohesion issue).

## Anti-patterns

### Shallow wrapper

```ts
// shallow — passthrough with one line of "work"
export function getUserName(id: string) {
  return db.users.findById(id).name;
}
```

The wrapper saves the caller 8 characters (`.name`) and forces them to learn a new function name. The leverage is < 1. Inline it.

When to keep a one-liner: it's an **adapter** that locks down ONE meaning of a query. If `getUserName` exists specifically because there are 4 candidate fields (`name`, `displayName`, `username`, `fullName`) and the team's canonical answer is "use `name`", the function IS pulling its weight by serving as the project's vocabulary anchor. Document the WHY in the function body.

### God-module

A class with 60 public methods is shallow even if each method is 100 lines, because the caller has to learn 60 concepts. Split by **role**: what is the smallest subset of methods a caller typically uses together? Each subset becomes a deep module of its own.

### Implementation-leak abstraction

A module that returns its internal representation directly (`return this.cache;`) makes callers depend on the internals. The first time the cache shape changes, every caller breaks. Wrap or copy on egress.

## When refactor-audit raises a smell, also check deep-modules

The combined verdict:

| Fowler smell | Deep-modules check | Action |
|--------------|--------------------|--------|
| Long Method | Is the method INSIDE a deep module that callers don't see? | If yes, leave it — internal complexity is OK in deep modules. If no, extract. |
| Large Class | Is the public surface narrow despite the line count? | If yes, leave it — that's deep. If no, split by role. |
| Long Parameter List | Does each parameter represent a real choice the caller makes? | If yes, the deep module is forcing necessary configuration; introduce a config object. If parameters are derivable from each other, reduce. |
| Feature Envy | Is the module a deep adapter between domains? | If yes, the "envy" is the mediation — keep it. If no, move the logic closer to its data. |
| Middle Man | Does the middle man absorb any complexity? | If no, delete the middle man. If yes, document the why (it's a deep wrapper). |

A flat "this is too long, extract method" recommendation is shallow analysis. Combining Fowler + deep-modules surfaces when the smell is actually a feature.

## Output for refactor-audit findings

When the audit flags a module as a shallow-wrapper or god-module candidate, the finding entry adds a `Deep-modules: <verdict>` line:

```markdown
### P1 — Shallow Wrapper
**Files:** src/lib/getUserName.ts
**Symptom:** 1-line passthrough wrapper around `db.users.findById(id).name`; 2 callers.
**Deep-modules:** SHALLOW — fails deletion test (only 2 callers) AND leverage test (0 LOC saved per call).
**Refactor:** Inline at both call sites. If the wrapper exists for vocabulary reasons, document and keep — otherwise remove.
**Risk:** Low.
```

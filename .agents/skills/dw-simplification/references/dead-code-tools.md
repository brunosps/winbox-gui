# Dead-code detection tools (find candidates; then apply Chesterton's Fence)

Detection tools surface **candidates** — they do not know about reflection, dynamic imports, framework
entry points, or external consumers of a public API. Every hit is a SAFE-tier candidate to verify, not an
automatic deletion. Run the gate (lint + tests + build) after each batch.

## JavaScript / TypeScript
- `npx knip` — unused files, exports, and dependencies in one pass (the broadest single tool).
- `npx depcheck` — unused and missing npm dependencies.
- `npx ts-prune` — unused TypeScript exports.
- `npx eslint . --report-unused-disable-directives` — stale `eslint-disable` comments.

## Other ecosystems
- **Python:** `vulture`, `ruff` (`F401` unused imports), `deptry` (unused dependencies).
- **Go:** `deadcode` (golang.org/x/tools/cmd/deadcode), `staticcheck` (`U1000`).
- **Rust:** `cargo +nightly udeps` (unused deps), the compiler's own `dead_code` warnings.
- **C# / .NET:** Roslyn analyzers (`IDE0051` unused private member, `IDE0059` unnecessary assignment).

## Discipline
1. Run the tool; treat every result as a **SAFE-tier candidate**, not a confirmed removal.
2. For each candidate, apply Chesterton's Fence (Rule 1) — dynamic/reflective/plugin use won't appear in static analysis.
3. Remove in batches by category (deps → exports → files); run the gate after each batch; commit per batch.
4. Anything the tool flags that turns out to be load-bearing → add a comment explaining why, so the next pass doesn't re-flag it.

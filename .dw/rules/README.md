# Project Rules

This directory contains auto-generated project rules created by the `analyze-project` command.

## How to populate

Run the `/dw-analyze-project` command inside your AI assistant to scan your codebase and generate:

- `index.md` — Project overview, stack summary, quick reference
- `{module}.md` — Per-module detailed rules with patterns and conventions

## Structure

```
.dw/rules/
├── README.md          # This file
├── index.md           # Project overview (auto-generated)
└── {module}.md        # Per-module rules (auto-generated)
```

## Usage

These rules are automatically read by workflow commands (`create-prd`, `create-techspec`, `run-task`, etc.) to ensure generated artifacts follow your project's conventions.

Re-run `/dw-analyze-project` whenever your stack or conventions change significantly.

## Related: the curated baseline library

This directory (`.dw/rules/`) is **analytical** — what the code IS. It is complemented by
`.dw/rules-library/` — a **curated declarative baseline** (what good code SHOULD look like) shipped per
stack, loaded lazily via `.dw/config/stack-mappings.json` — and by `.dw/constitution.md` — the principles
your team commits to. `/dw-analyze-project` references the library to seed constitution proposals; you don't
edit the library here. See `.dw/rules-library/README.md`.

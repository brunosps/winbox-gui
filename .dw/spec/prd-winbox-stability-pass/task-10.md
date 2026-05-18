---
type: task
schema_version: "1.0"
status: pending
task_id: 10
depends_on: [1, 2, 3, 4, 5, 6, 7, 8, 9]
---

# Task 10.0: cargo clippy -D warnings clean

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and fix every warning. Aim for zero `#[allow]` overrides; if a lint is a real false positive, document inline with rationale + (optional) link to clippy issue.

**Functional Requirements covered**: RF-09

<requirements>
- `cargo clippy --all-targets -- -D warnings` exits 0.
- No new `#[allow(clippy::...)]` without a `// reason: ...` comment.
- `cargo fmt --check` also passes (zero diff).
- All prior tasks landed (clippy depends on the final code shape).
</requirements>

## Subtasks

### Implementation
- [ ] 10.1 Run clippy. Categorize warnings.
- [ ] 10.2 Apply mechanical fixes (.clone-needed, format!-without-arg, &Vec<T> → &[T], etc.).
- [ ] 10.3 For each non-mechanical, judgment-call warning, either refactor or add `#[allow(...)] // reason: …`.
- [ ] 10.4 Re-run clippy; iterate until clean.
- [ ] 10.5 `cargo fmt`; commit any formatting deltas.

### Unit Tests
None — clippy is the gate.

## Success Criteria

- CI `clippy` step passes.
- `cargo fmt --check` passes.

## Relevant Files

- All `src-tauri/src/**/*.rs`.

## Commit on Completion

```
chore(clippy): drive cargo clippy -D warnings to zero
```

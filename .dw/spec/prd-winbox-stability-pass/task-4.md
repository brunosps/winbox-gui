---
type: task
schema_version: "1.0"
status: pending
task_id: 4
depends_on: [3]
---

# Task 4.0: Integration tests for ensure_running / start with MockDocker

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Add unit tests in `src-tauri/src/commands/launch.rs` (#[cfg(test)] mod tests) that drive `ensure_running` and `start` with `MockDocker`. Cover the full decision tree of `ensure_running` plus `start`, including the new defensive `rm_force` arm for stale containers.

**Functional Requirements covered**: RF-02

<requirements>
- ≥7 new test cases covering: running, paused, absent, exited, created, dead — for both `ensure_running` and `start` (de-duplicate covered cases).
- Tests assert order of mock calls (e.g., `rm_force` before `compose_run`).
- Tests for failure paths: compose returns `PortConflict` → propagates; logs_contains never reports Windows-ready → `TimeoutWindows`.
- Tests for `start` mirror `ensure_running` defensive logic.
- No filesystem writes (we don't write env files in these tests — MockDocker doesn't need them).
</requirements>

## Subtasks

### Implementation
- [ ] 4.1 Extend `MockDocker` with helpers: `seed_status(name, status)`, `seed_logs_contains(name, needle, result)`, `expect_call_order(&[&str])`, `set_compose_failure(err)`.
- [ ] 4.2 In `#[cfg(test)] mod tests` of `commands::launch.rs`, write tests per the list in techspec § "Testing Approach / commands::launch::tests".
- [ ] 4.3 For tests that hit `wait_for_ready`, mock the env_file read by providing a minimal `tempfile`-based config.env OR (simpler) add a `wait_for_ready_with_family` private helper that takes the family directly so tests bypass file I/O.

### Unit Tests
Inline in this task (no separate test file needed).

## Implementation Details

See techspec § "Testing Approach / Unit Tests (Rust)".

## Success Criteria

- ≥7 new passing tests in `cargo test commands::launch`.
- Total cargo test count goes from 12 → ≥22 (this task + prior tasks contribute).

## Relevant Files

- `src-tauri/src/commands/launch.rs`
- `src-tauri/src/core/docker.rs` (MockDocker extensions)

## Commit on Completion

```
test(commands): cover ensure_running / start decision tree
```

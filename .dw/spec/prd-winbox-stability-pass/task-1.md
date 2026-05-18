---
type: task
schema_version: "1.0"
status: pending
task_id: 1
---

# Task 1.0: Extract DockerClient trait + introduce CliDocker + MockDocker

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Refactor `src-tauri/src/core/docker.rs`: introduce `pub trait DockerClient` covering the 9 operations currently exposed as free functions. Provide `pub struct CliDocker` that wraps the existing `Command::new("docker")` logic (unchanged semantics). Add `pub mod mock` under `#[cfg(test)]` with a `MockDocker` that records calls and lets tests script return values.

**Functional Requirements covered**: RF-01

<requirements>
- `DockerClient` trait covers: container_status, compose_run, logs, logs_contains, pause, unpause, stop, kill, rm_force, pull.
- `CliDocker` is a unit struct. Existing free functions in core::docker remain as thin wrappers that delegate to `CliDocker` (so we don't break callers in this task; signature migration is Task 4).
- `MockDocker` is gated by `#[cfg(test)]` and exposes RefCell-based fields so tests can inspect calls and inject failures.
- Build must remain green (`cargo build`). No behavior change.
</requirements>

## Subtasks

### Implementation
- [ ] 1.1 Define `pub trait DockerClient` in `src-tauri/src/core/docker.rs`.
- [ ] 1.2 Implement the trait for a new unit struct `pub struct CliDocker`. Reuse the existing private helpers (`compose_bin`) verbatim.
- [ ] 1.3 Keep existing free functions (`container_status`, `compose_run`, etc.) as deprecated-internal wrappers calling `CliDocker.method()` so this task ships isolated.
- [ ] 1.4 Add `#[cfg(test)] pub mod mock` with `MockDocker { statuses: RefCell<HashMap<String,String>>, calls: RefCell<Vec<String>>, fail_compose: Option<LaunchError> /* placeholder until Task 3 lands */ }`.

### Unit Tests
- [ ] 1.5 `mock_records_calls` — instantiate MockDocker, call `container_status` + `rm_force`, assert `calls` vector content.
- [ ] 1.6 `mock_status_returns_seeded_value` — preset `statuses` map and verify lookup.

## Implementation Details

See `techspec.md` § "Key Interfaces / `core::docker::DockerClient`".

Note: Task 3 introduces `LaunchError` — the `MockDocker::fail_compose` field type is a placeholder (`Option<String>`) until Task 3 lands; refactor at that point.

## Success Criteria

- `cargo build --manifest-path src-tauri/Cargo.toml` succeeds.
- `cargo test --manifest-path src-tauri/Cargo.toml` adds ≥2 passing tests.
- No call-site changes outside `core::docker.rs`.

## Relevant Files

- `src-tauri/src/core/docker.rs`

## Commit on Completion

```
refactor(core): introduce DockerClient trait + CliDocker + MockDocker
```

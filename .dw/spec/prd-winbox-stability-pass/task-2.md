---
type: task
schema_version: "1.0"
status: pending
task_id: 2
depends_on: [1]
---

# Task 2.0: Add `core::launch_error::LaunchError` enum

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Create `src-tauri/src/core/launch_error.rs` with the `LaunchError` enum (Debug, Clone, Serialize), implement `code()` returning stable snake_case identifiers, implement `Display` for CLI consumption, and implement `std::error::Error`. Wire it into `core::mod`.

**Functional Requirements covered**: RF-03 (foundation)

<requirements>
- Variants per techspec: KvmDenied, DockerMissing, DockerDaemonDown, FreeRdpMissing, PortConflict { port }, ImagePullFailed { image, stderr }, ContainerCrash { container, log_tail }, TimeoutWindows { profile }, TimeoutLinux { profile, port }, Other { message }.
- `#[serde(tag = "code", rename_all = "snake_case")]` so JSON shape is `{ "code": "kvm_denied" }` for unit variants and `{ "code": "port_conflict", "port": 3389 }` for struct variants.
- `LaunchError::code()` returns &'static str matching the serde tag.
- `impl From<anyhow::Error> for LaunchError` returns `LaunchError::Other { message: e.to_string() }` so legacy `?` continues working during migration.
</requirements>

## Subtasks

### Implementation
- [ ] 2.1 Create `src-tauri/src/core/launch_error.rs` with the enum and derives.
- [ ] 2.2 Add `pub mod launch_error` to `src-tauri/src/core/mod.rs`.
- [ ] 2.3 Implement `Display`, `Error`, `From<anyhow::Error>`.
- [ ] 2.4 Update `MockDocker::fail_compose` field type from placeholder `Option<String>` to `Option<LaunchError>` (Task 1 left it as placeholder).

### Unit Tests
- [ ] 2.5 `code_returns_stable_snake_case` — exhaustive match over all variants, assert stable strings.
- [ ] 2.6 `serializes_with_tag` — `serde_json::to_string` of `KvmDenied` produces `{"code":"kvm_denied"}`; of `PortConflict { port: 3389 }` produces `{"code":"port_conflict","port":3389}`.

## Implementation Details

See techspec § "`core::launch_error::LaunchError`".

## Success Criteria

- `cargo test core::launch_error` passes with ≥2 tests.
- `cargo build` succeeds.

## Relevant Files

- `src-tauri/src/core/launch_error.rs` (new)
- `src-tauri/src/core/mod.rs`
- `src-tauri/src/core/docker.rs` (mock field type)

## Commit on Completion

```
feat(core): add LaunchError enum with serde tag-based serialization
```

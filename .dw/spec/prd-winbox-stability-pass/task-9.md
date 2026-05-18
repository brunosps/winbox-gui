---
type: task
schema_version: "1.0"
status: pending
task_id: 9
depends_on: []
---

# Task 9.0: docs/DEBUGGING.md (EN-US, contributor-facing)

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Write `docs/DEBUGGING.md` in English for contributors. Covers: log locations, dev-mode tracing, XDG paths, ways to simulate failures locally, and a quick-reference table of `LaunchError` codes ↔ source location.

**Functional Requirements covered**: RF-08

<requirements>
- Sections: "Where logs live", "Running in dev mode", "XDG paths", "Simulating failure conditions", "LaunchError reference table".
- Reference table maps `LaunchError::Variant` → file/line in `core::launch_error` or where it's emitted in `commands/launch`.
- ~400-700 words. Code blocks for shell commands.
</requirements>

## Subtasks

### Implementation
- [ ] 9.1 Create `docs/DEBUGGING.md` per PRD § "RF-08".
- [ ] 9.2 Link from `README.md` or `CONTRIBUTING.md` (whichever exists; if neither, just leave it in docs/).

## Success Criteria

- File renders cleanly in GitHub Markdown.

## Relevant Files

- `docs/DEBUGGING.md` (new)

## Commit on Completion

```
docs(contrib): add EN-US debugging guide with LaunchError reference
```

---
type: task
schema_version: "1.0"
status: pending
---

# Task X.0: [Main Task Title]

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

[Brief task description]

**Functional Requirements covered**: FR[X.Y], FR[X.Z] (maximum 2 per task)

<requirements>
[List of mandatory requirements]
</requirements>

## Subtasks

### Implementation
- [ ] X.1 [Subtask description]
- [ ] X.2 [Subtask description]

### Unit Tests (Mandatory for Backend)
- [ ] X.3 Create tests for [service/use-case]
- [ ] X.4 Create tests for [controller/adapter]

## Unit Tests

### Test Cases

| Method | Cases |
|--------|-------|
| `[method1]` | Happy path, edge case, error |
| `[method2]` | Happy path, not found |

### Required Mocks
- `[repository/service]` - mocked via mock function

## Implementation Details

[Relevant sections from the tech spec - reference techspec.md instead of duplicating content]

## Success Criteria

- [Measurable outcomes]
- [Quality requirements]
- **Unit tests passing**
- **Minimum 80% coverage** on services/use-cases

## Relevant Files
- [Files relevant to this task]
- [File].spec.ts - Unit tests

## Commit on Completion

When completing this task, commit:
```bash
git add .
git commit -m "feat([module]): [description]

- [item 1]
- [item 2]
- Add unit tests"
```

## Related ADRs

[ADRs that constrain this task's decisions. Leave empty if none.

- `adrs/adr-NNN.md` — [short title, how the decision affects this task]]

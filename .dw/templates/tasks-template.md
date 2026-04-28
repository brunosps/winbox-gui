---
type: tasks-index
schema_version: "1.0"
status: draft
---

# Implementation Tasks Summary for [Feature]

## Branch

```
feat/prd-[feature-name]
```

## Impacted Projects

- [ ] [Project 1]
- [ ] [Project 2]

## Tasks

| Task | Description | FRs | Status |
|------|-------------|-----|--------|
| 1.0 | [Title] | FR1.1, FR1.2 | Pending |
| 2.0 | [Title] | FR2.1 | Pending |
| 3.0 | [Title] | FR3.1, FR3.2 | Pending |

## Progress

- [ ] 1.0 Main Task Title
- [ ] 2.0 Main Task Title
- [ ] 3.0 Main Task Title

## Workflow

Each task follows this flow:
1. `/execute-task [N]_task.md` - Implements the task
2. Unit tests included in the implementation
3. Commit at the end of each task (no push)
4. Next task or `/dw-generate-pr [target-branch]` when all tasks are completed

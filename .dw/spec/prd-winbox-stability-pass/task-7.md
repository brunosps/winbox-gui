---
type: task
schema_version: "1.0"
status: pending
task_id: 7
depends_on: [3, 4, 5]
---

# Task 7.0: GitHub Actions CI workflow

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Add `.github/workflows/ci.yml` with two jobs: `rust` (fmt + clippy -D warnings + test) and `js` (check:js + test:js). Triggered on pull_request and on push to `main`.

**Functional Requirements covered**: RF-06

<requirements>
- Two jobs `rust` and `js`, both on `ubuntu-latest`.
- `rust` installs GTK/WebKit dev headers so cargo can link.
- Cargo cache keyed by `Cargo.lock` hash.
- Triggers: `pull_request` (any branch) and `push` to `main`.
- Workflow file is valid YAML and passes a quick lint (`yq` or `actionlint` if available — not required to install).
</requirements>

## Subtasks

### Implementation
- [ ] 7.1 Create `.github/workflows/ci.yml` per techspec § "RF-06 — GitHub Actions CI".
- [ ] 7.2 Verify locally with a yaml linter (optional): `python -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml'))"`.

### Unit Tests
None — verified by CI itself once a PR is opened.

## Implementation Details

See techspec § "RF-06".

## Success Criteria

- File exists and is valid YAML.
- A subsequent PR triggers both jobs.
- Both jobs pass on the current `main` content (after prior tasks land).

## Relevant Files

- `.github/workflows/ci.yml` (new)

## Commit on Completion

```
ci: add GitHub Actions workflow for Rust + JS tests
```

---
type: task
schema_version: "1.0"
status: pending
task_id: 5
depends_on: []
---

# Task 5.0: Extract profile-display.js + node:test coverage

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Extract `modeMeta`, `primaryActionMeta`, `profileState` from `src/main.js` into `src/profile-display.js` as named ESM exports. Add `src/profile-display.test.js` with node:test cases covering the decision tables. Update `src/main.js` to import from the new file. Update `package.json` `check:js` to also `node --check src/profile-display.js`.

**Functional Requirements covered**: RF-05

<requirements>
- Three named exports: `modeMeta(p)`, `profileState(p)`, `primaryActionMeta(p, t)`.
- `primaryActionMeta` takes `t` (i18n fn) as second arg (it was using the global `t` before — explicit is cleaner and trivially testable).
- `src/main.js` uses the imports; no behavior change visible in the GUI.
- ≥8 new tests passing via `npm run test:js`.
</requirements>

## Subtasks

### Implementation
- [ ] 5.1 Create `src/profile-display.js` with the three named exports.
- [ ] 5.2 Remove old definitions from `src/main.js` and add the import statement.
- [ ] 5.3 Pass `t` into `primaryActionMeta` at the call site.
- [ ] 5.4 Add `node --check src/profile-display.js` to `package.json` `check:js`.

### Unit Tests
- [ ] 5.5 `src/profile-display.test.js` covering: modeMeta {rdp / web_vnc / undefined}, profileState {Up… / running / Paused / Exited / Dead / Created / undefined}, primaryActionMeta {running → connect, paused → resume, exited → launch, absent → launch}.

## Implementation Details

See techspec § "profile-display.js" and the `modeMeta`/`primaryActionMeta`/`profileState` source in techspec.

## Success Criteria

- `npm run test:js` goes from 3 → ≥11 passing tests.
- `npm run check:js` includes the new file.
- `npm run dev` still renders profile cards identically (manual smoke).

## Relevant Files

- `src/profile-display.js` (new)
- `src/profile-display.test.js` (new)
- `src/main.js`
- `package.json`

## Commit on Completion

```
test(ui): extract profile-display module and add node:test coverage
```

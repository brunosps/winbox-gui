---
type: task
schema_version: "1.0"
status: pending
task_id: 6
depends_on: [3]
---

# Task 6.0: Frontend renderLaunchError + i18n keys

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Add `renderLaunchError(err)` in `src/main.js`. Detects whether `err` is a structured object (`{ code, ...fields }`) and renders a localized toast with a copy-to-clipboard button when the i18n message embeds a shell command. Falls back to plain string rendering if shape is wrong. Add `launch.error.<code>` keys to both locales.

**Functional Requirements covered**: RF-04 (frontend half)

<requirements>
- 10 i18n keys per locale (one per LaunchError variant): `launch.error.kvm_denied`, `docker_missing`, `docker_daemon_down`, `freerdp_missing`, `port_conflict`, `image_pull_failed`, `container_crash`, `timeout_windows`, `timeout_linux`, `other`.
- PT-BR messages embed actionable shell commands where applicable.
- `renderLaunchError` only changes the launch flow handler; other commands still use `showErrorToast`.
- Graceful fallback when err is string or has unknown `code`.
</requirements>

## Subtasks

### Implementation
- [ ] 6.1 Add `renderLaunchError(err)` in `src/main.js`. Replace the `launch_profile` catch with it.
- [ ] 6.2 Add the 10 keys in `src/locales/pt-BR.js` and `src/locales/en-US.js`. Use `{{port}}`, `{{image}}`, `{{profile}}` interpolation as needed (consistent with existing `t()` helper).
- [ ] 6.3 Optionally surface a "Copiar comando" button if hint string contains a backticked command — defer to a follow-up if scope creeps.

### Unit Tests
- [ ] 6.4 Manual smoke: trigger a port conflict deliberately (set RDP_PORT to 22 in a test profile), confirm toast shows interpolated port.

## Implementation Details

See techspec § "Frontend toast estruturado" and PRD § "RF-04".

## Success Criteria

- 10 locale keys added in each language.
- Manual smoke shows new toast on KvmDenied (fake by removing /dev/kvm group temporarily — or just by forcing the variant in JS console).
- No regression in other commands' error toasts.

## Relevant Files

- `src/main.js`
- `src/locales/pt-BR.js`
- `src/locales/en-US.js`

## Commit on Completion

```
feat(ui): render structured LaunchError with i18n hints
```

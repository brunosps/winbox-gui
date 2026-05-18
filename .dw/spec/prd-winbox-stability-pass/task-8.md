---
type: task
schema_version: "1.0"
status: pending
task_id: 8
depends_on: []
---

# Task 8.0: TROUBLESHOOTING.md (PT-BR, user-facing)

<critical>Read the prd.md and techspec.md files in this folder. If you don't read these files your task will be invalidated.</critical>

## Overview

Write `TROUBLESHOOTING.md` at repo root in PT-BR. Four sections corresponding to the most common observed symptoms. Each section: sintoma → diagnóstico (1-3 comandos) → fix → onde fica log de evidência. Link from `README.md`.

**Functional Requirements covered**: RF-07

<requirements>
- 4 sections: (1) Cliquei Iniciar e nada acontece → FreeRDP missing; (2) Janela VNC abre toda preta → GPU passthrough display routing; (3) docker compose name conflict → already fixed by ensure_running rm_force; (4) Windows não termina de iniciar em 240s → primeira instalação baixa ISO.
- Linked from `README.md` with a "Problemas comuns?" section.
- Each section opens with sintoma in bold, ends with "Log de evidência: <path>".
- ~200-400 palavras totais — escannable.
</requirements>

## Subtasks

### Implementation
- [ ] 8.1 Create `TROUBLESHOOTING.md` per PRD § "RF-07".
- [ ] 8.2 Add reference link in `README.md`.

### Unit Tests
None — doc review on its own.

## Success Criteria

- File renders cleanly in GitHub Markdown.
- README has a one-liner pointing to it.

## Relevant Files

- `TROUBLESHOOTING.md` (new)
- `README.md`

## Commit on Completion

```
docs(troubleshooting): add user-facing TROUBLESHOOTING.md (PT-BR)
```

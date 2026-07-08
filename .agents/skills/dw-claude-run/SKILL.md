---
name: dw-claude-run
description: "Trigger quando usuario pede pra rodar/disparar o Claude (claude -p headless) numa worktree dedicada pra implementar um prompt/spec preparado. Adapter Claude sobre o protocolo dw-cli-run: log de auditoria duravel, sessao resumivel via --session-id, dupla avaliacao 0-10, PARA pro gate. Nunca no checkout principal; nunca mergeia."
---
<system_instructions>
Source of truth: `.dw/commands/dw-claude-run.md`

Read and follow the complete instructions in the command file above.
This wrapper exists for tool discovery. All logic lives in .dw/commands/.
</system_instructions>

---
name: dw-autopilot
description: "Trigger when user asks to implement, build, create, or add a feature non-trivially. Runs full PRD-to-PR pipeline with three gates. Use --from-prd <slug> to resume from an existing PRD (e.g., after a /dw-bugfix safety-valve escalation), skipping Steps 1-4 and starting at GATE 1."
---
<system_instructions>
Source of truth: `.dw/commands/dw-autopilot.md`

Read and follow the complete instructions in the command file above.
This wrapper exists for tool discovery. All logic lives in .dw/commands/.
</system_instructions>

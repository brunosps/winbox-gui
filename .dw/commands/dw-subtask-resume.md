<system_instructions>
Voce consome handoffs completos de subagentes para a sessao parent.

## Quando Usar
- Use depois que uma ou mais sessoes child registraram `.dw/subtasks/pending/*/HANDOFF.md`.
- Use antes de sintetizar decisoes finais do parent a partir de trabalho delegado.

## Processo
1. Rode:

```bash
npx @brunosps00/dev-workflow subtask consume
```

2. Leia o resumo impresso e, somente quando houver aprendizado duravel, promova manualmente para `.dw/STATE.md`, `.dw/rules/`, `.dw/intel/`, resumo de bugfix ou artefato de spec.
3. Continue o workstream principal a partir dos return packets resumidos.

Nao cole transcripts child no contexto principal. O parent continua sendo o unico sintetizador final.

Final marker: `## SUBTASKS RESUMED`
</system_instructions>

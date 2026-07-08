<system_instructions>
Voce registra um handoff estruturado de subagente.

## Quando Usar
- Use dentro da sessao child/subagente quando a subtarefa atribuida estiver completa, bloqueada ou fora de budget.

## Formato Obrigatorio

```markdown
## Objective

## Result

## Files Read

## Files Changed

## Decisions

## Risks

## Verification

## Next Steps

## Blocked Or Not Done
```

## Processo
1. Escreva o handoff em um arquivo markdown local.
2. Rode:

```bash
npx @brunosps00/dev-workflow subtask complete --slug=<slug> --file=<handoff.md>
```

Retorne apenas evidencia resumida. Nao inclua logs completos nem dumps de transcript.

Final marker: `## SUBTASK HANDOFF RECORDED`
</system_instructions>

---
type: tasks-index
schema_version: "1.0"
status: draft
---

# Resumo de Tarefas de Implementação de [Funcionalidade]

## Branch

```
feat/prd-[nome-funcionalidade]
```

## Projetos Impactados

- [ ] [Projeto 1]
- [ ] [Projeto 2]

## Tarefas

| Task | Descrição | RFs | Status |
|------|-----------|-----|--------|
| 1.0 | [Título] | RF1.1, RF1.2 | Pendente |
| 2.0 | [Título] | RF2.1 | Pendente |
| 3.0 | [Título] | RF3.1, RF3.2 | Pendente |

## Progresso

- [ ] 1.0 Título da Tarefa Principal
- [ ] 2.0 Título da Tarefa Principal
- [ ] 3.0 Título da Tarefa Principal

## Workflow

Cada task segue o fluxo:
1. `/dw-run [N]_task.md` - Implementa a task
2. Testes unitários incluídos na implementação
3. Commit ao final da task (sem push)
4. Próxima task ou `/dw-generate-pr [branch-alvo]` quando todas concluídas

---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa X.0: [Título da Tarefa Principal]

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

[Breve descrição da tarefa]

**Requisitos Funcionais cobertos**: RF[X.Y], RF[X.Z] (máximo 2 por task)

<requirements>
[Lista de requisitos obrigatórios]
</requirements>

## Subtarefas

### Implementação
- [ ] X.1 [Descrição da subtarefa]
- [ ] X.2 [Descrição da subtarefa]

### Testes Unitários (Obrigatório para Backend)
- [ ] X.3 Criar testes para [service/use-case]
- [ ] X.4 Criar testes para [controller/adapter]

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `[método1]` | Happy path, edge case, erro |
| `[método2]` | Happy path, not found |

### Mocks Necessários
- `[repositório/service]` - mockado via mock function

## Detalhes de Implementação

[Seções relevantes da spec técnica - referencie a techspec.md ao invés de duplicar conteúdo]

## Critérios de Sucesso

- [Resultados mensuráveis]
- [Requisitos de qualidade]
- **Testes unitários passando**
- **Cobertura mínima 80%** em services/use-cases

## Arquivos Relevantes
- [Arquivos relevantes desta tarefa]
- [Arquivo].spec.ts - Testes unitários

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add .
git commit -m "feat([modulo]): [descrição]

- [item 1]
- [item 2]
- Add unit tests"
```

## Related ADRs

[ADRs que restringem decisões desta task. Deixe vazio se não houver.

- `adrs/adr-NNN.md` — [título curto, como a decisão afeta esta task]]

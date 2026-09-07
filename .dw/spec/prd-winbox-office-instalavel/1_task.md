---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 1.0: Registrar ADRs obrigatórios da feature Office

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Registrar os três ADRs que sustentam as decisões de risco antes de implementar código: fronteira AGPL/artefatos OEM, estratégia ODT e política `cert:ignore` vs `tofu`. O texto é proposta técnica; a aprovação final é do dono no gate da task.

**Requisitos Funcionais cobertos**: FR-3.5, FR-3.6

**Depende de**: nenhuma task anterior.

**Constitution**: respects P-005, P-007.

<requirements>
- Criar `adrs/` dentro de `.dw/spec/prd-winbox-office-instalavel/`.
- Criar um ADR para a fronteira AGPL e a regra "não vendorizar WinApps".
- Criar um ADR para estratégia ODT: setup pré-staged pelo winbox, binários Office baixados pelo guest quando necessário.
- Criar um ADR para `RDP_FLAGS` com `cert:ignore`, risco aceito e alternativa `tofu`.
- Marcar cada ADR como proposta aguardando aprovação do dono.
</requirements>

## Subtarefas

### Implementação
- [ ] 1.1 Criar `adrs/adr-agpl-winapps-runtime-boundary.md`.
- [ ] 1.2 Criar `adrs/adr-office-odt-download-strategy.md`.
- [ ] 1.3 Criar `adrs/adr-rdp-cert-ignore-vs-tofu.md`.
- [ ] 1.4 Linkar cada ADR às seções correspondentes do techspec sem alterar o techspec.

### Testes Automatizados
- [ ] 1.5 Sem teste automatizado: task puramente documental; invariante protegido por revisão humana é "decisões high-risk existem antes do código"; camada: gate de PR/ADR.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| n/a | ADRs existem, têm status proposto, decisão, contexto, consequências e relação com FRs |

### Mocks Necessários
- n/a.

## Detalhes de Implementação

Ver techspec §§Related ADRs, Conformidade com Padrões, Decisões Principais.

## Critérios de Sucesso

- Os três ADRs existem no diretório da feature.
- Cada ADR declara decisão, alternativas, trade-offs e FRs impactados.
- Nenhum arquivo fora de `.dw/spec/prd-winbox-office-instalavel/adrs/` é editado nesta task.

## Arquivos Relevantes
- `.dw/spec/prd-winbox-office-instalavel/adrs/adr-agpl-winapps-runtime-boundary.md`
- `.dw/spec/prd-winbox-office-instalavel/adrs/adr-office-odt-download-strategy.md`
- `.dw/spec/prd-winbox-office-instalavel/adrs/adr-rdp-cert-ignore-vs-tofu.md`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add .dw/spec/prd-winbox-office-instalavel/adrs
git commit -m "docs(office): record office architecture ADRs"
```

## Related ADRs

- Esta task cria os ADRs-base da feature.

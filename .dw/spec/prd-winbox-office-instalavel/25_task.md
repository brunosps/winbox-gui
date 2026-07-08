---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 25.0: Implementar endpoint mínimo de telemetria beta [infra externa]

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Provisionar o endpoint HTTPS mínimo self-hosted no Coolify do dono, aceitando JSON público sem auth no beta, com retenção curta e export simples.

**Requisitos Funcionais cobertos**: FR-8.3, FR-8.4

**Depende de**: 24.0.

**Constitution**: respects P-006, P-007.

<requirements>
- Endpoint público sem autenticação no beta, com risco de poisoning aceito conforme techspec.
- Persistir eventos JSON e aplicar retenção curta.
- Derivar abandono server-side pelo último evento por pseudônimo.
- Fornecer export simples para análise do beta.
- Atualizar a constante placeholder de `core/paths.rs` por release quando a URL pública mudar.
- Não armazenar IDs persistentes de máquina.
</requirements>

## Subtarefas

### Implementação
- [ ] 25.1 Criar serviço mínimo no Coolify do dono conforme stack escolhida fora do repo principal.
- [ ] 25.2 Implementar validação básica de shape JSON e limites de tamanho.
- [ ] 25.3 Implementar retenção curta e export simples.
- [ ] 25.4 Confirmar URL pública usada pela constante placeholder da task 24.0.

### Testes Automatizados
- [ ] 25.5 Gate manual do dono `[infra externa]` `telemetry_endpoint_accepts_valid_beta_event` — invariante: evento válido retorna 2xx; camada: API smoke externa.
- [ ] 25.6 Gate manual do dono `[infra externa]` `telemetry_endpoint_rejects_oversized_payload` — invariante: payload grande não é gravado; camada: API smoke externa.
- [ ] 25.7 Gate manual do dono `[infra externa]` `abandonment_is_derived_server_side` — invariante: abandono não depende de evento client especial; camada: integração servidor externa.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| endpoint externo | evento válido, JSON inválido, payload grande, export, retenção |

### Mocks Necessários
- Fixture HTTP local ou ambiente Coolify staging.

## Detalhes de Implementação

Ver techspec §§Telemetria, Endpoint, Riscos, Sequenciamento de Desenvolvimento.

## Critérios de Sucesso

- Smoke `curl` para endpoint retorna 2xx com evento válido.
- Retenção curta está configurada.
- Export simples disponível para o dono.
- URL de beta não contém segredo.

## Arquivos Relevantes
- `[infra externa] Coolify do dono`
- `src-tauri/src/core/telemetry.rs`
- `src-tauri/src/core/paths.rs`
- `.github/workflows/*release*`

## Commit ao Final

Ao completar esta task, fazer commit se houver mudança no repo principal:
```bash
git add src-tauri/src/core/telemetry.rs .github/workflows
git commit -m "feat(telemetry): configure beta telemetry endpoint"
```

## Related ADRs

- n/a.

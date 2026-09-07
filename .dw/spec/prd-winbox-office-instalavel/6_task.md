---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 6.0: Detectar recursos insuficientes e conflitos de subnet

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Completar o preflight com recursos mínimos/recomendados e detecção best-effort de conflito entre redes Docker e rotas locais do host.

**Requisitos Funcionais cobertos**: FR-2.3, FR-2.5

**Depende de**: 4.0.

**Constitution**: respects P-001, P-002, P-004, P-006.

<requirements>
- Comparar `docker network inspect`, `default-address-pools` do Docker e `ip route`.
- Emitir `preflight_subnet_conflict` com `action_hint` concreto.
- Modelar warnings de recurso com override explícito.
- Marcar a checagem de subnet como best-effort com comentário e teste.
</requirements>

## Subtarefas

### Implementação
- [ ] 6.1 Criar parser puro para CIDRs de Docker e rotas locais.
- [ ] 6.2 Estender `DockerClient`/mock para network inspect e default-address-pools.
- [ ] 6.3 Implementar classificação de recursos insuficientes com `warning-override`.
- [ ] 6.4 Integrar conflito de subnet no preflight.

### Testes Automatizados
- [ ] 6.5 Teste `subnet_conflict_detected_between_docker_and_host_routes` — invariante: CIDRs sobrepostos bloqueiam com hint; camada: unit Rust core/preflight.
- [ ] 6.6 Teste `resource_warning_requires_explicit_override` — invariante: warning não segue sem override; camada: unit Rust core/preflight.
- [ ] 6.7 Teste `subnet_check_best_effort_never_masks_other_blockers` — invariante: falha de inspeção de rede não engole erros reais; camada: unit Rust core/preflight.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `parse_route_cidrs` | rota default, CIDR específico, saída vazia |
| `detect_subnet_conflict` | sem overlap, overlap Docker/host, dados incompletos |
| `classify_resources` | mínimo ok, warning, blocker |

### Mocks Necessários
- `MockDocker`.
- Trait de host command para `ip route`.

## Detalhes de Implementação

Ver techspec §§FR-2.5 subnet, PreflightCheck, Riscos.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- subnet resources preflight` passa.
- `preflight_subnet_conflict` inclui `details.action_hint`.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte de subnet/recursos passando.

## Arquivos Relevantes
- `src-tauri/src/core/office_preflight.rs`
- `src-tauri/src/core/docker.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): detect resource and subnet preflight issues"
```

## Related ADRs

- n/a.

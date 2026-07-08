---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 4.0: Implementar preflight base com erros acionáveis

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar o núcleo de preflight Office para KVM, Docker/Compose, rede base e shape `PreflightCheck` com `requirement`, `impact` e `action_hint`.

**Requisitos Funcionais cobertos**: FR-2.1, FR-2.2

**Depende de**: 3.0.

**Constitution**: respects P-001, P-002, P-004, P-006.

<requirements>
- Criar funções puras de classificação de preflight e I/O atrás de traits.
- Estender `DockerClient`/`MockDocker` sem exigir Docker nos testes existentes.
- Emitir erros estáveis como `preflight_kvm_missing`, `preflight_docker_missing` e checks acionáveis.
- Alinhar args de `office_preflight` com `office_start_provisioning`.
</requirements>

## Subtarefas

### Implementação
- [ ] 4.1 Definir `PreflightCheck` e `Resources` conforme techspec.
- [ ] 4.2 Estender trait/mocks para checks Docker e Compose necessários.
- [ ] 4.3 Implementar checks KVM, Docker, Compose e conectividade base.
- [ ] 4.4 Preparar retorno consumível pelos comandos Tauri da task 10.0.

### Testes Automatizados
- [ ] 4.5 Teste `preflight_check_has_action_hint_for_blocker` — invariante: todo blocker tem ação concreta; camada: unit Rust core/preflight.
- [ ] 4.6 Teste `preflight_uses_mock_docker_without_daemon` — invariante: 108 testes seguem sem Docker real; camada: unit Rust core com MockDocker.
- [ ] 4.7 Teste `office_preflight_args_match_start_provisioning` — invariante: os dois contratos aceitam o mesmo perfil/config; camada: integração commands com mocks.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `classify_preflight` | ok, warning, blocker, action_hint ausente rejeitado |
| `check_docker` | daemon ok, daemon ausente, compose ausente |
| `check_kvm` | disponível, ausente, permissão insuficiente |

### Mocks Necessários
- `MockDocker`.
- Trait para comandos de host quando necessário.

## Detalhes de Implementação

Ver techspec §§Contratos de API, Preflight, Estratégia de Testes.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- preflight` passa.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte de preflight passando.

## Arquivos Relevantes
- `src-tauri/src/core/office_preflight.rs`
- `src-tauri/src/core/docker.rs`
- `src-tauri/src/core/validation.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): add actionable host preflight checks"
```

## Related ADRs

- n/a.

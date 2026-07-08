---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 3.0: Validar configuração Office e imutabilidade do perfil

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Adicionar validação central para chaves Office no `config.env`, incluindo `PROFILE_KIND=office`, Windows 11 Pro, Product ID, idioma e guarda contra alteração destrutiva de versão/idioma depois do provisionamento.

**Requisitos Funcionais cobertos**: FR-4.3, FR-4.4

**Depende de**: 2.0.

**Constitution**: respects P-001, P-004, P-006.

<requirements>
- Reusar `core/validation.rs` e `env_file::set_key`.
- Defaults de compatibilidade para perfis existentes devem ser strings vazias, sem migration formal.
- Bloquear alteração de `VERSION`/`OFFICE_LANGUAGE` em perfil Office já provisionado sem caminho destrutivo explícito.
- Validar `OFFICE_PRODUCT_ID`, `OFFICE_LANGUAGE`, `PROFILE_KIND` e escopo de perfil Office.
</requirements>

## Subtarefas

### Implementação
- [ ] 3.1 Adicionar validadores puros para chaves Office em `core/validation.rs`.
- [ ] 3.2 Adicionar helpers de leitura/gravação compatíveis com `env_file::set_key`.
- [ ] 3.3 Implementar guarda de imutabilidade para `VERSION` e idioma em perfis Office.
- [ ] 3.4 Integrar validação ao fluxo de configuração do perfil sem tocar no frontend ainda.

### Testes Automatizados
- [ ] 3.5 Teste `office_env_defaults_are_backward_compatible` — invariante: perfil antigo sem chaves Office carrega com defaults seguros; camada: unit Rust core/validation.
- [ ] 3.6 Teste `office_profile_rejects_language_version_mutation_after_provisioning` — invariante: alteração destrutiva é bloqueada; camada: unit Rust core/validation.
- [ ] 3.7 Teste `office_product_id_allowlist` — invariante: Product ID fora da allowlist falha antes de I/O; camada: unit Rust core/validation.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `validate_office_config` | Product ID válido, idioma válido, valores vazios de compatibilidade, valor inválido |
| `guard_office_immutable_fields` | antes do provisionamento, depois do provisionamento, perfil não Office |

### Mocks Necessários
- Estado Office da task 2.0 em memória.
- Arquivos `config.env` temporários.

## Detalhes de Implementação

Ver techspec §§Modelos de Dados, Compatibilidade com Perfis Existentes, Validação Central.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- validation office` passa.
- Nenhum efeito colateral ocorre antes dos validadores.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte de validação passando.

## Arquivos Relevantes
- `src-tauri/src/core/validation.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/core/profile.rs`
- `src-tauri/src/commands/set.rs`
- `src-tauri/src/commands/reapply.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core/validation.rs src-tauri/src/core/office_state.rs src-tauri/src/core/profile.rs
git commit -m "feat(office): validate office profile configuration"
```

## Related ADRs

- `adrs/adr-office-odt-download-strategy.md` — define campos de idioma e Product ID.

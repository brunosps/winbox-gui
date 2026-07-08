---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 15.0: Remover perfil Office com confirmação e sem destruição acidental

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar remoção limpa do perfil Office, com `confirmToken` efêmero single-use, uninstall WinApps não-interativo e preservação de ativos não gerenciados.

**Requisitos Funcionais cobertos**: FR-6.5, FR-7.5

**Depende de**: 11.0, 12.0, 13.0, 14.0.

**Constitution**: respects P-001, P-002, P-004, P-006, P-007.

<requirements>
- `office_remove_profile` sem token retorna `remove_requires_confirmation` com `details.confirmToken`.
- Token é efêmero, single-use, validado no backend e escopado por perfil+`deleteDisk`.
- GUI e CLI usam o mesmo fluxo.
- Remover apenas recursos no `managedScope` do winbox.
- Usar `winapps-setup --uninstall` não-interativo quando aplicável.
</requirements>

## Subtarefas

### Implementação
- [ ] 15.1 Implementar mint/validação de `confirmToken` no backend.
- [ ] 15.2 Integrar remoção com escopo gerenciado da task 14.0.
- [ ] 15.3 Emitir step `office_remove_winapps` ao remover WinApps gerenciado.
- [ ] 15.4 Emitir step `office_remove_desktop` ao remover launchers/MIME gerenciados.
- [ ] 15.5 Remover estado Office e disco apenas quando `deleteDisk` for confirmado.
- [ ] 15.6 Expor o fluxo em Tauri e CLI com o mesmo contrato.

### Testes Automatizados
- [ ] 15.7 Teste `office_remove_requires_single_use_token` — invariante: token é obrigatório, escopado e não pode ser reutilizado; camada: integração commands.
- [ ] 15.8 Teste `remove_preserves_unmanaged_adoption_assets` — invariante: ativos do usuário não são apagados; camada: unit Rust core/adoption/removal.
- [ ] 15.9 Teste `office_cli_remove_uses_same_confirmation_flow` — invariante: CLI imprime token/frase e segunda chamada remove; camada: integração CLI Rust.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `mint_confirm_token` | novo token, escopo perfil+deleteDisk, expiração |
| `validate_confirm_token` | válido, reutilizado, perfil errado, deleteDisk errado |
| `remove_office_profile` | gerenciado, adoção mista, uninstall WinApps falha |

### Mocks Necessários
- `MockWinAppsClient`.
- `MockDocker`.
- Store de token em memória.

## Detalhes de Implementação

Ver techspec §§Ciclo de Vida do confirmToken, Remoção Limpa, Adoção e Managed Scope.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- remove confirm_token` passa.
- Nenhum segredo ou senha é retornado ao frontend.

## Arquivos Relevantes
- `src-tauri/src/commands/office.rs`
- `src-tauri/src/cli_office.rs`
- `src-tauri/src/core/winapps.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/commands/lifecycle.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/commands src-tauri/src/core src-tauri/src/cli_office.rs
git commit -m "feat(office): add confirmed clean removal"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — remoção respeita fronteira de artefatos gerenciados.

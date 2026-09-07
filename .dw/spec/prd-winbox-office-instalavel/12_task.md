---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 12.0: Integrar WinApps gerenciado, configuração e setup

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar cliente WinApps gerenciado pelo winbox, com clone pinado, `winapps.conf` seguro, setup dos launchers Office e mapeamento de exit codes.

**Requisitos Funcionais cobertos**: FR-3.5, FR-5.3

**Depende de**: 5.0, 9.0, 10.0.

**Constitution**: respects P-002, P-004, P-005, P-006, P-007.

<requirements>
- Clonar WinApps em `$XDG_DATA_HOME/winbox/winapps`, pinado por commit.
- Nunca vendorizar WinApps no repositório.
- Gerar `winapps.conf` 0600 com `RDP_USER`, `RDP_PASS`, `WAFLAVOR=manual`, `FREERDP_COMMAND` e `RDP_FLAGS`.
- Aplicar quoting/escaping para arquivo bash-sourced e documentar allowlist de senha se necessário.
- Rodar `winapps-setup --user --setupAllOfficiallySupportedApps`.
- Mapear exit codes 4/5/13/14/15 para `OfficeError`.
</requirements>

## Subtarefas

### Implementação
- [ ] 12.1 Criar trait `WinAppsClient` e mock.
- [ ] 12.2 Implementar clone/fetch/checkout pinado por commit.
- [ ] 12.3 Implementar renderização segura de `winapps.conf`.
- [ ] 12.4 Implementar setup, uninstall não-interativo e mapeamento de exit codes.

### Testes Automatizados
- [ ] 12.5 Teste `winapps_conf_quotes_rdp_credentials_for_bash_source` — invariante: senha não quebra arquivo sourced; camada: unit Rust core/winapps.
- [ ] 12.6 Teste `winapps_exit_code_maps_to_office_error` — invariante: codes upstream viram codes Office estáveis; camada: unit Rust core/winapps.
- [ ] 12.7 Teste `winapps_clone_pin_mismatch_is_reported` — invariante: commit diferente retorna `winapps_pin_mismatch`; camada: unit Rust core/winapps.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `render_winapps_conf` | credenciais simples, senha com caracteres permitidos, senha recusada |
| `map_winapps_exit_code` | 4, 5, 13, 14, 15, desconhecido |
| `ensure_winapps_clone` | clone novo, já pinado, rede falhou, pin mismatch |

### Mocks Necessários
- `MockWinAppsClient`.
- Trait de comando host para `git` e `winapps-setup`.

## Detalhes de Implementação

Ver techspec §§WinApps, FreeRDP Flatpak, Contratos de Erro, Minimalismo.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- winapps` passa.
- Nenhuma licença AGPL é vendorizada no repo.

## Arquivos Relevantes
- `src-tauri/src/core/winapps.rs`
- `src-tauri/src/core/flatpak.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core/winapps.rs src-tauri/src/core/flatpak.rs src-tauri/src/commands/office.rs
git commit -m "feat(office): manage winapps setup"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — proíbe vendorizar WinApps.
- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — rege `RDP_FLAGS`.

---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 5.0: Detectar FreeRDP nativo/Flatpak e preparar fallback

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar `FlatpakClient`/detecção FreeRDP: preferir `xfreerdp` nativo compatível major >=3, forçar Flatpak quando o nativo for inadequado e oferecer instalação guiada do Flatpak quando ausente.

**Requisitos Funcionais cobertos**: FR-2.4

**Depende de**: 4.0.

**Constitution**: respects P-001, P-002, P-004, P-006.

<requirements>
- Detectar `xfreerdp /version` nativo e gate major >=3.
- Detectar `flatpak info com.freerdp.FreeRDP` e versão via `flatpak list`.
- Detectar/solicitar `flatpak override --user --filesystem=home`.
- Retornar `flatpak_freerdp_missing` como bloqueante só quando o usuário recusar instalação.
- Gerar decisão para `FREERDP_COMMAND=flatpak` quando nativo for antigo ou quebrado.
</requirements>

## Subtarefas

### Implementação
- [ ] 5.1 Criar trait `FlatpakClient` e mock sem executar processos reais nos testes.
- [ ] 5.2 Criar parser puro de versão do `xfreerdp`.
- [ ] 5.3 Implementar checks de Flatpak, override home e escolha de comando FreeRDP.
- [ ] 5.4 Integrar resultados ao preflight da task 4.0.

### Testes Automatizados
- [ ] 5.5 Teste `native_freerdp_v2_forces_flatpak` — invariante: nativo major <3 não é usado; camada: unit Rust core/flatpak.
- [ ] 5.6 Teste `flatpak_missing_blocks_only_after_refusal` — invariante: ausência vira oferta guiada antes de erro; camada: unit Rust core/flatpak.
- [ ] 5.7 Teste `flatpak_home_override_required_for_home_drive` — invariante: sem override há warning/blocker acionável; camada: unit Rust core/flatpak.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `parse_freerdp_version` | v3, v2, saída inválida |
| `choose_freerdp_command` | nativo ok, nativo antigo, flatpak ausente, usuário recusou |

### Mocks Necessários
- `MockFlatpakClient`.
- Fixture de saídas de `xfreerdp /version` e `flatpak list`.

## Detalhes de Implementação

Ver techspec §§FreeRDP Flatpak, Preflight, Minimalismo.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- flatpak freerdp` passa.
- Nenhuma dependência Rust nova é adicionada.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte FreeRDP/Flatpak passando.

## Arquivos Relevantes
- `src-tauri/src/core/flatpak.rs`
- `src-tauri/src/core/office_preflight.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core/flatpak.rs src-tauri/src/core/office_preflight.rs src-tauri/src/commands/office.rs
git commit -m "feat(office): detect freerdp and flatpak fallback"
```

## Related ADRs

- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — define flags de RDP que dependem do comando FreeRDP.

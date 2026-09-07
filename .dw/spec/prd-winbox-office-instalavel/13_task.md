---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 13.0: Registrar desktop, MIME e verificação final

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Registrar launchers `.desktop` e associações MIME para Excel, Word e PowerPoint, e consolidar `final_verify` como critério de perfil pronto.

**Requisitos Funcionais cobertos**: FR-5.4, FR-5.5

**Depende de**: 12.0.

**Constitution**: respects P-002, P-004, P-006.

<requirements>
- Registrar `.xls`, `.xlsx`, `.doc`, `.docx`, `.ppt`, `.pptx`.
- Detectar launchers `excel-o365`, `word-o365`, `powerpoint-o365`.
- Decisão de ownership: o patch dos `.desktop` fica nesta task, junto do registro desktop/MIME.
- Reescrever `Exec=winbox office launch <perfil> <app> --gui-progress -- %F`, preservando `Icon`, `Name`, `StartupWMClass`, `Categories` e `MimeType`.
- Garantir que `final_verify` reexecuta depois de `desktop_registration` e `file_association`.
- Retornar `desktop_registration_failed`, `file_association_failed` ou `app_not_registered` com detalhes.
</requirements>

## Subtarefas

### Implementação
- [ ] 13.1 Implementar registro/validação de `.desktop` gerados pelo WinApps.
- [ ] 13.2 Implementar associação MIME por extensão Office.
- [ ] 13.3 Implementar patch dos `.desktop` para trocar apenas `Exec` pelo wrapper winbox.
- [ ] 13.4 Implementar `final_verify` consolidando Office, WinApps, desktop e MIME.
- [ ] 13.5 Integrar reexecução conforme grafo de retry da task 2.0.

### Testes Automatizados
- [ ] 13.6 Teste `desktop_patch_preserves_observable_fields` — invariante: patch altera só `Exec` e preserva campos observáveis; camada: unit Rust core/winapps.
- [ ] 13.7 Teste `final_verify_requires_desktop_and_mime_entries` — invariante: perfil não fica ready sem launchers e MIME; camada: unit Rust core/winapps ou office_state.
- [ ] 13.8 Teste `retry_winapps_config_reexecutes_desktop_registration_and_final_verify` — invariante: retry de `winapps_config` invalida descendentes; camada: unit Rust core/office_state.
- [ ] 13.9 Teste `mime_registration_maps_all_required_extensions` — invariante: seis extensões Office são associadas; camada: unit Rust core/winapps.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `verify_desktop_entries` | três apps ok, app ausente, launcher stale |
| `patch_desktop_exec` | preserva campos observáveis, troca `Exec`, arquivo malformado |
| `register_mime_associations` | novo registro, reapply idempotente, falha de comando |
| `final_verify` | completo, Office ausente, WinApps ausente, MIME ausente |

### Mocks Necessários
- `MockWinAppsClient`.
- Diretório XDG temporário.

## Detalhes de Implementação

Ver techspec §§Desktop Registration, File Association, Ready Semantics, Retry e Invalidação.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- desktop mime final_verify` passa.
- Reaplicar registro é idempotente.

## Arquivos Relevantes
- `src-tauri/src/core/winapps.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): register desktop launchers and mime types"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — launchers dependem do WinApps runtime.

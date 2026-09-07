---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 17.0: Integrar ciclo de vida Office e aviso de apps abertos

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Integrar perfis Office ao lifecycle do winbox e bloquear stop/restart/pause com confirmação quando houver sessão Office possivelmente ativa.

**Requisitos Funcionais cobertos**: FR-7.1, FR-7.6

**Depende de**: 10.0, 16.0.

**Constitution**: respects P-001, P-002, P-004, P-006, P-007.

<requirements>
- Detectar sessão ativa por processo `xfreerdp` Flatpak/nativo conectado ao `RDP_PORT` do perfil via `pgrep`/`ss`.
- Se indeterminável, avisar sempre quando `PROFILE_KIND=office` e VM estiver running.
- Adicionar `activeSessions` best-effort em `office_get_state`.
- Adicionar confirm-code `office_apps_maybe_open` em stop/restart/pause.
- Hook deve viver em `commands/lifecycle.rs`.
</requirements>

## Subtarefas

### Implementação
- [ ] 17.1 Criar detector best-effort de sessões RDP por porta do perfil.
- [ ] 17.2 Integrar `activeSessions` ao estado Office retornado.
- [ ] 17.3 Adicionar hook no lifecycle para stop/restart/pause.
- [ ] 17.4 Garantir confirmação compartilhada por GUI e CLI.

### Testes Automatizados
- [ ] 17.5 Teste `active_session_detected_by_xfreerdp_port` — invariante: processo conectado ao `RDP_PORT` vira `activeSessions=true`; camada: unit Rust core/lifecycle detector.
- [ ] 17.6 Teste `office_lifecycle_blocks_when_apps_maybe_open` — invariante: stop/restart/pause retorna confirm-code; camada: integração commands.
- [ ] 17.7 Teste `indeterminate_session_warns_for_running_office_profile` — invariante: falha best-effort não libera stop silencioso; camada: unit Rust commands/lifecycle.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `detect_active_rdp_sessions` | conectado, sem processo, erro de `ss`, porta inválida |
| `guard_office_lifecycle` | perfil Office running, perfil não Office, confirm já aceito |

### Mocks Necessários
- Trait de comando host para `pgrep`/`ss`.
- `MockDocker` para status da VM.

## Detalhes de Implementação

Ver techspec §§FR-7.6, Lifecycle, Contratos de API, State Matrix.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- lifecycle office_apps_maybe_open` passa.
- O guard não retorna senha nem detalhes sensíveis ao frontend.

## Arquivos Relevantes
- `src-tauri/src/commands/lifecycle.rs`
- `src-tauri/src/commands/office.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/core/winapps.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/commands/lifecycle.rs src-tauri/src/commands/office.rs src-tauri/src/core
git commit -m "feat(office): warn before stopping active office apps"
```

## Related ADRs

- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — ciclo de vida depende da porta RDP do perfil.

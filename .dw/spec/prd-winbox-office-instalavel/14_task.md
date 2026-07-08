---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 14.0: Detectar adoção manual e revisão segura

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Detectar setups manuais existentes de Windows/WinApps/Office, construir `AdoptionFinding` e permitir adoção apenas depois de revisão segura, sem destruir recursos do usuário.

**Requisitos Funcionais cobertos**: FR-6.3, FR-6.4

**Depende de**: 2.0, 10.0, 12.0.

**Constitution**: respects P-001, P-002, P-004, P-006, P-007.

<requirements>
- Detectar sinais: container `winbox-windows`, `~/.config/winapps/winapps.conf`, clone WinApps existente, `.desktop` Office existentes e perfil com `RDP_PORT` publicado.
- Modelar `managedScope` e `AdoptionFinding`.
- Validar RemoteApp no-op antes de marcar adoção como segura.
- Nunca remover ou sobrescrever instalação existente sem confirmação explícita.
</requirements>

## Subtarefas

### Implementação
- [ ] 14.1 Criar detectores puros para sinais de adoção e escopo gerenciado.
- [ ] 14.2 Integrar leitura de WinApps existente e perfil Docker publicado.
- [ ] 14.3 Implementar review de adoção em comando Tauri.
- [ ] 14.4 Persistir decisão de adoção no estado Office.

### Testes Automatizados
- [ ] 14.5 Teste `adoption_detects_all_required_signals` — invariante: cada sinal do techspec produz finding; camada: unit Rust core/winapps/adoption.
- [ ] 14.6 Teste `managed_scope_prevents_overwriting_user_assets` — invariante: ativo não gerenciado não é sobrescrito; camada: unit Rust core/adoption.
- [ ] 14.7 Teste `adoption_requires_remoteapp_noop_success` — invariante: adoção só segue com canal RemoteApp preparado; camada: integração commands com MockGuestExecutor.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `detect_adoption_findings` | container, winapps.conf, clone, desktop, RDP_PORT |
| `classify_managed_scope` | gerenciado pelo winbox, usuário, misto, desconhecido |
| `approve_adoption` | aprovado seguro, conflito, RemoteApp não preparado |

### Mocks Necessários
- `MockDocker`.
- `MockWinAppsClient`.
- `MockGuestExecutor`.
- Diretórios XDG temporários.

## Detalhes de Implementação

Ver techspec §§Adoção, Managed Scope, Contratos de API, Paradoxo RemoteApp.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- adoption` passa.
- Nenhuma remoção destrutiva ocorre no fluxo de adoção.

## Arquivos Relevantes
- `src-tauri/src/core/winapps.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): detect and review manual adoption"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — adoção respeita instalação WinApps do usuário.

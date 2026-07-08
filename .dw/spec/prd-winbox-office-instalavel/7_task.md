---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 7.0: Preparar RemoteApp antes do executor guest

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar a fase `remoteapp_prepare` antes de qualquer execução RemoteApp, com bootstrap `/oem` clean-room, marker de conclusão e fallback explícito para adoção ou falha de `/oem`.

**Requisitos Funcionais cobertos**: FR-3.6, FR-5.2

**Depende de**: 2.0, 3.0, 4.0.

**Constitution**: respects P-002, P-004, P-006.

<requirements>
- Gerar scripts CRLF clean-room sem copiar `RDPApps.reg`.
- Aplicar as chaves registry definidas no techspec e gravar marker `remoteapp_prepare.json`.
- Em adoção, testar canal com script no-op via RemoteApp.
- Se falhar, retornar `guest_remoteapp_not_prepared` com ação guiada.
- Garantir que nenhuma fase posterior use GuestExecutor antes de `remoteapp_prepare=done`.
- Criar o trait `GuestExecutor` e `MockGuestExecutor` como fronteira de I/O para as tasks posteriores.
</requirements>

## Subtarefas

### Implementação
- [ ] 7.1 Criar trait `GuestExecutor` com interface curta e `MockGuestExecutor`.
- [ ] 7.2 Criar gerador de bootstrap `/oem` clean-room com CRLF.
- [ ] 7.3 Implementar verificação de marker de `remoteapp_prepare`.
- [ ] 7.4 Implementar preflight de canal RemoteApp com script no-op.
- [ ] 7.5 Integrar a fase ao estado da task 2.0.

### Testes Automatizados
- [ ] 7.6 Teste `remoteapp_prepare_precedes_guest_executor_phases` — invariante: fases guest dependem de RemoteApp pronto; camada: unit Rust core/office_state.
- [ ] 7.7 Teste `remoteapp_bootstrap_is_clean_room_crlf` — invariante: script gerado é CRLF e contém só chaves permitidas; camada: unit Rust core/guest_executor.
- [ ] 7.8 Teste `adoption_noop_failure_returns_guest_remoteapp_not_prepared` — invariante: adoção falha com code acionável; camada: integração commands com MockGuestExecutor.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `render_remoteapp_bootstrap` | chaves esperadas, CRLF, sem payload upstream copiado |
| `verify_remoteapp_marker` | marker ok, ausente, JSON inválido |
| `prepare_remoteapp` | `/oem` ok, `/oem` falhou, adoção no-op falhou |

### Mocks Necessários
- `MockGuestExecutor`.
- Diretório compartilhado temporário.

## Detalhes de Implementação

Ver techspec §§Sequência de Fases, Execução no Guest, Paradoxo RemoteApp, Fronteira AGPL/FR-3.6.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- remoteapp` passa.
- Nenhuma cópia de `RDPApps.reg` entra no repositório.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte RemoteApp passando.

## Arquivos Relevantes
- `src-tauri/src/core/guest_executor.rs`
- `src-tauri/src/core/office_state.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): prepare remoteapp before guest execution"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — restringe origem dos artefatos RemoteApp.
- `adrs/adr-rdp-cert-ignore-vs-tofu.md` — afeta sessão RDP usada para no-op.

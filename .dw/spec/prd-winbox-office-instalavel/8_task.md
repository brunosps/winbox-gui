---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 8.0: Gerar staging ODT, XML e integridade do instalador

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Implementar o staging do Office Deployment Tool no share do perfil, gerar `configuration.xml` canônico e validar a integridade do `OfficeSetup.exe` antes da instalação.

**Requisitos Funcionais cobertos**: FR-4.2, FR-4.7

**Depende de**: 3.0, 7.0.

**Constitution**: respects P-001, P-002, P-004, P-005, P-006.

<requirements>
- Gerar XML com Product ID escolhido, `OfficeClientEdition=64`, `Channel=Current`, idioma e `ExcludeApp`.
- Preservar regra de reapply: `ExcludeApp` não pode virar merge acidental.
- Pré-stage do instalador no share do perfil com CRLF nos scripts auxiliares.
- Validar `OfficeSetup.exe` por SHA256 conhecido por release ou Authenticode no guest.
- Retornar `office_odt_stage_failed` para falha de host.
</requirements>

## Subtarefas

### Implementação
- [ ] 8.1 Criar `core/office_odt.rs` com renderização pura do XML.
- [ ] 8.2 Implementar staging do `OfficeSetup.exe` e scripts auxiliares no share.
- [ ] 8.3 Implementar verificação de integridade e fallback para rotação da Microsoft.
- [ ] 8.4 Integrar Product ID e idioma vindos de `config.env`.

### Testes Automatizados
- [ ] 8.5 Teste `odt_configuration_excludes_only_supported_apps` — invariante: XML contém a lista canônica de `ExcludeApp`; camada: unit Rust core/office_odt.
- [ ] 8.6 Teste `odt_reapply_replaces_exclude_apps_instead_of_merging` — invariante: reaplicação não acumula apps antigos; camada: unit Rust core/office_odt.
- [ ] 8.7 Teste `odt_stage_host_failure_returns_office_odt_stage_failed` — invariante: falha de download/escrita no share tem code correto; camada: integração commands com mock de filesystem/processo.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| `render_configuration_xml` | pt-br, en-us, Product IDs válidos, Product ID inválido recusado antes |
| `stage_odt_assets` | setup já existe, download ok, erro de escrita, checksum divergente |

### Mocks Necessários
- Diretório temporário do share.
- Trait de comando host para `curl`/verificação quando necessário.

## Detalhes de Implementação

Ver techspec §§ODT, Modelos de Dados, Execução no Guest, Riscos.

## Critérios de Sucesso

- `cargo fmt --check` passa.
- `cargo test --manifest-path src-tauri/Cargo.toml -- office_odt` passa.
- Nenhuma dependência Rust nova é adicionada.
- Cobertura consolidada na task 29.0; nesta task o gate local é a suíte ODT passando.

## Arquivos Relevantes
- `src-tauri/src/core/office_odt.rs`
- `src-tauri/src/core/validation.rs`
- `src-tauri/src/core/guest_executor.rs`
- `src-tauri/src/commands/office.rs`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/src/core src-tauri/src/commands/office.rs
git commit -m "feat(office): stage odt installer and configuration"
```

## Related ADRs

- `adrs/adr-office-odt-download-strategy.md` — define o que é staged no host e o que baixa no guest.

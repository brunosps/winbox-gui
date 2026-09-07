---
type: task
schema_version: "1.0"
status: pending
---

# Tarefa 26.0: Ajustar empacotamento, atribuições e CI de release

<critical>Ler os arquivos de prd.md e techspec.md desta pasta, se você não ler esses arquivos sua tarefa será invalidada</critical>

## Visão Geral

Ajustar release para `.deb` e AppImage, checksums, baseline Ubuntu 22.04, aviso `libfuse2t64` e atribuições/licenças de terceiros.

**Requisitos Funcionais cobertos**: FR-1.1, FR-3.7

**Depende de**: 13.0, 16.0, 23.0.

**Constitution**: respects P-005, P-008.

<requirements>
- Fixar targets Tauri `["deb","appimage"]`.
- Build de release em Ubuntu 22.04.
- Gerar `SHA256SUMS` em step extra do CI.
- Não adicionar `tauri-plugin-updater` no MVP.
- Criar/atualizar `THIRD-PARTY.md` com WinApps, FreeRDP, dockur, ODT e dependências relevantes.
- Expor atribuições no wizard/superfície do app.
- Documentar aviso AppImage `libfuse2t64` para Ubuntu 24.04.
</requirements>

## Subtarefas

### Implementação
- [ ] 26.1 Ajustar `src-tauri/tauri.conf.json` para targets explícitos.
- [ ] 26.2 Atualizar workflow de release para Ubuntu 22.04 e `SHA256SUMS`.
- [ ] 26.3 Criar/atualizar `THIRD-PARTY.md`.
- [ ] 26.4 Adicionar superfície de atribuições no wizard/app e docs de `libfuse2t64`.

### Testes Automatizados
- [ ] 26.5 Teste `tauri_bundle_targets_are_deb_and_appimage` — invariante: release não publica target inesperado; camada: script/CI config check.
- [ ] 26.6 Teste `release_workflow_generates_sha256sums` — invariante: checksums são artefato do release; camada: CI config check.
- [ ] 26.7 Teste `third_party_attribution_surface_is_localized` — invariante: link/superfície existe em pt-BR e en-US; camada: node:test JS puro.

## Testes Unitários

### Casos a Testar

| Método | Casos |
|--------|-------|
| config/release check | targets corretos, workflow Ubuntu 22.04, step SHA256SUMS |
| `renderThirdPartyAttributions` | link presente, locales paralelos, escape |

### Mocks Necessários
- Leitura de JSON/YAML do repo em teste.
- DOM fake para superfície de atribuições.

## Detalhes de Implementação

Ver techspec §§Tauri Bundler, Empacotamento/CI, Related ADRs, Conformidade com Padrões.

## Critérios de Sucesso

- `npm run check:js` passa se UI for tocada.
- `npm run test:js` passa se testes JS forem adicionados.
- `cargo test --manifest-path src-tauri/Cargo.toml` não regride.
- Workflow de release gera `.deb`, AppImage e `SHA256SUMS`.

## Arquivos Relevantes
- `src-tauri/tauri.conf.json`
- `.github/workflows/ci.yml`
- `.github/workflows/*release*`
- `THIRD-PARTY.md`
- `src/office-wizard.js`
- `src/locales/en-US.js`
- `src/locales/pt-BR.js`

## Commit ao Final

Ao completar esta task, fazer commit:
```bash
git add src-tauri/tauri.conf.json .github/workflows THIRD-PARTY.md src
git commit -m "ci(office): package deb appimage and publish attributions"
```

## Related ADRs

- `adrs/adr-agpl-winapps-runtime-boundary.md` — fundamenta atribuições e fronteira AGPL.

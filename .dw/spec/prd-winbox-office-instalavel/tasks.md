---
type: tasks-index
schema_version: "1.0"
status: draft
---

# Resumo de Tarefas de Implementação do Perfil Office Instalável

## Branch

```bash
feat/prd-winbox-office-instalavel
```

## Projetos Impactados

- [ ] Backend Rust/Tauri (`src-tauri`)
- [ ] Frontend JS puro, CSS e i18n (`src`)
- [ ] Harness E2E WebdriverIO (`tests/e2e`)
- [ ] CI/release e empacotamento (`.github/workflows`, `src-tauri/tauri.conf.json`)
- [ ] Documentação, ADRs e atribuições (`.dw/spec/.../adrs`, `THIRD-PARTY.md`)
- [ ] Infra externa de telemetria no Coolify do dono

## Tarefas

| Task | Descrição | FRs | Depende de | Status |
|------|-----------|-----|------------|--------|
| 1.0 | Registrar ADRs obrigatórios da feature Office | FR-3.5, FR-3.6 | - | Concluída |
| 2.0 | Modelar estado persistido, fases e retry em cascata | FR-6.1, FR-6.2 | 1.0 | Concluída |
| 3.0 | Validar configuração Office e imutabilidade do perfil | FR-4.3, FR-4.4 | 2.0 | Concluída |
| 4.0 | Implementar preflight base com erros acionáveis | FR-2.1, FR-2.2 | 3.0 | Concluída |
| 5.0 | Detectar FreeRDP nativo/Flatpak e preparar fallback | FR-2.4 | 4.0 | Concluída |
| 6.0 | Detectar recursos insuficientes e conflitos de subnet | FR-2.3, FR-2.5 | 4.0 | Concluída |
| 7.0 | Preparar RemoteApp antes do executor guest | FR-3.6, FR-5.2 | 2.0, 3.0, 4.0 | Concluída |
| 8.0 | Gerar staging ODT, XML e integridade do instalador | FR-4.2, FR-4.7 | 3.0, 7.0 | Concluída |
| 9.0 | Executar instalação Office no guest e verificar readiness | FR-5.1, FR-5.5 | 7.0, 8.0 | Concluída |
| 10.0 | Expor OfficeError, comandos Tauri e progresso estruturado | FR-1.3, FR-4.5 | 2.0, 9.0 | Concluída |
| 11.0 | Implementar CLI `winbox office` com envelope preservado | FR-1.3, FR-5.6 | 10.0 | Concluída |
| 12.0 | Integrar WinApps gerenciado, configuração e setup | FR-3.5, FR-5.3 | 5.0, 9.0, 10.0 | Concluída |
| 13.0 | Registrar desktop, MIME e verificação final | FR-5.4, FR-5.5 | 12.0 | Concluída |
| 14.0 | Detectar adoção manual e revisão segura | FR-6.3, FR-6.4 | 2.0, 10.0, 12.0 | Concluída |
| 15.0 | Remover perfil Office com confirmação e sem destruição acidental | FR-6.5, FR-7.5 | 11.0, 12.0, 13.0, 14.0 | Concluída |
| 16.0 | Implementar launcher Office, progresso GUI e validação de arquivos | FR-7.4, FR-7.7 | 10.0, 11.0, 12.0, 13.0 | Concluída |
| 17.0 | Integrar ciclo de vida Office e aviso de apps abertos | FR-7.1, FR-7.6 | 10.0, 16.0 | Concluída |
| 18.0 | Criar shell do wizard Office e entrada no app | FR-1.2, FR-4.1 | 10.0 | Concluída |
| 19.0 | Implementar aceite BYOL e bloqueios legais no wizard | FR-3.1, FR-3.2 | 18.0 | Concluída |
| 20.0 | Implementar UI de preflight, warnings e adoção | FR-2.2, FR-6.4 | 4.0, 6.0, 14.0, 18.0 | Concluída |
| 21.0 | Implementar UI de provisionamento, duração e progresso | FR-4.5, FR-4.6 | 10.0, 11.0, 18.0 | Concluída |
| 22.0 | Implementar orientação pós-launch, ativação e i18n final | FR-7.2, FR-7.3 | 16.0, 18.0 | Concluída |
| 23.0 | Publicar escopo single-user e orientação de licença | FR-3.3, FR-3.4 | 1.0, 18.0 | Concluída |
| 24.0 | Implementar cliente de telemetria opt-in | FR-8.1, FR-8.2 | 18.0 | Concluída |
| 25.0 | Implementar endpoint mínimo de telemetria beta | FR-8.3, FR-8.4 | 24.0 | Concluída |
| 26.0 | Ajustar empacotamento, atribuições e CI de release | FR-1.1, FR-3.7 | 13.0, 16.0, 23.0 | Concluída |
| 27.0 | Corrigir harness E2E e cobrir fluxo do wizard | FR-4.6 | 20.0, 21.0, 22.0, 26.0 | Concluída |
| 28.0 | Cobrir E2E de launch, lifecycle e upgrade Office | FR-1.4, FR-7.7 | 16.0, 17.0, 26.0, 27.0 | Concluída |
| 29.0 | Fechar gates de cobertura e regressão de contrato | FR-1.4, FR-8.4 | 10.0, 18.0, 26.0, 27.0, 28.0 | Pendente |

## Progresso

- [x] 1.0 Registrar ADRs obrigatórios da feature Office
- [x] 2.0 Modelar estado persistido, fases e retry em cascata
- [x] 3.0 Validar configuração Office e imutabilidade do perfil
- [x] 4.0 Implementar preflight base com erros acionáveis
- [x] 5.0 Detectar FreeRDP nativo/Flatpak e preparar fallback
- [x] 6.0 Detectar recursos insuficientes e conflitos de subnet
- [x] 7.0 Preparar RemoteApp antes do executor guest
- [x] 8.0 Gerar staging ODT, XML e integridade do instalador
- [x] 9.0 Executar instalação Office no guest e verificar readiness
- [x] 10.0 Expor OfficeError, comandos Tauri e progresso estruturado
- [x] 11.0 Implementar CLI `winbox office` com envelope preservado
- [x] 12.0 Integrar WinApps gerenciado, configuração e setup
- [x] 13.0 Registrar desktop, MIME e verificação final
- [x] 14.0 Detectar adoção manual e revisão segura
- [x] 15.0 Remover perfil Office com confirmação e sem destruição acidental
- [x] 16.0 Implementar launcher Office, progresso GUI e validação de arquivos
- [x] 17.0 Integrar ciclo de vida Office e aviso de apps abertos
- [x] 18.0 Criar shell do wizard Office e entrada no app
- [x] 19.0 Implementar aceite BYOL e bloqueios legais no wizard
- [x] 20.0 Implementar UI de preflight, warnings e adoção
- [x] 21.0 Implementar UI de provisionamento, duração e progresso
- [x] 22.0 Implementar orientação pós-launch, ativação e i18n final
- [x] 23.0 Publicar escopo single-user e orientação de licença
- [x] 24.0 Implementar cliente de telemetria opt-in
- [x] 25.0 Implementar endpoint mínimo de telemetria beta
- [x] 26.0 Ajustar empacotamento, atribuições e CI de release
- [x] 27.0 Corrigir harness E2E e cobrir fluxo do wizard
- [x] 28.0 Cobrir E2E de launch, lifecycle e upgrade Office
- [ ] 29.0 Fechar gates de cobertura e regressão de contrato

## Mapa FR -> Task(s)

| FR | Task(s) |
|----|---------|
| FR-1.1 | 26.0 |
| FR-1.2 | 18.0 |
| FR-1.3 | 10.0, 11.0 |
| FR-1.4 | 28.0, 29.0 |
| FR-2.1 | 4.0 |
| FR-2.2 | 4.0, 20.0 |
| FR-2.3 | 6.0 |
| FR-2.4 | 5.0 |
| FR-2.5 | 6.0 |
| FR-3.1 | 19.0 |
| FR-3.2 | 19.0 |
| FR-3.3 | 23.0 |
| FR-3.4 | 23.0 |
| FR-3.5 | 1.0, 12.0 |
| FR-3.6 | 1.0, 7.0 |
| FR-3.7 | 26.0 |
| FR-4.1 | 18.0 |
| FR-4.2 | 8.0 |
| FR-4.3 | 3.0 |
| FR-4.4 | 3.0 |
| FR-4.5 | 10.0, 21.0 |
| FR-4.6 | 21.0, 27.0 |
| FR-4.7 | 8.0 |
| FR-5.1 | 9.0 |
| FR-5.2 | 7.0 |
| FR-5.3 | 12.0 |
| FR-5.4 | 13.0 |
| FR-5.5 | 9.0, 13.0 |
| FR-5.6 | 11.0 |
| FR-6.1 | 2.0 |
| FR-6.2 | 2.0 |
| FR-6.3 | 14.0 |
| FR-6.4 | 14.0, 20.0 |
| FR-6.5 | 15.0 |
| FR-7.1 | 17.0 |
| FR-7.2 | 22.0 |
| FR-7.3 | 22.0 |
| FR-7.4 | 16.0 |
| FR-7.5 | 15.0 |
| FR-7.6 | 17.0 |
| FR-7.7 | 16.0, 28.0 |
| FR-8.1 | 24.0 |
| FR-8.2 | 24.0 |
| FR-8.3 | 25.0 |
| FR-8.4 | 25.0, 29.0 |

## Workflow

Cada task segue o fluxo:
1. `/dw-run [N]_task.md` - implementa a task.
2. Releitura obrigatória de `prd.md` e `techspec.md` antes de editar.
3. Testes automatizados incluídos na menor camada que detecta o defeito.
4. Commit ao final da task, sem push.
5. Próxima task ou `/dw-generate-pr [branch-alvo]` quando todas concluídas.

## Gates Locais do Plano

- Cada task cobre no máximo 2 FRs.
- Toda dependência aponta para task anterior e o grafo está em ordem topológica.
- `cargo fmt --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run check:js` e `npm run test:js` entram conforme a camada tocada.
- Cobertura é consolidada na task 29.0: `cargo-llvm-cov` perto de 80% nos módulos `core/office_*` novos e 70% nos módulos `commands` novos; JS medido por `node --test --experimental-test-coverage` ou `c8` no job de CI.

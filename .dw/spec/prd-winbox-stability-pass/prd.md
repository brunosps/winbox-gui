---
type: prd
schema_version: "1.0"
status: draft
slug: prd-winbox-stability-pass
created: 2026-05-17
---

# PRD — winbox-gui Stability Pass

## Overview

`winbox-gui` recém-saiu de um round de bugfixes pontuais (commit `7f32333`: stale container rm, noVNC autoconnect, xfreerdp stderr log, image pin 5.14/7.29). Os bugs imediatos foram resolvidos, mas o projeto continua frágil em três eixos: zero teste de integração no caminho install→launch, mensagens de erro genéricas (`bail!("Timeout. Verifique...")`) que não orientam o usuário, e nenhum gate de CI prevenindo regressão.

Este PRD encerra esse débito. Não muda nenhuma feature visível; coloca infra de testes, errors tipados, CI, e documentação de troubleshooting para que o projeto passe de "funciona na minha máquina" a "funciona reprodutivelmente e quebra cedo quando algo regride".

## Goals

- **Reduzir tempo de diagnóstico** de falha de launch de "ler log raw" para "ler código + hint" via `LaunchError` estruturado serializado pra UI.
- **Prevenir regressão** das classes de bug recém-corrigidos (stale container, image pin, noVNC autoconnect, FreeRDP silent fail) com testes determinísticos.
- **Estabelecer gate de qualidade** automático em PRs (cargo test + clippy -D warnings + JS tests + node --check) sem fricção visível pro desenvolvedor.
- **Documentar** os 4 sintomas mais comuns descobertos nesta sessão pra usuários finais (PT-BR) + contributors (EN-US).

Métrica de sucesso (binária):

1. `cargo test` retorna >18 testes verdes (12 atuais + ≥6 novos).
2. `npm run test:js` retorna ≥6 testes verdes (3 atuais + ≥3 novos).
3. `cargo clippy --all-targets -- -D warnings` retorna 0 warnings.
4. CI roda em PRs e em push para `main`.
5. TROUBLESHOOTING.md + docs/DEBUGGING.md existem na raiz / `docs/`.

## User Stories

- Como **desenvolvedor do winbox-gui** abrindo um PR, quero saber em <2min se ele quebra alguma classe de erro conhecida, sem precisar lembrar de rodar os testes localmente.
- Como **usuário final** que clica "Iniciar" e nada acontece, quero ver um toast com **código** + **dica acionável** (e.g., `KvmDenied — usuário não está no grupo kvm. Rode: sudo usermod -aG kvm $USER && reboot`) em vez de "Falha ao iniciar perfil".
- Como **contributor** investigando bug remotamente, quero um doc com fluxo de diagnóstico (ler quais logs, qual `docker inspect`, qual env file) para não depender de sessão síncrona com o maintainer.
- Como **maintainer** quero garantir que mudanças no template de `compose.yml` continuam emitindo image tags pinned (não regredindo para `:latest`).

## Core Features

### RF-01 — DockerClient trait + DockerCli/MockDocker

**O quê**: Extrair as 7 funções de `core::docker.rs` que dependem do Command spawn (`container_status`, `compose_run`, `logs`, `pause`, `unpause`, `stop`, `kill`, `rm_force`, `pull`) atrás de um trait `DockerClient`. Manter struct `CliDocker` como wrapper de `Command::new("docker")` (default no runtime). Adicionar `MockDocker` em `#[cfg(test)]` com cenários scriptáveis (status fila, calls capturadas, falhas injetadas).

**Por quê**: Sem isso, qualquer teste de integração precisa de daemon Docker real — slow, flaky, e impossível em CI sem dind. Trait + injection permite testes determinísticos de `ensure_running`, `start`, `update`, etc.

**Como (alto nível)**:
- Refactor mínimo: assinatura das funções de top-level em `launch.rs`, `lifecycle.rs` ganha `&impl DockerClient`. Hoje elas chamam `docker::funcao()` direto — vamos parametrizar.
- Onde Tauri command chama, instancia `CliDocker` antes de delegar.

### RF-02 — Integration tests para ensure_running

**O quê**: Testes em `src-tauri/src/commands/launch.rs` cobrindo a árvore de decisão de `ensure_running` com MockDocker:

- status `running` → não chama compose_run nem rm_force.
- status `paused` → chama unpause apenas.
- status `absent` → chama compose_run; não chama rm_force.
- status `exited`/`created`/`dead`/`restarting` → chama rm_force ANTES de compose_run.
- compose_run falha → erro propaga, wait_for_ready não é chamado.

Mesmo conjunto para `start` (path no-RDP).

### RF-03 — enum LaunchError + reescrita de wait_for_*

**O quê**: Novo módulo `core::launch_error::LaunchError` com variantes:

```
KvmDenied              // /dev/kvm sem permissão
DockerMissing          // docker não está no PATH
DockerDaemonDown       // daemon inacessível
FreeRdpMissing         // xfreerdp3 não está no PATH
PortConflict { port }  // bind: address already in use detectado nos logs
ImagePullFailed { image, stderr }
ContainerCrash { container, log_tail }
TimeoutWindows { profile }
TimeoutLinux { profile, port }
StaleStateRecovery { previous_status }  // info-level, não erro
```

Cada variante tem `code()` (str estável), `message()` (i18n-aware PT/EN), e `hint()` (comando ou ação). `wait_for_windows`, `wait_for_web_port`, `launch_rdp`, `ensure_for_web_vnc` retornam `Result<_, LaunchError>` em vez de `anyhow::Result`. A camada Tauri converte em `{code, message, hint}` JSON antes de map_err.

### RF-04 — Frontend exibe erro estruturado

**O quê**: `src/main.js` ganha `renderLaunchError({code, message, hint})` que mostra toast colorido por categoria (vermelho/amarelo) e — se houver `hint` com comando shell — botão "Copiar comando" usando `navigator.clipboard`. Strings de mensagem ficam em `src/locales/{pt-BR,en-US}.js` com chaves `launch.error.<code>`.

### RF-05 — JS tests para modeMeta + primaryActionMeta + profileState

**O quê**: Extrair funções pure de `src/main.js` (`modeMeta`, `primaryActionMeta`, `profileState`) para `src/profile-display.js` (módulo separado, importado por `main.js`). Adicionar `src/profile-display.test.js` com node:test cobrindo:

- modeMeta para connect_mode `rdp` / `web_vnc` / undefined.
- primaryActionMeta para state `running` / `paused` / `exited` / `absent`.
- profileState retorna o mapeamento esperado para cada container_state input.

### RF-06 — GitHub Actions CI

**O quê**: `.github/workflows/ci.yml` rodando em PR e push pra `main`:

```
jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - actions/checkout@v4
      - actions/cache@v4 (target/, ~/.cargo)
      - cargo fmt --check
      - cargo clippy --all-targets -- -D warnings
      - cargo test
  js:
    runs-on: ubuntu-latest
    steps:
      - actions/checkout@v4
      - actions/setup-node@v4 (20.x)
      - npm ci
      - npm run check:js
      - npm run test:js
```

Sem matrix por SO — Tauri build full não cabe ainda; isso fica de fora desse PRD.

### RF-07 — TROUBLESHOOTING.md (PT-BR, usuários)

**O quê**: Arquivo na raiz cobrindo 4 sintomas observáveis:

1. **"Cliquei Iniciar e não acontece nada"** — provável FreeRDP missing. Como conferir e instalar.
2. **"Janela VNC abre toda preta"** — provável guest com GPU passthrough escondendo display. Como diagnosticar via screendump QMP.
3. **"docker compose up erro de container name in use"** — explica o fix automático que landed e diz quando intervir manual.
4. **"Windows nunca termina de iniciar (240s)"** — primeira instalação baixa ISO; como inspecionar progresso via `winbox logs`.

Cada seção: sintoma → diagnóstico (1-3 comandos) → fix → onde fica o log de evidência.

### RF-08 — docs/DEBUGGING.md (EN-US, contributors)

**O quê**: Arquivo em `docs/` com mapa interno:

- Onde cada camada faz log (qual arquivo, qual stream).
- Como rodar a app em modo dev com tracing verboso.
- Caminho do AppData/cache/config XDG.
- Como simular condições de erro localmente (forçar exit do container, deletar /dev/kvm, etc.).
- Referência rápida a `LaunchError` codes ↔ source location.

### RF-09 — Clippy clean

**O quê**: Rodar `cargo clippy --all-targets -- -D warnings` e zerar warnings. Aplicar fixes diretos onde óbvio (`.map(|_| ())` em vez de `.map(|_| Ok(()))`, `&str` em vez de `&String`, etc.). Se algum warning for false positive justificado, adicionar `#[allow(clippy::<lint>)]` inline com comentário explicando.

## User Experience

- **Erro de launch agora carrega contexto**: hoje o toast diz `Falha ao iniciar perfil`. Após este PRD: `KvmDenied: KVM não disponível. Rode: sudo usermod -aG kvm $USER && reboot`. O `code` permite estilizar o toast e fazer matching em telemetria futura.
- **CI verde em <5min**: o developer recebe feedback rápido em PR. Cache `~/.cargo` + `target/` cortam build incremental.
- **Tela do TROUBLESHOOTING.md é GitHub-renderizável**: usuário pode chegar nela por link direto no README.

Acessibilidade: copy-to-clipboard buttons no toast de erro precisam de aria-label e fallback se `navigator.clipboard` indisponível (raro em Tauri WebView mas defensivo). Mensagens i18n respeitam o locale atual da UI.

## High-Level Technical Constraints

- **Stack travada**: Tauri 2.0, Rust 1.75, Node ≥20. Sem novas dependências major (não vamos adicionar Vitest, Playwright, ou tracing crate neste PRD — backlog).
- **Não quebrar API IPC**: `launch_profile` continua existindo com mesma assinatura. O `Result<_, String>` que volta agora pode conter JSON em vez de string raw (frontend faz try-parse).
- **CI no GitHub Actions público**: assumindo repo público ou Actions habilitado. Sem secrets externos necessários.
- **DockerClient trait não pode vazar pro CLI**: o CLI standalone (`src-tauri/src/cli.rs`) continua usando `CliDocker` direto. Trait é detalhe de implementação, não API pública.
- **Sem mexer em styles.css / index.html**: redesign UI é PRD separado.

## Out of Scope

- Phase 2 cloud-init xrdp pra Linux distros (backlog).
- Migração Vitest / Playwright / tracing crate.
- Matrix CI (macOS, Windows runners — Tauri build cross-platform fica pra release pipeline futura).
- Integração com observabilidade externa (Sentry / Datadog).
- Testes E2E rodando GUI Tauri real (precisa display server + ferramentas que ainda não vetamos).
- Hardening de segurança extra (esse projeto roda local, não rede pública).

## Open Questions

- O `LaunchError::PortConflict { port }` detecta via parsing do stderr do `docker compose` — qual regex robusta? **Decisão**: começar com `bind: address already in use` literal; se aparecer outra variante, adicionar caso conforme reincidir.
- Como populamos `LaunchError::ContainerCrash { log_tail }` sem custo de chamar `docker logs` em todo erro? **Decisão**: só populamos quando `wait_for_*` deu timeout E o container já não está running.
- Quanto de `dom-utils.js` cobrir nos testes JS? **Decisão**: manter o que já existe; foco em `profile-display.js` extraído.

## Related ADRs

(nenhuma ADR existente; se decisões emergirem na execução vamos rodar `/dw-adr`)

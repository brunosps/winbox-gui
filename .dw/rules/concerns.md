---
schema_version: "1.0"
generated_by: dw-analyze-project (Step 9)
last_refreshed: "2026-07-08"
---

# Concerns — Mapa de Riscos

Mapa de riscos deste codebase. Nao sao convencoes ("como fazemos as coisas" — isso e `.dw/rules/`), nao e arquitetura ("como esta construido" — isso e `.dw/intel/arch.md`). Este arquivo responde uma unica pergunta: **onde e perigoso mexer?**

Carregado on-demand por `/dw-plan`, `/dw-run` e `/dw-bugfix` quando o alvo deles toca uma entrada abaixo. Auto-instalado pelo `/dw-analyze-project` Step 9; nunca bloqueia (ausencia = nenhuma area flagada ainda).

## Hot Spots

Arquivos ou modulos com churn alto. Sem historico de bugfix em `.dw/bugfixes/` ainda — todas as entradas sao "churn alto, verificar com o time"; a proxima analise cruza com bugfixes reais.

| Path | Por que e quente | Primeiro flag | Ultimo incidente |
|------|------------------|---------------|------------------|
| `src-tauri/src/lib.rs` | 12 commits/90d; entry point com 31 comandos e 5 handlers de lifecycle duplicados (457-638) | 2026-07-08 | — |
| `src-tauri/src/commands/launch.rs` | 9 commits/90d; máquina de estados do container + waits por string mágica/TCP | 2026-07-08 | — |
| `src-tauri/src/core/docker.rs` | 6 commits/90d; classificação de stderr do vendor + god node (Ca=10) | 2026-07-08 | — |
| `src/main.js` | 6 commits/90d; hub de 1734 linhas com zero testes unitários | 2026-07-08 | — |

## Integracoes Fragis

| Integracao | Modo de falha | Mitigacao esperada |
|------------|---------------|--------------------|
| dockurr/windows:5.14 | Boot detectado por string mágica `"windows started successfully"` nos logs (launch.rs:130) — vendor mudar wording = todo launch vira TimeoutWindows; RAM_SIZE reajustado silenciosamente pelo vendor quando não cabe; containers zombie durante setup (ACPI ignorado); 1º boot baixa ISO ~5GB vs timeout 4min | Pin de versão (paths.rs) + bump deliberado; kill = SIGKILL + `rm -f` sempre (lifecycle.rs:24); testar launch após qualquer bump de imagem |
| qemux/qemu:7.29 | `SUPPORTED_DISTROS` (image_family.rs:57) é lista curada à mão do resolver do vendor — drift silencioso a cada release | Ao bumpar a imagem, revalidar a lista contra o resolver; teste garante que distro não-suportada falha |
| Docker / Docker Desktop / WSL | `classify_compose_stderr` (docker.rs:255) e `extract_wsl_distro` dependem de frases exatas EN do stderr; wording novo degrada para `LaunchError::Other` | Fixtures de stderr reais nos testes (padrão já existente) — adicionar fixture nova a cada wording visto em campo |
| cloud-images.ubuntu.com | URLs hardcoded (cloud_init.rs:7-11); `verify_image_hash` re-baixa SHA256SUMS a cada verificação → perfil linux_cloud falha OFFLINE mesmo com imagem cacheada | Cachear SHA256SUMS junto com a imagem; retry `curl --retry 3` já existe |
| FreeRDP flatpak × WinApps (frente Office) | Bug RAIL do freerdp3-x11 3.5.1 (X_CopyArea BadMatch) contornado com Flathub 3.27+; `WAFLAVOR=manual` exige VM de pé; `RDP_FLAGS` (`/cert:ignore +home-drive`), bundle `msoffice` e associação xls/xlsx no Thunar vivem SÓ no host do dono — fora do repo | Absorver no produto via bundle + wizard (one-pager `.dw/spec/ideas/winbox-office-instalavel.md`); até lá, tratar o setup real como não-reprodutível a partir do repo |
| WebKitGTK (webview Tauri) | Não entrega teclado/mouse ao canvas noVNC (lib.rs:407) — razão permanente do viewer abrir no browser padrão | Não tentar "consertar" trazendo o viewer para o webview sem validar o bug upstream primeiro |
| GitHub Releases (canal de distribuição) | Tarballs sem checksum/assinatura; `setup-host-excel.sh` baixa e executa via curl; versão default `v0.1.0` hardcoded no script | Adicionar sha256sums ao release; resolver "latest" via API |
| Contrato textual de erro Rust ↔ JS | Frase literal `"\nPróxima ação:"` replicada nos dois lados (lib.rs:165 × main.js:54) — mudar o wording quebra a tradução silenciosamente; CLI `machine_error_code` (cli.rs:835) classifica por substring de mensagens humanas PT/EN, exposto a clia/neodrive | Código novo usa codes estáveis (padrão LaunchError); não reescrever mensagens de erro sem checar os dois consumidores |

## Codigo Hostil

| Path / funcao | Por que e hostil | Owner / contexto |
|---------------|------------------|------------------|
| `core/cloud_init.rs::user_data` + `desktop_bootstrap` (239-468) | ~200 linhas de cloud-config gerado: escaping simultâneo Rust `{{}}` × YAML × bash heredoc × systemd units; 1 espaço de indentação errado quebra o cloud-init silenciosamente dentro da VM | Testes assertam conteúdo do YAML — rodá-los é obrigatório; entender os 2 escapers próprios (yaml_single_quote, shell_quote) antes de editar |
| `core/vfio_setup.rs::build_setup_script` (103-139) | Script sh gerado por format! e executado como ROOT via pkexec — escreve /etc/modprobe.d e rebuilda initramfs do host | Inputs passam por validate_bdf/normalize_id; qualquer campo novo interpolado PRECISA de validação equivalente |
| `core/gpu.rs::list/split_vendor_model` (29-151) | Parser custom do lspci com heurísticas encadeadas (classe, ' Corporation', tamanho de string) — ZERO testes; variação de formato degrada sem erro | Adicionar fixture de lspci real antes de mexer |
| `core/compose.rs::render_*` (82-263) | 4 templates YAML por concatenação format! com indentação embutida nos literais — é a única "fonte do Dockerfile" do produto, nada versionado para diffar | Testes de invariantes (pins, memlock) existem; rodar VM real após mudança |
| `cli.rs::dispatch` + `dispatch_json` (288-715) | ~30 subcomandos duplicados nos dois switches; comando novo entra em DOIS lugares ou a superfície `--json` (contrato clia/neodrive) fica dessincronizada | Checar os dois dispatches em todo diff de CLI |
| `src/main.js` (hub inteiro) | 6+ listeners globais de click cuja ordem/stopPropagation importam (leitura de menuProfile ANTES de closeAllMenus); estado global mutável; re-render total 3s invalida referências DOM guardadas | Zero testes; validar manualmente menu flutuante, modais reabertos (dataset.wired) e foco durante refresh |
| `tools/windows-spike/winbox-spike.ps1` | State machine persistida + RunOnce no registry + `Restart-Computer -Force` — mexer errado deixa host Windows em loop de reboot | Exploração órfã (links de contexto quebrados); não usar como referência de padrão |

## Historico de Bugs Conhecidos

`.dw/bugfixes/` vazio — nenhum fix registrado pelo dev-workflow ainda. Cicatrizes documentadas em comentário no código (pré-dev-workflow):

| Modulo | Contagem de bugs | Slugs recentes |
|--------|------------------|----------------|
| `commands/lifecycle.rs` | 1 (comentário) | "docker kill não recupera zombie" → kill = SIGKILL + rm -f sempre |
| `core/bootstrap.rs` | 1 (comentário) | bootstrap sobrescreveu distro WSL de usuário → guarda `NeedsConfirmation` |
| `src/main.js` (histórico git) | 2 (fac132b, 4b5f808) | UI freeze em stop/kill → handlers async; teclado noVNC |

## Tech Debt — Reconhecida

| Area | Descricao do debt | Por que fica | Trigger de cleanup |
|------|-------------------|--------------|--------------------|
| `tests/e2e/` | Suíte inoperante: seletores do DOM antigo (`.profile-card` vs `.profile-row`), `mock-docker.sh` referenciado mas inexistente, fora do CI | Redesign "Ops Premium Clean" não atualizou o E2E | Antes de qualquer refactor no main.js (é a única rede de segurança possível do hub) |
| `docs/DEBUGGING.md` + `tools/windows-spike/README.md` | Documentam código removido (`FreeRdpMissing`, `preflight_freerdp`, rdp-*.log) e linkam `.dw/spec/prd-windows-port/` apagado (commit f737289) | Remoção do tooling dw antigo decapitou o contexto | Próxima sessão de docs; distinguir FreeRDP-viewer (removido) de FreeRDP-WinApps (frente ativa) |
| `core/cloud_init.rs:230`, `cli.rs:748`, locale en-US | Usuário `bruno` hardcoded como default de linux_cloud (com teste blindando) + path pessoal `/home/bruno/winbox-disks` em mensagem de erro e no locale | Produto nasceu como uso próprio | Antes de qualquer distribuição externa (MVP do one-pager Office) |
| `src/index.html:428` + `main.js:694` | Feature morta: modal remove-profile com checkbox "Also delete disk files" nunca acionado — remove real sempre usa confirmDialog sem flag | Decisão pendente: preservar disco na remoção ou apagar o caminho morto | Primeiro diff que tocar remoção de perfil |
| `core/wsl_autostart.rs` (467) + `core/wsl_config_writer.rs` (301) | 768 linhas públicas sem NENHUM chamador — port Windows aguardando fiação do wizard ou código morto | Port Windows em andamento (windows-build.yml existe) | Decidir no próximo milestone do port: fiar ou remover |
| Duplicações estruturais | `dispatch`×`dispatch_json` (cli.rs); 5 handlers lifecycle copiados (lib.rs:457-638); `build_firstlogon` (reapply.rs) × oem.rs; posicionamento de menu 3× (main.js) | Funciona; refactor exige testes que ainda não existem | Quando o custo da dupla manutenção morder (novo subcomando CLI / novo bundle) |
| `src/main.js` | 1734 linhas sem teste unitário; lógica pura extraível presa no hub (parseExtraPorts, matchesProfile, profileSort, formatErrorForToast) | Decomposição começou (profile-display.js) e parou | Extrair módulo puro a cada feature nova que tocar o hub |
| `.github/workflows/windows-build.yml` | Build completo Windows (msi+nsis) em TODO push/PR sem filtro de paths; reinstala tauri-cli a cada run | Setup simples | Adicionar `paths:` filter + cache do tauri-cli no próximo toque em CI |

---

**Como manter este arquivo:**

- `/dw-analyze-project` reescreve a cada execucao. Entradas escritas a mao entre `<!-- preserved:start -->` e `<!-- preserved:end -->` sao mantidas.
- Quando um bugfix descobrir uma nova area perigosa, adicione manualmente em Hot Spots e deixe a proxima analise confirmar.
- Promova entradas para `.dw/constitution.md` quando virarem regras nao-negociaveis ("nunca toque X sem ADR").

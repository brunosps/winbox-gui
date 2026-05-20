# winbox-gui

GUI e CLI em Tauri 2 para gerenciar VMs Windows/Linux via Docker/QEMU.

O app lista perfis, mostra status ao vivo, cria perfis, abre o viewer
noVNC no navegador, altera recursos, aplica bundles, gerencia GPU VFIO,
expõe snapshots e faz diagnóstico operacional do host antes de ações
críticas.

> **Conexão via browser**: tanto perfis Windows quanto Linux são
> acessados pelo viewer noVNC servido pelo dockurr/qemux na `WEB_PORT`
> (aberto no navegador padrão). Não há mais dependência de clientes RDP
> standalone (`xfreerdp3`/`mstsc`) no host.

## Stack

- Backend: Rust + Tauri 2, com módulos em `src-tauri/src/commands` e `src-tauri/src/core`
- Frontend: HTML + CSS + JavaScript vanilla em `src`
- Auto-refresh do dashboard a cada 3s
- Cache de health check do host por 30s para evitar probes pesados em loop
- Config por perfil em `$XDG_CONFIG_HOME/winbox/profiles`
- Dados por perfil em `$XDG_DATA_HOME/winbox/profiles`

## Pré-requisitos

1. Rust toolchain
2. Node.js
3. Docker com Compose v2 ou `docker-compose`
4. KVM disponível em `/dev/kvm`
5. Libs de sistema para Tauri 2 no Linux:

```bash
sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev \
  build-essential curl wget file pkg-config
```

## Setup

```bash
npm install
```

## Desenvolvimento

```bash
npm run dev
```

## Verificação

```bash
npm run check:js
npm run test
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Build

```bash
npm run build
```

Artefatos ficam em `src-tauri/target/release/bundle/` quando bundling estiver ativo.

## Problemas comuns?

Veja [TROUBLESHOOTING.md](TROUBLESHOOTING.md) — cobre os sintomas mais
frequentes (storage em drvfs lento, RAM ajustada automaticamente pelo
dockurr, container travado/zombie, tela preta no VNC por GPU
passthrough, conflito de container name, timeout do primeiro boot do
Windows). Para detalhes internos, veja [docs/DEBUGGING.md](docs/DEBUGGING.md).

## Funcionalidades

- Perfis Windows com bundles PowerShell, acessados pelo browser (noVNC).
- Perfis Linux por distro/ISO, também via browser (noVNC).
- Criação de perfil pelo menu **"Novo perfil"** (Windows / Linux distro /
  ISO personalizada), com o modal focado só no tipo escolhido.
- **Local de armazenamento configurável** por perfil — escolha onde o
  disco da VM vive. Caminhos em `/mnt/<letra>/` (drvfs do Windows) são
  **rejeitados** por serem lentos demais (~104 MB/s vs. ~3 GB/s no ext4
  da distro); use um caminho dentro do WSL (ex.: `~/winbox-disks/...`),
  que grava no mesmo disco físico mas via I/O nativo.
- Configuração de RAM, CPU, disco, portas extras e GPU.
- Snapshots e rollback do storage.
- Reapply de bundles copiando `winbox-reapply.ps1` para a pasta compartilhada.
- GPU passthrough via VFIO com configuração persistente de host (Linux).
- Host Health com Docker, Compose, KVM, storage, portas, perfis e GPU/VFIO.
  Erros de launch são classificados em variantes específicas
  (`wsl_distro_down`, `container_oom_killed`, `disk_full`,
  `storage_path_invalid`, etc.) com dica acionável.
- Progresso de `docker pull` em tempo real (camadas + bytes).
- Timeline de operações longas via eventos Tauri `operation-progress`.

## Segurança Operacional

- Senha de Windows deve ser informada explicitamente; não há senha padrão na GUI.
- `config.env` contém segredos e é gravado com permissão `0600` em Unix.
- A UI escapa dados dinâmicos vindos de perfis, bundles, GPU e snapshots antes de renderizar.
- O frontend não recebe permissão Tauri de shell.
- Portas publicadas pelo Compose ficam presas em `127.0.0.1`.

## Estrutura

```text
winbox-gui/
├── src/                      # frontend
│   ├── index.html
│   ├── main.js
│   ├── dom-utils.js
│   ├── styles.css
│   └── locales/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # escolhe CLI ou GUI
│   │   ├── lib.rs            # comandos Tauri
│   │   ├── cli.rs            # CLI winbox
│   │   ├── commands/         # fluxos de alto nível
│   │   └── core/             # Docker, Compose, perfis, validação
│   ├── capabilities/
│   ├── assets/
│   └── tauri.conf.json
└── package.json
```

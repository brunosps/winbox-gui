# Integrações — winbox-gui

> Auto-gerado por /dw-analyze-project em 2026-07-08. Os 4 contratos que definem o produto:
> Tauri IPC (interno), dockurr/qemux (VMs), WinApps/FreeRDP (frente Office) e o contrato de
> release para consumidores externos (clia, neodrive).

## Grafo de Dependências

```
src/main.js ──invoke/listen──► src-tauri/src/lib.rs (31 comandos, 2 eventos)
src-tauri (core/docker.rs) ──std::process──► docker compose ──► dockurr/windows:5.14 | qemux/qemu:7.29
winapps (externo, ~/code/winapps) ──FreeRDP flatpak :RDP_PORT──► VM Windows (RemoteApp)
clia / neodrive ──tarball release + CLI --json/--progress jsonl──► binário winbox
scripts/setup-host-excel.sh ──curl GitHub Release──► binário winbox + clone WinApps upstream
```

## 1. Contrato Frontend ↔ Backend (Tauri IPC)

**Comandos (31)** — invocados via `window.__TAURI__.core.invoke(cmd, args)`; args aceitam
camelCase e snake_case (serde alias no Rust):

| Grupo | Comandos |
|-------|----------|
| Perfis | `list_profiles`, `install_profile`, `update_profile`, `get_profile_config`, `remove_profile`, `set_default_profile` |
| Lifecycle | `launch_profile`, `stop_profile`, `kill_profile`, `pause_profile`, `resume_profile`, `restart_profile`, `update_profile_image` |
| Diagnóstico | `host_health`, `host_info`, `bootstrap_status`, `bootstrap_run_step`, `version`, `get_logs` |
| Snapshots | `list_snapshots`, `create_snapshot`, `rollback_snapshot` |
| Bundles | `list_bundles`, `reapply_bundles` |
| GPU | `list_host_gpus`, `gpu_setup_status`, `gpu_setup_apply`, `gpu_setup_revert` |
| Misc | `list_supported_distros`, `pick_iso_file`, `pick_storage_dir`, `open_web_vnc` |

**Eventos (2):** `operation-progress` `{profile, op, step, status: running|success|error, message}`
(steps semânticos: prepare/preflight/starting/wait_vnc/launching/complete/failed) e
`profile-error` `{profile, op, message}`.

**Formato de erro:** `launch_profile` rejeita com `LaunchError` estruturado
`{"code": "snake_case", ...campos}` → frontend resolve `launch.error.<code>` no i18n (10 códigos
mapeados nos 2 locales). Demais comandos rejeitam com String (cadeia anyhow `{e:#}`), podendo
conter o sufixo textual `"\nPróxima ação: ..."` que o frontend traduz por replace literal —
**mudar essa frase no Rust quebra a tradução silenciosamente**.

## 2. Contrato com dockurr/windows e qemux/qemu

Imagens **pinadas** em `core/paths.rs`: `dockurr/windows:5.14`, `qemux/qemu:7.29` (upgrade =
bump deliberado das constantes; teste de regressão bloqueia `:latest`). O app gera
`compose.yml` por perfil interpolando `config.env` (0600):

| Canal | Chaves |
|-------|--------|
| environment (Windows) | VERSION, RAM_SIZE, CPU_CORES, DISK_SIZE, USERNAME, PASSWORD, LANGUAGE, REGION, KEYBOARD, TZ, ARGUMENTS |
| environment (Linux/qemux) | BOOT, BOOT_INDEX, RAM_SIZE, CPU_CORES, DISK_SIZE, TZ, ARGUMENTS |
| ports (sempre 127.0.0.1) | WEB_PORT→8006 (noVNC), RDP_PORT→3389 tcp+udp, SSH_PORT→22 (linux), DESKTOP_WEB_PORT→6080 (linux_cloud desktop), EXTRA_PORTS |
| volumes | STORAGE_DIR→/storage, SHARED_DIR→/shared (`~/Windows/<perfil>`), OEM_DIR→/oem (Windows), ISO ro→/boot.iso |
| GPU (GPU_BDF setado) | devices /dev/vfio, caps SYS_ADMIN+IPC_LOCK, ulimit memlock -1, ARGUMENTS vfio-pci com kvm=off |

**Acoplamentos frágeis conhecidos** (detalhe em concerns.md): marcador de boot por string mágica
`"windows started successfully"` nos logs; `SUPPORTED_DISTROS` curada à mão contra o resolver do
qemux; classificação de erro por frases exatas do stderr do Docker; dockurr reajusta RAM_SIZE
sozinho; containers zombie durante setup (força `docker rm -f`).

**Regra dura:** NÃO mudar VERSION/LANGUAGE de perfil com disco instalado (dockur faz backup e
reinstala do zero) — `set.rs` deliberadamente não expõe esses campos.

## 3. Contrato WinApps / FreeRDP (frente ativa Office)

O backend **não conhece** FreeRDP/WinApps — apenas publica `RDP_PORT` (3389 tcp+udp em
127.0.0.1). A integração é externa, provisionada por `scripts/setup-host-excel.sh`:

- FreeRDP do **Flathub** (`com.freerdp.FreeRDP` 3.27+) em vez do freerdp3-x11 3.5.1 do Ubuntu
  24.04 (bug RAIL "X_CopyArea BadMatch"); flatpak recebe `--filesystem=home`.
- WinApps clonado do upstream em runtime (`~/code/winapps`) — **copyleft, nunca embutir**.
- `~/.config/winapps/winapps.conf` (fora do repo): `RDP_USER`, `RDP_PASS`, `RDP_IP=127.0.0.1`,
  `RDP_PORT` (do config.env do perfil), `WAFLAVOR="manual"` (VM tem que estar de pé),
  `FREERDP_COMMAND="flatpak run --command=xfreerdp com.freerdp.FreeRDP"`.
- Mecanismo de extensão natural do produto: **bundles .ps1** — user bundles em
  `$XDG_CONFIG_HOME/winbox/bundles` sobrescrevem os builtin sem recompilar (bundles.rs:41-49).
- Peças do stack real ainda FORA do repo: `RDP_FLAGS` (`/cert:ignore +home-drive`), bundle
  `msoffice` + OfficeSetup.exe no share, associação xls/xlsx no Thunar (gap que o one-pager
  `.dw/spec/ideas/winbox-office-instalavel.md` propõe fechar).

## 4. Contrato de release para clia / neodrive

- **Artefatos:** `winbox-<target-triple>.tar.gz` (só o binário CLI) anexados ao GitHub Release da
  tag `v*` — "These tarballs are what downstream apps (e.g. clia.local) bundle via their prepare
  script" (release-cli.yml). Installers Windows (msi/nsis) na mesma tag.
- **Superfície machine-readable:** `winbox --json <cmd>` → envelope `{ok:true, value}` /
  `{ok:false, error:{code, message, hint}}`; `--progress jsonl` → linhas
  `{type, profile, operation, phase, status, message, timestamp}` (RFC3339).
- **Fragilidade:** os `code` do CLI vêm de `machine_error_code()` (cli.rs:835) — heurística por
  substring de mensagens humanas PT/EN; reescrever mensagem muda código exposto a consumidores.
  O backend GUI já tem a solução (LaunchError com codes estáveis) — candidato a unificação.
- Sem checksums/assinatura nos tarballs (consumidor baixa e executa via curl).

## Configuração Compartilhada

| Config | Localização | Consumido por |
|--------|-------------|---------------|
| config.env por perfil | `$XDG_CONFIG_HOME/winbox/profiles/<p>/` | backend (env_file.rs), compose --env-file, WinApps (RDP_PORT via leitura manual) |
| compose.yml por perfil | idem (derivado — sempre regenerado) | docker compose |
| Bundles .ps1 | builtin no binário + `$XDG_CONFIG_HOME/winbox/bundles` (override) | OEM firstlogon + reapply |
| DESIGN.md | raiz do repo | frontend (autoridade de design) |
| winapps.conf | `~/.config/winapps/` (fora do repo) | WinApps/FreeRDP |

## Ordem de Build & Deploy

1. `cargo build` (src-tauri) — frontend é estático, sem build próprio
2. `cargo tauri build` — bundle GUI (deb/rpm/AppImage/msi/nsis)
3. Tag `v*` → release-cli.yml (tarballs 3 OS) + windows-build.yml (installers) → mesmo Release
4. Consumidores: prepare scripts de clia/neodrive + setup-host-excel.sh baixam da Release

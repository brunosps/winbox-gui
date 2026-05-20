# Troubleshooting — winbox-gui

Guia rápido para os sintomas mais comuns observados na prática. Cada
seção segue: **sintoma → diagnóstico → fix → onde fica o log de
evidência**. Para contributors interessados nos detalhes internos,
veja [`docs/DEBUGGING.md`](docs/DEBUGGING.md).

---

## 1. Cliquei "Conectar" e nada (ou abriu a aba errada)

**Sintoma**: clico em "Conectar" e nada acontece, ou a aba do navegador
abre numa URL que não carrega.

**Como funciona hoje**: tanto perfis Windows quanto Linux conectam pelo
**viewer noVNC no navegador** — o app abre
`http://127.0.0.1:<WEB_PORT>/?autoconnect=true&resize=scale` no browser
padrão. **Não há mais dependência de `xfreerdp3`/`mstsc`**; se você viu
o erro `free_rdp_missing` em versões antigas, ele não existe mais.

**Diagnóstico**:

```bash
# o container está de pé e a WEB_PORT respondendo?
docker ps --filter name=winbox-<perfil> --format '{{.Status}} {{.Ports}}'
grep ^WEB_PORT ~/.config/winbox/profiles/<perfil>/config.env
curl -sI http://127.0.0.1:<WEB_PORT>/ | head -1
```

**Fix**: se o container não está rodando, use "Conectar" de novo (ele
faz `compose up`). Se a porta não responde mas o container está up, o
guest ainda está bootando — acompanhe pelos logs (seção 4).

**Log de evidência**: `docker logs winbox-<perfil>`.

---

## 2. Janela VNC abre toda preta ("Display output is not active")

**Sintoma**: ao iniciar um perfil Linux, o noVNC carrega mas exibe
**"Display output is not active."** A VM está rodando, só não enxergamos.

**Diagnóstico** — confira se o perfil tem GPU passthrough ativo:

```bash
grep ^GPU_BDF ~/.config/winbox/profiles/<perfil>/config.env
```

Se vier algo como `GPU_BDF=0000:01:00.0`, o guest Linux pegou conta da
GPU passada via VFIO e está mandando vídeo pela **saída HDMI/DP
física** da placa — o canal virtio-vga (que o noVNC enxerga) fica
ocioso.

**Fix** — escolha:

- **(A) Plugar um monitor físico** na saída da GPU passada.
- **(B) Desligar o passthrough**: edite o `config.env` do perfil,
  deixe `GPU_BDF=` vazio, e reinicie o container.

---

## 3. Storage lento / instalação do Windows trava ou demora horas

**Sintoma**: criei o perfil apontando o "Local de armazenamento" para um
caminho em `/mnt/c`, `/mnt/d`, `/mnt/e`, etc., e a instalação do Windows
fica eternamente no logo "Windows for Docker" ou trava.

**Diagnóstico** — esses caminhos são **drvfs** (a ponte WSL↔Windows),
que entrega ~104 MB/s contra ~3 GB/s do ext4 nativo da distro (30×). VM
disk image em drvfs é inviável.

```bash
grep ^STORAGE_DIR ~/.config/winbox/profiles/<perfil>/config.env
# se começa com /mnt/<letra>/ → é drvfs, problema.
```

**Fix**: o app **rejeita** caminhos `/mnt/<letra>/` na criação do perfil
(tanto no Browse quanto na validação do backend). Use um caminho dentro
do WSL, ex.: `/home/<user>/winbox-disks/<perfil>`. Ele grava no **mesmo
disco físico** (o `ext4.vhdx` da distro), mas via I/O nativo rápido.

Para um perfil já criado errado, edite o `.env` e mova:

```bash
docker rm -f winbox-<perfil>
mkdir -p /home/<user>/winbox-disks/<perfil>
sed -i 's#^STORAGE_DIR=.*#STORAGE_DIR=/home/<user>/winbox-disks/<perfil>#' \
  ~/.config/winbox/profiles/<perfil>/config.env
# conecte de novo pelo app
```

---

## 4. Windows não termina de iniciar (timeout de ~4min)

**Sintoma**: **"Windows '<perfil>' não terminou de iniciar a tempo."**

No noVNC pode aparecer `failed to load Boot0002 "UEFI QEMU HARDDISK" Not
Found` → cai pro `Boot0001 DVD-ROM` → logo "Windows for Docker". **Isso
é normal** numa instalação nova: disco vazio, então o UEFI cai pro DVD
(ISO) e começa o setup. Não é o erro.

**Diagnóstico** — abra os logs do perfil pelo app ("Mais ações" → "Logs")
ou via CLI:

```bash
docker logs --tail 100 winbox-<perfil>
```

A primeira instalação baixa uma ISO de ~5 GB (`Downloading Windows...`)
e roda o setup automatizado. Em drvfs isso leva horas; em ext4, ~20-40min.

**Fix**: espere terminar (acompanhe pelos logs até `Windows started
successfully`). Se estiver em drvfs, mova o storage (seção 3). **Não
reinicie o app nem `wsl --shutdown` durante o setup** — interromper mata
a instalação e ela recomeça do zero.

---

## 5. A VM ficou com menos RAM do que configurei

**Sintoma**: configurei RAM=12G mas o Gerenciador de Tarefas do Windows
mostra 8G (ou outro valor menor).

**Diagnóstico** — o dockurr **ajusta o RAM_SIZE pra caber na memória
livre da distro** (proteção dele). Procure no log:

```bash
docker logs winbox-<perfil> 2>&1 | grep -i 'RAM_SIZE.*too high'
# "Your configured RAM_SIZE of 12 GB is too high for the 9.5 GB available..."
```

A distro WSL tem um teto (`memory=` no `.wslconfig`), e os **outros
containers rodando** (bancos, apps, outras VMs) consomem parte dele.

**Fix** — escolha:

- Parar containers que não estão em uso para liberar RAM, depois
  **Restart** na VM (o dockurr re-detecta a RAM no boot).
- Aumentar `memory=` no `C:\Users\<user>\.wslconfig` e `wsl --shutdown`
  (derruba todos os containers).
- Reduzir o RAM do perfil para um valor que sempre caiba.

---

## 6. Container travado / não para ("Force kill")

**Sintoma**: "Desligar" fica preso, ou aparece `tried to kill container,
but did not receive an exit event`. Comum quando o Windows ainda está em
setup (o dockurr não consegue mandar ACPI durante a instalação).

**Diagnóstico**:

```bash
docker inspect winbox-<perfil> --format '{{.State.Status}} {{.State.Pid}}'
# Pid=0 com Status não-exited = container fantasma
```

**Fix**: use **"Force kill"** no menu do perfil — agora ele faz `docker
kill` (best-effort) seguido de `docker rm -f`, que recupera até
containers em estado zombie. Manualmente:

```bash
docker rm -f winbox-<perfil>
```

O disco da VM persiste no `STORAGE_DIR`; só o container é recriado no
próximo "Conectar".

---

## 7. "Erro no docker compose up" / container name in use

**Sintoma**: `container name "winbox-<x>" is already in use`.

**Fix**:

```bash
docker rm -f winbox-<perfil>
```

E suba pelo app de novo. O app já faz `docker rm -f` defensivo antes de
`compose up` quando o container está em `exited`/`created`/`dead`.

**Log de evidência**: o toast traz o `code` (`docker_daemon_down`,
`image_pull_failed`, `wsl_distro_down`, `disk_full`,
`storage_path_invalid`, etc.) — anote.

---

## Mais

Se nada acima cobre o seu caso, abra um issue com:

1. Saída de `docker ps -a --filter name=winbox`.
2. Conteúdo do toast de erro (com o `code`).
3. Trecho relevante de `docker logs winbox-<perfil>`.
4. `grep -E 'STORAGE_DIR|RAM_SIZE' ~/.config/winbox/profiles/<perfil>/config.env`.

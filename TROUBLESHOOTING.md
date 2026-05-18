# Troubleshooting — winbox-gui

Guia rápido para os sintomas mais comuns observados na prática. Cada
seção segue: **sintoma → diagnóstico → fix → onde fica o log de
evidência**. Para contributors interessados nos detalhes internos,
veja [`docs/DEBUGGING.md`](docs/DEBUGGING.md).

---

## 1. Cliquei "Iniciar" / "Conectar" e nada acontece

**Sintoma**: o botão fica desabilitado, vira ativo de novo, mas
nenhuma janela RDP abre e nenhum toast de erro aparece. Mais comum em
perfis Windows.

**Diagnóstico** — confira se o cliente RDP está instalado:

```bash
xfreerdp3 --version
```

Se o comando não existir, o backend ainda assim chega a `spawn`-ar o
processo, mas a janela do FreeRDP falha em iniciar silenciosamente
porque seu stderr era jogado em `/dev/null` em versões antigas.

**Fix**:

```bash
sudo apt install -y freerdp3-x11   # Ubuntu / Debian
sudo dnf install freerdp           # Fedora
```

Após a versão atual, o app captura stderr e mostra o toast
**"xfreerdp3 não está instalado"** quando o binário falta — então
você deve ver a mensagem em vez do silêncio.

**Log de evidência**: `~/.cache/winbox/rdp-<perfil>.log` (criado a
partir do commit que ativou a captura — se o arquivo não existe,
xfreerdp nem chegou a ser invocado).

---

## 2. Janela VNC abre toda preta ("Display output is not active")

**Sintoma**: ao iniciar um perfil Linux, a janela de VNC carrega o
noVNC mas exibe a mensagem **"Display output is not active."** O VM
está rodando, só não enxergamos.

**Diagnóstico** — confira se o perfil tem GPU passthrough ativo:

```bash
grep ^GPU_BDF ~/.config/winbox/profiles/<perfil>/config.env
```

Se vier algo como `GPU_BDF=0000:01:00.0`, o guest Linux pegou conta da
GPU passada via VFIO e está mandando vídeo pela **saída HDMI/DP
física** da placa — o canal virtio-vga (que o noVNC enxerga) fica
ocioso.

**Fix** — escolha:

- **(A) Plugar um monitor físico** na saída da GPU passada. Esse é o
  uso pretendido de passthrough.
- **(B) Desligar o passthrough**: edite o `config.env` do perfil,
  deixe `GPU_BDF=` vazio, e reinicie o container. O virtio-vga volta a
  desenhar e a noVNC mostra o desktop. Você perde aceleração 3D real.

**Log de evidência**: você pode tirar um screenshot do framebuffer do
QEMU pelo monitor (porta 7100) e converter:

```bash
docker exec winbox-<perfil> sh -c '
  rm -f /tmp/s.ppm
  (sleep 0.5; printf "screendump /tmp/s.ppm\n"; sleep 2; printf "quit\n") \
    | nc localhost 7100 >/dev/null
'
docker cp winbox-<perfil>:/tmp/s.ppm /tmp/s.ppm
ffmpeg -y -i /tmp/s.ppm -update 1 /tmp/s.png
# abra /tmp/s.png — se vier "Display output is not active", é o caso (A/B).
```

---

## 3. "Erro no docker compose up" / container name in use

**Sintoma**: ao subir um perfil que já rodou antes, o app retorna um
erro mencionando algo como `container name "winbox-<x>" is already in
use`.

**Diagnóstico** — versões atuais do app fazem `docker rm -f` defensivo
antes de `compose up` quando o container está em `exited`, `created`
ou `dead`, então esse sintoma só deve aparecer se você rodou
`docker compose` manualmente sem `-p winbox-<perfil>`.

**Fix**:

```bash
docker rm -f winbox-<perfil>
```

E suba pelo app de novo. Se o problema persistir, confira que a versão
do app está atualizada (deve incluir o commit
`fix(core): unblock VM launch flows and pin Docker image tags`).

**Log de evidência**: a mensagem do toast traz o `code:
"docker_daemon_down"`, `"image_pull_failed"`, etc. — anote.

---

## 4. Windows não termina de iniciar (timeout de ~4min)

**Sintoma**: o app mostra **"Windows '<perfil>' não terminou de
iniciar a tempo. Na primeira execução o dockur baixa a ISO — abra os
logs do perfil para acompanhar."**

**Diagnóstico** — abra os logs do perfil pelo próprio app (menu
"Mais ações" → "Logs") ou via CLI:

```bash
docker logs --tail 100 winbox-<perfil>
```

Você vai ver linhas tipo:

```
❯ Downloading Windows.iso ... 32%
```

A primeira instalação de cada perfil Windows baixa uma ISO de ~5 GB.
A 240 segundos não é suficiente para isso em conexão doméstica.

**Fix**:

- Esperar terminar o download (acompanhe pelos logs). Quando aparecer
  `windows started successfully`, o app já vai detectar.
- Em conexões muito lentas, pode ser preciso reiniciar o launch após
  a ISO completar.

**Log de evidência**: `docker logs winbox-<perfil>` mostra o progresso
do dockur em tempo real.

---

## 5. Atalhos do Windows / Linux não chegam ao guest

**Sintoma**: aperto a tecla **Win** dentro da janela RDP e o menu do GNOME/KDE
do host abre (em vez do menu Iniciar do Windows). Mesma coisa para
`Alt+Tab`, `Super+L`, etc.

**Por quê**: por padrão o servidor X só envia teclas "normais" para o cliente
de janela. Teclas com modificadores reservados pelo WM ficam no host. xfreerdp
contorna isso ativando o **grab de teclado** (`+grab-keyboard`), que faz com
que TODAS as teclas vão para o guest enquanto a janela RDP estiver focada.

**Fix (Windows)**: já está ativo a partir desta versão. Como **soltar** o grab
quando quiser usar atalhos do host:

- Aperte **`Right CTRL`** (Ctrl da direita) — xfreerdp libera o teclado e o mouse.
- Ou simplesmente clique fora da janela RDP.

**Linux via noVNC (navegador)**: o browser não permite capturar a tecla Super
(restrição de sandbox). Use uma das alternativas:

- Toolbar do noVNC: clique no menu lateral → "Send Key" → "Windows".
- Cliente VNC nativo: `remmina vnc://127.0.0.1:<WEB_PORT>` (ou tigervnc-viewer)
  com a opção *grab keyboard* habilitada. A porta 5900 do container expõe VNC
  raw também — mas só dentro da rede do container. Para acesso direto, mapeie
  `5900:5900` em `EXTRA_PORTS` do perfil.

**Log de evidência**: tente `Right CTRL` e depois aperte Win. Se ainda não
funcionar, o servidor RDP do guest pode estar mapeando o keyboard layout
errado — ajuste o profile language/keyboard.

---

## Mais

Se nada acima cobre o seu caso, abra um issue com:

1. Saída de `docker ps -a --filter name=winbox`.
2. Conteúdo do toast de erro (com o `code`).
3. Trecho relevante de `docker logs winbox-<perfil>`.
4. Trecho de `~/.cache/winbox/rdp-<perfil>.log`, se aplicável.

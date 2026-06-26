#!/usr/bin/env bash
# setup-host-excel.sh
# Prepara o host (Linux Mint 22.3 / Ubuntu 24.04) para rodar o Microsoft Excel real
# via winbox-gui (VM Windows em dockur/windows) + WinApps (janela isolada do Excel).
# Faz APENAS o lado do host: deps, Docker, winbox (release) e clona o WinApps.
# Instalar Windows/Office e a config final do WinApps são passos manuais (guest).
set -euo pipefail

### Config (sobrescrevível por env) ###
WINBOX_VERSION="${WINBOX_VERSION:-v0.1.0}"
WINBOX_REPO="${WINBOX_REPO:-brunosps/winbox-gui}"
WINBOX_ASSET="winbox-x86_64-unknown-linux-gnu.tar.gz"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/share/winbox}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
WINAPPS_DIR="${WINAPPS_DIR:-$HOME/code/winapps}"

log()  { printf '\033[1;34m[setup]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[aviso]\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31m[erro]\033[0m %s\n' "$*" >&2; exit 1; }

[ "$(id -u)" -ne 0 ] || die "Rode como usuário normal (usa sudo quando precisa), não root."
command -v sudo >/dev/null || die "sudo não encontrado."
[ "$(uname -m)" = "x86_64" ] || die "Script é para x86_64."

### 1. Preflight: virtualização ###
log "Verificando virtualização..."
grep -qE 'vmx|svm' /proc/cpuinfo || die "CPU sem VT-x/AMD-V habilitado na BIOS."
[ -e /dev/kvm ] || die "/dev/kvm ausente — habilite virtualização na BIOS."
log "VT-x + /dev/kvm OK."

### 2. Dependências de sistema ###
log "Atualizando índice APT..."
sudo apt-get update -y

# instala o 1º pacote existente dentre os candidatos (lida com nomes t64 do 24.04)
apt_install_one() {
  local p
  for p in "$@"; do
    if apt-cache show "$p" >/dev/null 2>&1; then
      sudo apt-get install -y "$p" && return 0
    fi
  done
  warn "Nenhum destes pacotes existe nesta distro: $*"; return 1
}

log "Libs de runtime do winbox (Tauri)..."
apt_install_one libwebkit2gtk-4.1-0
apt_install_one libgtk-3-0t64 libgtk-3-0
apt_install_one libayatana-appindicator3-1
apt_install_one librsvg2-2
if command -v docker >/dev/null 2>&1; then
  log "Docker já instalado ($(docker --version 2>/dev/null)); pulando."
else
  log "Instalando Docker..."
  apt_install_one docker.io docker-ce || warn "Docker não instalado via APT; instale manualmente."
fi
if docker compose version >/dev/null 2>&1 || command -v docker-compose >/dev/null 2>&1; then
  log "Docker Compose já disponível; pulando."
else
  apt_install_one docker-compose-v2 docker-compose || warn "Compose não instalado."
fi
apt_install_one curl; apt_install_one tar; apt_install_one git
# Cliente FreeRDP para o WinApps.
# IMPORTANTE: o freerdp3-x11 do Ubuntu 24.04 (3.5.1) tem um bug de renderização
# de janelas RemoteApp/RAIL no X11 (crash 'X_CopyArea BadMatch'). Por isso usamos
# o FreeRDP do Flathub (3.27+), que o WinApps suporta via FREERDP_COMMAND.
log "Cliente FreeRDP (Flatpak, p/ evitar bug RAIL do 3.5.1 do sistema)..."
apt_install_one flatpak || warn "flatpak não instalado."
if command -v flatpak >/dev/null 2>&1; then
  sudo flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo || warn "flathub não adicionado."
  sudo flatpak install -y flathub com.freerdp.FreeRDP || warn "FreeRDP (flatpak) não instalado."
  sudo flatpak override --filesystem=home com.freerdp.FreeRDP || true
fi

### 3. Serviço Docker + grupos ###
log "Habilitando Docker..."
sudo systemctl enable --now docker || warn "Falha ao habilitar docker via systemctl."
for grp in docker kvm; do
  if getent group "$grp" >/dev/null && ! id -nG "$USER" | tr ' ' '\n' | grep -qx "$grp"; then
    log "Adicionando $USER ao grupo $grp..."; sudo usermod -aG "$grp" "$USER"; NEED_RELOGIN=1
  fi
done

### 4. Baixar e instalar o winbox (release) ###
log "Baixando winbox $WINBOX_VERSION..."
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
url="https://github.com/$WINBOX_REPO/releases/download/$WINBOX_VERSION/$WINBOX_ASSET"
curl -fL --retry 3 -o "$tmp/winbox.tar.gz" "$url" || die "Falha ao baixar $url"
tar -xzf "$tmp/winbox.tar.gz" -C "$tmp"
# o tarball do release traz o executável como 'winbox-gui' (fallback: 'winbox')
bin="$(find "$tmp" -type f \( -name winbox-gui -o -name winbox \) | head -n1)"
[ -n "$bin" ] || die "Binário do winbox não encontrado no tarball."
mkdir -p "$INSTALL_DIR" "$BIN_DIR"
install -m 0755 "$bin" "$INSTALL_DIR/winbox"
ln -sf "$INSTALL_DIR/winbox" "$BIN_DIR/winbox"
log "winbox em $INSTALL_DIR/winbox (link em $BIN_DIR/winbox)."
case ":$PATH:" in
  *":$BIN_DIR:"*) : ;;
  *) warn "$BIN_DIR fora do PATH. Rode: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc" ;;
esac

### 5. Atalho no menu ###
desktop_dir="$HOME/.local/share/applications"; mkdir -p "$desktop_dir"
cat > "$desktop_dir/winbox.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=winbox
Comment=Gerenciador de VMs Windows (dockur/windows)
Exec=$INSTALL_DIR/winbox
Terminal=false
Categories=System;Utility;
Keywords=windows;vm;rdp;dockur;winbox;excel;
EOF
log "Atalho em $desktop_dir/winbox.desktop."

### 6. Clonar WinApps (instalador roda depois, com a VM no ar) ###
if [ ! -d "$WINAPPS_DIR/.git" ]; then
  log "Clonando WinApps em $WINAPPS_DIR..."
  git clone --depth 1 https://github.com/winapps-org/winapps "$WINAPPS_DIR" || warn "Falha ao clonar WinApps."
else
  log "WinApps já presente em $WINAPPS_DIR."
fi

### Próximos passos (manuais) ###
cat <<NEXT

==================================================================
 HOST PRONTO. Próximos passos manuais:
 1) Abra o winbox (comando 'winbox' ou pelo menu).
    - Crie um perfil Windows (instala Windows 11 Pro no dockur).
    - Defina USUÁRIO e SENHA RDP do perfil (anote).
    - Pelo noVNC, instale o Microsoft 365/Office (Excel) no guest.
 2) No Windows: importe o RDPApps.reg do WinApps e confirme RDP ligado.
 3) ~/.config/winapps/winapps.conf (modo manual, FreeRDP via Flatpak):
       RDP_USER="<usuario>"  RDP_PASS="<senha>"
       RDP_IP="127.0.0.1"    RDP_PORT="<veja config.env do perfil>"
       WAFLAVOR="manual"
       FREERDP_COMMAND="flatpak run --command=xfreerdp com.freerdp.FreeRDP"
    Rode:  cd ${WINAPPS_DIR} && ./setup.sh --user --setupAllOfficiallySupportedApps
 4) Excel aparece no menu do XFCE como janela isolada (winapps excel-o365-x86).
==================================================================
NEXT
[ "${NEED_RELOGIN:-0}" = "1" ] && warn "Adicionado a grupos novos (docker/kvm). FAÇA LOGOUT/LOGIN antes de usar o winbox."
log "Concluído."

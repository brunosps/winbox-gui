use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::paths;

const UBUNTU_CLOUD_IMAGE_URL: &str =
    "https://cloud-images.ubuntu.com/releases/noble/release/ubuntu-24.04-server-cloudimg-amd64.img";
const UBUNTU_SHA256SUMS_URL: &str =
    "https://cloud-images.ubuntu.com/releases/noble/release/SHA256SUMS";
const UBUNTU_IMAGE_NAME: &str = "ubuntu-24.04-server-cloudimg-amd64.img";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudInitProfile {
    Server,
    XubuntuDesktop,
}

impl CloudInitProfile {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
            "" | "server" | "ubuntu-server" | "ubuntu_deploy_vm" | "deploy" => Ok(Self::Server),
            "desktop"
            | "xubuntu"
            | "xubuntu-desktop"
            | "xubuntu_desktop"
            | "xubuntu-desktop-minimal" => Ok(Self::XubuntuDesktop),
            other => bail!(
                "unsupported_cloud_init_profile: '{}' is not supported. Use server or xubuntu-desktop.",
                other
            ),
        }
    }

    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::XubuntuDesktop => "xubuntu-desktop",
        }
    }

    pub(crate) fn desktop_enabled(self) -> bool {
        matches!(self, Self::XubuntuDesktop)
    }
}

pub fn prepare_profile(
    profile: &str,
    disk_size: &str,
    requested_user: &str,
    requested_password: &str,
    cloud_init_profile: CloudInitProfile,
) -> Result<()> {
    let user = normalize_user(requested_user);
    let public_key = local_ssh_public_key().ok_or_else(|| {
        anyhow::anyhow!(
            "ssh_public_key_missing: no ~/.ssh/id_ed25519.pub or ~/.ssh/id_rsa.pub found"
        )
    })?;
    let storage_dir = paths::profile_storage_dir(profile);
    std::fs::create_dir_all(&storage_dir)
        .with_context(|| format!("mkdir {}", storage_dir.display()))?;

    let image_cache = paths::cache_dir().join("images");
    std::fs::create_dir_all(&image_cache)
        .with_context(|| format!("mkdir {}", image_cache.display()))?;
    let base_image = image_cache.join("ubuntu-24.04-server-cloudimg-amd64.qcow2");
    ensure_base_image(&base_image)?;

    let boot_image = storage_dir.join("boot.qcow2");
    std::fs::copy(&base_image, &boot_image).with_context(|| {
        format!(
            "copy cloud image {} -> {}",
            base_image.display(),
            boot_image.display()
        )
    })?;
    resize_qcow2(&boot_image, disk_size)?;
    write_seed(
        profile,
        &user,
        &public_key,
        requested_password,
        &storage_dir,
        cloud_init_profile,
    )?;
    Ok(())
}

fn ensure_base_image(base_image: &Path) -> Result<()> {
    if base_image.is_file() {
        return verify_image_hash(base_image);
    }
    let tmp = base_image.with_extension("qcow2.tmp");
    let _ = std::fs::remove_file(&tmp);
    run_command(
        Command::new("curl")
            .arg("-fL")
            .arg("--retry")
            .arg("3")
            .arg("-o")
            .arg(&tmp)
            .arg(UBUNTU_CLOUD_IMAGE_URL),
        "download Ubuntu cloud image",
    )?;
    verify_image_hash(&tmp)?;
    std::fs::rename(&tmp, base_image)
        .with_context(|| format!("move {} -> {}", tmp.display(), base_image.display()))?;
    Ok(())
}

fn verify_image_hash(image: &Path) -> Result<()> {
    let sums_path = paths::cache_dir()
        .join("images")
        .join("ubuntu-24.04-SHA256SUMS");
    run_command(
        Command::new("curl")
            .arg("-fL")
            .arg("--retry")
            .arg("3")
            .arg("-o")
            .arg(&sums_path)
            .arg(UBUNTU_SHA256SUMS_URL),
        "download Ubuntu SHA256SUMS",
    )?;
    let sums = std::fs::read_to_string(&sums_path)
        .with_context(|| format!("read {}", sums_path.display()))?;
    let expected = expected_hash(&sums, UBUNTU_IMAGE_NAME).ok_or_else(|| {
        anyhow::anyhow!("sha256_missing: {UBUNTU_IMAGE_NAME} not found in SHA256SUMS")
    })?;
    let output = Command::new("sha256sum")
        .arg(image)
        .output()
        .with_context(|| "run sha256sum")?;
    if !output.status.success() {
        bail!(
            "sha256sum_failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let actual = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if actual != expected {
        bail!("sha256_mismatch: expected {expected}, got {actual}");
    }
    Ok(())
}

fn resize_qcow2(image: &Path, disk_size: &str) -> Result<()> {
    run_command(
        Command::new("qemu-img")
            .arg("resize")
            .arg("-f")
            .arg("qcow2")
            .arg(image)
            .arg(disk_size),
        "resize Ubuntu cloud image",
    )
}

fn write_seed(
    profile: &str,
    user: &str,
    public_key: &str,
    password: &str,
    storage_dir: &Path,
    cloud_init_profile: CloudInitProfile,
) -> Result<()> {
    let seed_dir = storage_dir.join("cloud-init");
    std::fs::create_dir_all(&seed_dir).with_context(|| format!("mkdir {}", seed_dir.display()))?;
    std::fs::write(
        seed_dir.join("user-data"),
        user_data(profile, user, public_key, password, cloud_init_profile),
    )
    .with_context(|| "write cloud-init user-data")?;
    std::fs::write(seed_dir.join("meta-data"), meta_data(profile))
        .with_context(|| "write cloud-init meta-data")?;

    let seed_iso = storage_dir.join("drivers.iso");
    let _ = std::fs::remove_file(&seed_iso);
    run_command(
        Command::new("docker")
            .arg("run")
            .arg("--rm")
            .arg("--entrypoint")
            .arg("genisoimage")
            .arg("-v")
            .arg(format!("{}:/storage", storage_dir.display()))
            .arg(paths::IMAGE_QEMU)
            .arg("-output")
            .arg("/storage/drivers.iso")
            .arg("-volid")
            .arg("cidata")
            .arg("-joliet")
            .arg("-rock")
            .arg("/storage/cloud-init/user-data")
            .arg("/storage/cloud-init/meta-data"),
        "generate cloud-init NoCloud seed ISO",
    )
}

fn run_command(command: &mut Command, label: &str) -> Result<()> {
    let output = command.output().with_context(|| format!("run {label}"))?;
    if output.status.success() {
        return Ok(());
    }
    bail!(
        "{label}_failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn local_ssh_public_key() -> Option<String> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    for filename in ["id_ed25519.pub", "id_rsa.pub"] {
        let path = home.join(".ssh").join(filename);
        if let Ok(value) = std::fs::read_to_string(path) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn normalize_user(user: &str) -> String {
    let trimmed = user.trim();
    if trimmed.is_empty() || trimmed == "docker" {
        "bruno".to_string()
    } else {
        trimmed.to_string()
    }
}

fn desktop_bootstrap(profile: CloudInitProfile, user: &str, password: &str) -> String {
    if !profile.desktop_enabled() {
        return String::new();
    }
    let password_bootstrap = if password.trim().is_empty() {
        String::new()
    } else {
        format!(
            "      printf '%s:%s\\n' {user_shell} {password_shell} | chpasswd\n      passwd -u {user_shell} || true\n",
            user_shell = shell_quote(user),
            password_shell = shell_quote(password),
        )
    };
    format!(
        r#"      DEBIAN_FRONTEND=noninteractive apt-get install -y software-properties-common
      add-apt-repository -y universe || true
      apt-get update
      echo 'lightdm shared/default-x-display-manager select lightdm' | debconf-set-selections || true
      DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends lightdm xubuntu-desktop-minimal xrdp xorgxrdp dbus-x11 xfce4-terminal x11vnc novnc websockify xfce4-whiskermenu-plugin menulibre
{password_bootstrap}      printf '/usr/sbin/lightdm\n' > /etc/X11/default-display-manager
      systemctl set-default graphical.target
      systemctl reset-failed lightdm display-manager || true
      systemctl start lightdm
      systemctl enable --now xrdp
      adduser xrdp ssl-cert || true
      cat > /home/{user}/.xsession <<'EOF'
      startxfce4
      EOF
      chown {user}:{user} /home/{user}/.xsession
      install -d -m 0755 /home/{user}/.config/xfce4/xfconf/xfce-perchannel-xml
      cat > /home/{user}/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml <<'EOF'
      <?xml version="1.0" encoding="UTF-8"?>

      <channel name="xfce4-panel" version="1.0">
        <property name="configver" type="int" value="2"/>
        <property name="panels" type="array">
          <value type="int" value="1"/>
          <property name="panel-1" type="empty">
            <property name="position" type="string" value="p=6;x=0;y=0"/>
            <property name="position-locked" type="bool" value="true"/>
            <property name="length" type="uint" value="100"/>
            <property name="length-adjust" type="bool" value="true"/>
            <property name="size" type="uint" value="28"/>
            <property name="plugin-ids" type="array">
              <value type="int" value="1"/>
              <value type="int" value="3"/>
              <value type="int" value="4"/>
              <value type="int" value="5"/>
              <value type="int" value="6"/>
              <value type="int" value="10"/>
            </property>
          </property>
        </property>
        <property name="plugins" type="empty">
          <property name="plugin-1" type="string" value="whiskermenu"/>
          <property name="plugin-3" type="string" value="tasklist"/>
          <property name="plugin-4" type="string" value="separator">
            <property name="expand" type="bool" value="true"/>
            <property name="style" type="uint" value="0"/>
          </property>
          <property name="plugin-5" type="string" value="systray"/>
          <property name="plugin-6" type="string" value="notification-plugin"/>
          <property name="plugin-10" type="string" value="clock"/>
        </property>
      </channel>
      EOF
      chown -R {user}:{user} /home/{user}/.config
      usermod -c {user} {user} || true
      install -d -m 0755 /etc/lightdm/lightdm.conf.d /var/lib/AccountsService/users
      cat > /etc/lightdm/lightdm.conf.d/50-dev-workflow-login.conf <<'EOF'
      [Seat:*]
      greeter-show-manual-login=true
      greeter-hide-users=false
      allow-guest=false
      user-session=xubuntu
      EOF
      cat > /var/lib/AccountsService/users/{user} <<'EOF'
      [User]
      XSession=xubuntu
      SystemAccount=false
      EOF
      if [ {user} != 'ubuntu' ]; then
        cat > /var/lib/AccountsService/users/ubuntu <<'EOF'
      [User]
      SystemAccount=true
      EOF
        sed -i 's/^hidden-users=.*/hidden-users=nobody nobody4 noaccess ubuntu/' /etc/lightdm/users.conf || true
      fi
      systemctl restart accounts-daemon || true
      cat > /etc/systemd/system/dw-x11vnc.service <<'EOF'
      [Unit]
      Description=dev-workflow LightDM x11vnc bridge
      After=lightdm.service graphical.target
      Wants=lightdm.service

      [Service]
      Type=simple
      ExecStart=/usr/bin/x11vnc -display :0 -auth /var/run/lightdm/root/:0 -forever -shared -nopw -rfbport 5901 -listen 127.0.0.1 -noxdamage -repeat
      Restart=always
      RestartSec=2

      [Install]
      WantedBy=graphical.target
      EOF
      cat > /etc/systemd/system/dw-novnc.service <<'EOF'
      [Unit]
      Description=dev-workflow browser VNC bridge
      After=dw-x11vnc.service network.target
      Wants=dw-x11vnc.service

      [Service]
      Type=simple
      ExecStart=/usr/bin/websockify --web=/usr/share/novnc 0.0.0.0:6080 127.0.0.1:5901
      Restart=always
      RestartSec=2

      [Install]
      WantedBy=multi-user.target
      EOF
      systemctl daemon-reload
      systemctl enable --now dw-x11vnc.service dw-novnc.service
"#,
        user = shell_quote(user),
        password_bootstrap = password_bootstrap,
    )
}

fn user_data(
    profile: &str,
    user: &str,
    public_key: &str,
    password: &str,
    cloud_init_profile: CloudInitProfile,
) -> String {
    let desktop_bootstrap = desktop_bootstrap(cloud_init_profile, user, password);
    format!(
        r#"#cloud-config
hostname: {hostname}
manage_etc_hosts: true
ssh_pwauth: false
disable_root: true
package_update: true
package_upgrade: false
packages:
  - ca-certificates
  - curl
  - gnupg
  - lsb-release
  - openssh-server
  - rsync
  - git
users:
  - default
  - name: {user_yaml}
    groups: [adm, sudo]
    shell: /bin/bash
    sudo: ["ALL=(ALL) NOPASSWD:ALL"]
    ssh_authorized_keys:
      - {public_key}
write_files:
  - path: /usr/local/sbin/dev-workflow-bootstrap.sh
    permissions: "0755"
    content: |
      #!/usr/bin/env bash
      set -euxo pipefail
      install -m 0755 -d /etc/apt/keyrings
      curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
      chmod a+r /etc/apt/keyrings/docker.asc
      . /etc/os-release
      docker_codename="${{VERSION_CODENAME:-noble}}"
      echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $docker_codename stable" > /etc/apt/sources.list.d/docker.list
      apt-get update
      DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        build-essential gcc g++ make pkg-config cmake ninja-build \
        python3 python3-dev python3-venv python3-pip python-is-python3 \
        nodejs npm git git-lfs jq unzip zip wget file ripgrep fd-find shellcheck \
        xvfb fonts-liberation fonts-noto-color-emoji fonts-unifont \
        libasound2t64 libatk-bridge2.0-0 libatk1.0-0 libatspi2.0-0 libcairo2 libcups2 \
        libdbus-1-3 libdrm2 libgbm1 libglib2.0-0 libgtk-3-0 libnspr4 libnss3 \
        libpango-1.0-0 libx11-6 libx11-xcb1 libxcb1 libxcomposite1 libxdamage1 \
        libxext6 libxfixes3 libxkbcommon0 libxrandr2 libxrender1 libxshmfence1 libxss1 libxtst6
      if command -v npm >/dev/null 2>&1; then
        npm install -g corepack pnpm@10 || true
        corepack prepare pnpm@10 --activate || true
      fi
      cat > /usr/local/bin/dw-playwright-bootstrap <<'EOF'
      #!/usr/bin/env bash
      set -euo pipefail
      cd "${{1:-$PWD}}"
      if [ -f package.json ]; then
        npm exec --yes playwright install chromium
      else
        npx --yes playwright install chromium
      fi
      EOF
      chmod 0755 /usr/local/bin/dw-playwright-bootstrap
      DEBIAN_FRONTEND=noninteractive apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
      systemctl enable --now ssh
      systemctl enable --now docker
{desktop_bootstrap}      loginctl enable-linger {user_shell} || true
      usermod -aG docker {user_shell}
      mkdir -p /mnt/winbox-shared
      if ! grep -q '^shared /mnt/winbox-shared ' /etc/fstab; then
        echo 'shared /mnt/winbox-shared 9p trans=virtio,version=9p2000.L,rw,_netdev,nofail 0 0' >> /etc/fstab
      fi
      modprobe 9p || true
      modprobe 9pnet || true
      modprobe 9pnet_virtio || true
      mount /mnt/winbox-shared || true
      mkdir -p /opt/dev-workflow
      printf '{{"status":"ready","build_essential":true,"node":"%s","npm":"%s","pnpm":"%s","playwright_deps":true,"playwright_bootstrap":"/usr/local/bin/dw-playwright-bootstrap","xfce_menu":%s}}\n' "$(node --version 2>/dev/null || echo unavailable)" "$(npm --version 2>/dev/null || echo unavailable)" "$(pnpm --version 2>/dev/null || echo unavailable)" "$(command -v xfce4-popup-whiskermenu >/dev/null 2>&1 && echo true || echo false)" > /opt/dev-workflow/dev-tools.json
      printf '{{"status":"ready","profile":"%s","user":"%s","cloud_init_profile":"%s","desktop":%s,"display_manager":"%s","shared_mounted":%s}}\n' {profile_shell} {user_shell} {cloud_init_profile_shell} {desktop_enabled_shell} "$(systemctl is-active lightdm 2>/dev/null || systemctl is-active display-manager 2>/dev/null || echo none)" "$(mountpoint -q /mnt/winbox-shared && echo true || echo false)" > /opt/dev-workflow/ready.json
runcmd:
  - [bash, /usr/local/sbin/dev-workflow-bootstrap.sh]
final_message: "dev-workflow cloud-init complete"
"#,
        hostname = cloud_hostname(profile),
        user_yaml = yaml_single_quote(user),
        user_shell = shell_quote(user),
        public_key = yaml_single_quote(public_key),
        profile_shell = shell_quote(profile),
        cloud_init_profile_shell = shell_quote(cloud_init_profile.as_env_value()),
        desktop_enabled_shell = if cloud_init_profile.desktop_enabled() {
            "true"
        } else {
            "false"
        },
        desktop_bootstrap = desktop_bootstrap,
    )
}

fn meta_data(profile: &str) -> String {
    format!(
        "instance-id: winbox-{profile}\nlocal-hostname: {}\n",
        cloud_hostname(profile)
    )
}

fn cloud_hostname(profile: &str) -> String {
    let mut out = profile
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        "winbox-cloud".to_string()
    } else {
        out.to_string()
    }
}

fn yaml_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn expected_hash(sums: &str, filename: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == filename).then(|| hash.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_line_is_parsed_for_exact_image() {
        let sums = "abc123  ubuntu-24.04-server-cloudimg-amd64.img\nzzz  other.img\n";
        assert_eq!(
            expected_hash(sums, "ubuntu-24.04-server-cloudimg-amd64.img").as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn cloud_init_seed_contains_ssh_docker_and_shared_mount() {
        let data = user_data(
            "dw-3-deploy",
            "bruno",
            "ssh-ed25519 AAA test",
            "",
            CloudInitProfile::Server,
        );
        assert!(data.contains("openssh-server"));
        assert!(data.contains("docker-ce"));
        assert!(data.contains("build-essential"));
        assert!(data.contains("npm install -g corepack pnpm@10"));
        assert!(data.contains("dw-playwright-bootstrap"));
        assert!(data.contains("playwright install chromium"));
        assert!(data.contains("playwright_deps"));
        assert!(data.contains("shared /mnt/winbox-shared 9p"));
        assert!(data.contains("'ssh-ed25519 AAA test'"));
        assert!(data.contains("cloud_init_profile"));
        assert!(data.contains("'server'"));
        assert!(!data.contains("xubuntu-desktop-minimal"));
    }

    #[test]
    fn desktop_cloud_init_seed_installs_xubuntu_and_xrdp() {
        let data = user_data(
            "dw-3-desktop",
            "bruno",
            "ssh-ed25519 AAA test",
            "ChangeMe123!",
            CloudInitProfile::XubuntuDesktop,
        );
        assert!(data.contains("xubuntu-desktop-minimal"));
        assert!(data.contains("xrdp"));
        assert!(data.contains("x11vnc"));
        assert!(data.contains("xfce4-whiskermenu-plugin"));
        assert!(data.contains("menulibre"));
        assert!(data.contains("xfce4-panel.xml"));
        assert!(data.contains("value=\"whiskermenu\""));
        assert!(data.contains("websockify --web=/usr/share/novnc 0.0.0.0:6080"));
        assert!(data.contains("dw-novnc.service"));
        assert!(data.contains("50-dev-workflow-login.conf"));
        assert!(data.contains("SystemAccount=true"));
        assert!(data.contains("hidden-users=nobody nobody4 noaccess ubuntu"));
        assert!(data.contains("lightdm"));
        assert!(data.contains("chpasswd"));
        assert!(data.contains("'ChangeMe123!'"));
        assert!(data.contains("systemctl set-default graphical.target"));
        assert!(data.contains("systemctl start lightdm"));
        assert!(data.contains("startxfce4"));
    }

    #[test]
    fn cloud_init_profile_parser_accepts_desktop_aliases() {
        assert_eq!(
            CloudInitProfile::parse(Some("xubuntu-desktop"))
                .expect("desktop profile")
                .as_env_value(),
            "xubuntu-desktop"
        );
        assert_eq!(
            CloudInitProfile::parse(None)
                .expect("default profile")
                .as_env_value(),
            "server"
        );
    }

    #[test]
    fn default_cloud_user_is_bruno() {
        assert_eq!(normalize_user(""), "bruno");
        assert_eq!(normalize_user("docker"), "bruno");
        assert_eq!(normalize_user("alice"), "alice");
    }
}

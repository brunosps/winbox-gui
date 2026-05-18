use anyhow::{bail, Result};
use std::net::{SocketAddr, TcpStream};
use std::thread::sleep;
use std::time::Duration;

use crate::core::image_family::ImageFamily;
use crate::core::{docker, env_file, gpu_hooks, paths, rdp};

#[derive(Clone, Copy, Debug)]
pub enum OnClose {
    Keep,
    Pause,
    Shutdown,
    Kill,
}

impl std::str::FromStr for OnClose {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "keep" => OnClose::Keep,
            "pause" => OnClose::Pause,
            "shutdown" => OnClose::Shutdown,
            "kill" => OnClose::Kill,
            _ => bail!("--on-close inválido: {s} (use keep|pause|shutdown|kill)"),
        })
    }
}

/// Ensure the VM container is running and wait for it to be reachable.
/// For Windows guests we wait on the dockur log line; for Linux guests
/// (qemux/qemu) we wait until the web-VNC port responds, since there's
/// no equivalent log marker.
pub fn ensure_running(profile: &str) -> Result<()> {
    let container = paths::profile_container(profile);
    let status = docker::container_status(&container);
    match status.as_str() {
        "paused" => {
            docker::unpause(&container)?;
            Ok(())
        }
        "running" => Ok(()),
        "absent" => {
            gpu_hooks::prepare_for_start(profile)?;
            docker::compose_run(profile, &["up", "-d"])?;
            wait_for_ready(profile, &container)
        }
        _ => {
            // exited/created/dead/restarting: the compose template hardcodes
            // container_name, so any stale container collides with `compose up`.
            // Force-remove before recreating.
            docker::rm_force(&container)?;
            gpu_hooks::prepare_for_start(profile)?;
            docker::compose_run(profile, &["up", "-d"])?;
            wait_for_ready(profile, &container)
        }
    }
}

fn wait_for_ready(profile: &str, container: &str) -> Result<()> {
    let env_path = paths::profile_env_file(profile);
    let map = env_file::read(&env_path)?;
    let family = ImageFamily::from_env_map(&map);
    match family {
        ImageFamily::Windows => wait_for_windows(profile, container),
        ImageFamily::LinuxDistro | ImageFamily::LinuxIso => {
            let port = env_file::get_u16(&map, "WEB_PORT");
            wait_for_web_port(profile, port)
        }
    }
}

fn wait_for_windows(profile: &str, container: &str) -> Result<()> {
    for _ in 0..120 {
        if docker::logs_contains(container, "windows started successfully") {
            return Ok(());
        }
        sleep(Duration::from_secs(2));
    }
    bail!("Timeout. Verifique: winbox logs {profile}")
}

fn wait_for_web_port(profile: &str, port: u16) -> Result<()> {
    if port == 0 {
        bail!("WEB_PORT não definido para '{profile}'");
    }
    let addr: SocketAddr = format!("{}:{}", paths::HOST, port)
        .parse()
        .map_err(|e| anyhow::anyhow!("endereço inválido {}:{} — {e}", paths::HOST, port))?;
    for _ in 0..60 {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok() {
            return Ok(());
        }
        sleep(Duration::from_secs(1));
    }
    bail!("web-VNC ainda não respondeu em {addr} após 60s — confira: winbox logs {profile}")
}

/// Only starts the container (no RDP), returning immediately after start.
pub fn start(profile: &str) -> Result<()> {
    let container = paths::profile_container(profile);
    let status = docker::container_status(&container);
    match status.as_str() {
        "running" => Ok(()),
        "paused" => docker::unpause(&container),
        "absent" => {
            gpu_hooks::prepare_for_start(profile)?;
            docker::compose_run(profile, &["up", "-d"])
        }
        _ => {
            docker::rm_force(&container)?;
            gpu_hooks::prepare_for_start(profile)?;
            docker::compose_run(profile, &["up", "-d"])
        }
    }
}

/// Windows path: ensure running + spawn xfreerdp. Caller must already know
/// the profile is RDP-capable (see core::connect::resolve_for_profile).
pub fn launch_rdp(profile: &str) -> Result<()> {
    docker::check_kvm()?;
    docker::require_installed()?;
    docker::check_daemon()?;
    ensure_running(profile)?;
    rdp::launch(profile)
}

/// Linux path: ensure running + return WEB_PORT so the caller (Tauri command
/// in the GUI, or `xdg-open` in the CLI) can open the noVNC client.
pub fn ensure_for_web_vnc(profile: &str) -> Result<u16> {
    docker::check_kvm()?;
    docker::require_installed()?;
    docker::check_daemon()?;
    ensure_running(profile)?;
    let map = env_file::read(&paths::profile_env_file(profile))?;
    Ok(env_file::get_u16(&map, "WEB_PORT"))
}

/// Backward-compat shim used by the legacy CLI `launch` subcommand.
/// Routes to RDP for Windows and to web-VNC (via xdg-open) for Linux.
pub fn launch(profile: &str, on_close: OnClose) -> Result<()> {
    let mode = crate::core::connect::resolve_for_profile(profile);
    match mode {
        crate::core::connect::ConnectMode::Rdp => launch_rdp(profile)?,
        crate::core::connect::ConnectMode::WebVnc => {
            let port = ensure_for_web_vnc(profile)?;
            let _ = std::process::Command::new("xdg-open")
                .arg(format!("http://{}:{}", paths::HOST, port))
                .spawn();
        }
    }
    let _ = on_close;
    Ok(())
}

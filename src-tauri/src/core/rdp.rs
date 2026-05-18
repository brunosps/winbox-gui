use anyhow::{Context, Result};
use std::fs;
use std::process::{Command, Stdio};

use super::env_file;
use super::paths;

/// Returns a scale factor flag ("/scale:<N>") if the X display is a HiDPI screen.
/// Best-effort: returns empty if xrandr unavailable.
fn detect_scale_flag() -> String {
    let out = Command::new("xrandr").arg("--current").output();
    let Ok(out) = out else {
        return String::new();
    };
    let s = String::from_utf8_lossy(&out.stdout);
    // Look for primary line "primary <WxH>+…"
    for line in s.lines() {
        if line.contains(" connected primary") {
            // Pick token with "x" and "+", extract width
            for tok in line.split_whitespace() {
                if tok.contains('x') && tok.contains('+') {
                    if let Some((res, _)) = tok.split_once('+') {
                        if let Some((w, _h)) = res.split_once('x') {
                            if let Ok(width) = w.parse::<u32>() {
                                if width >= 3400 {
                                    return "/scale:180".into();
                                } else if width >= 2500 {
                                    return "/scale:140".into();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    String::new()
}

pub fn launch(profile: &str) -> Result<()> {
    let env_path = paths::profile_env_file(profile);
    let map = env_file::read(&env_path)?;
    let user = env_file::get(&map, "USERNAME");
    let pass = env_file::get(&map, "PASSWORD");
    let rdp = env_file::get_u16(&map, "RDP_PORT");

    let scale = detect_scale_flag();

    let mut cmd = Command::new("systemd-inhibit");
    cmd.args([
        "--what=sleep:idle",
        "--who=winbox",
        &format!("--why=VM '{}' ativa", profile),
        "xfreerdp3",
    ])
    .arg(format!("/u:{}", user))
    .arg(format!("/p:{}", pass))
    .arg(format!("/v:{}:{}", paths::HOST, rdp))
    .args([
        "/cert:ignore",
        "/dynamic-resolution",
        "/sound",
        "/microphone",
        "/clipboard",
        "/gfx:AVC444",
        "-grab-keyboard",
        "/floatbar:sticky:off,default:visible,show:fullscreen",
    ])
    .arg(format!("/title:Windows ({})", profile));
    if !scale.is_empty() {
        cmd.arg(scale);
    }
    let log_path = paths::profile_rdp_log_file(profile);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("abrindo log RDP {}", log_path.display()))?;
    let stderr_dup = log_file
        .try_clone()
        .with_context(|| format!("clonando handle de log {}", log_path.display()))?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(stderr_dup))
        .spawn()?;
    Ok(())
}

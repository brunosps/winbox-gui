use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::paths;

/// Docker container lifecycle status.
pub fn container_status(name: &str) -> String {
    let out = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name=^{}$", name),
            "--format",
            "{{.State}}",
        ])
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                "absent".into()
            } else {
                s
            }
        }
        Err(_) => "absent".into(),
    }
}

/// Return ("docker", ["compose"]) or ("docker-compose", []) — whichever is on PATH.
fn compose_bin() -> (&'static str, &'static [&'static str]) {
    static CACHE: OnceLock<(&'static str, &'static [&'static str])> = OnceLock::new();
    *CACHE.get_or_init(|| {
        // Prefer `docker compose` (v2 plugin).
        let ok = Command::new("docker")
            .args(["compose", "version"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            ("docker", &["compose"])
        } else {
            ("docker-compose", &[])
        }
    })
}

/// Run `docker compose` in a profile's config dir with its env file.
/// `action_args` examples: ["up", "-d"], ["down"], ["pull"].
pub fn compose_run(profile: &str, action_args: &[&str]) -> Result<()> {
    let cfg_dir = paths::profile_cfg_dir(profile);
    let env_file = paths::profile_env_file(profile);
    let compose_file = paths::profile_compose_file(profile);
    let project = format!("winbox-{}", profile);

    let (bin, prefix) = compose_bin();
    let mut cmd = Command::new(bin);
    cmd.args(prefix)
        .arg("-p")
        .arg(&project)
        .arg("--env-file")
        .arg(&env_file)
        .arg("-f")
        .arg(&compose_file)
        .current_dir(&cfg_dir)
        .args(action_args);

    let status = cmd.status().with_context(|| format!("{} compose", bin))?;
    if !status.success() {
        bail!("docker compose {:?} failed", action_args);
    }
    Ok(())
}

/// docker logs <container> [args…] — returns stdout+stderr joined.
pub fn logs(container: &str, extra: &[&str]) -> Result<String> {
    let mut cmd = Command::new("docker");
    cmd.arg("logs").args(extra).arg(container);
    let out = cmd.output()?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(s)
}

pub fn pause(container: &str) -> Result<()> {
    let ok = Command::new("docker")
        .args(["pause", container])
        .status()?
        .success();
    if !ok {
        bail!("docker pause failed");
    }
    Ok(())
}

pub fn unpause(container: &str) -> Result<()> {
    let ok = Command::new("docker")
        .args(["unpause", container])
        .status()?
        .success();
    if !ok {
        bail!("docker unpause failed");
    }
    Ok(())
}

pub fn stop(container: &str, timeout: u32) -> Result<()> {
    let t = timeout.to_string();
    let ok = Command::new("docker")
        .args(["stop", "-t", &t, container])
        .status()?
        .success();
    if !ok {
        bail!("docker stop failed");
    }
    Ok(())
}

pub fn kill(container: &str) -> Result<()> {
    let _ = Command::new("docker").args(["kill", container]).status();
    Ok(())
}

pub fn rm_force(container: &str) -> Result<()> {
    let _ = Command::new("docker")
        .args(["rm", "-f", container])
        .status();
    Ok(())
}

pub fn pull(image: &str) -> Result<()> {
    let ok = Command::new("docker")
        .args(["pull", image])
        .status()?
        .success();
    if !ok {
        bail!("docker pull failed");
    }
    Ok(())
}

/// `docker logs <c>` + grep for substring — used to wait for readiness.
pub fn logs_contains(container: &str, needle: &str) -> bool {
    match logs(container, &[]) {
        Ok(s) => s.to_lowercase().contains(&needle.to_lowercase()),
        Err(_) => false,
    }
}

pub fn require_installed() -> Result<()> {
    which::which("docker")
        .map(|_| ())
        .map_err(|_| anyhow!("docker não encontrado no PATH"))
}

pub fn check_daemon() -> Result<()> {
    let ok = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success();
    if !ok {
        bail!("docker daemon inacessível (rode: sudo systemctl start docker)");
    }
    Ok(())
}

pub fn check_kvm() -> Result<()> {
    if !Path::new("/dev/kvm").exists() {
        bail!("/dev/kvm ausente — KVM não disponível neste host");
    }
    Ok(())
}

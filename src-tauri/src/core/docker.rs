use anyhow::{anyhow, bail, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::launch_error::LaunchError;
use super::paths;

/// Abstraction over the Docker CLI. Real callers use [`CliDocker`]; tests
/// inject [`mock::MockDocker`] (or any custom impl) to drive the launch
/// path without a real daemon.
///
/// Operations that surface launch-path failures (`compose_run`, `pull`)
/// return [`LaunchError`] so the frontend can render structured hints.
/// Lower-level operations stay on `anyhow::Result` to keep the trait
/// from leaking launch-specific semantics into housekeeping calls.
pub trait DockerClient {
    fn container_status(&self, name: &str) -> String;
    fn compose_run(
        &self,
        profile: &str,
        action_args: &[&str],
    ) -> std::result::Result<(), LaunchError>;
    fn logs(&self, container: &str, extra: &[&str]) -> Result<String>;
    fn logs_contains(&self, container: &str, needle: &str) -> bool;
    fn pause(&self, container: &str) -> Result<()>;
    fn unpause(&self, container: &str) -> Result<()>;
    fn stop(&self, container: &str, timeout: u32) -> Result<()>;
    fn kill(&self, container: &str) -> Result<()>;
    fn rm_force(&self, container: &str) -> Result<()>;
    fn pull(&self, image: &str) -> std::result::Result<(), LaunchError>;
}

/// Default implementation that shells out to `docker` (v2 `compose` plugin
/// if present, falling back to `docker-compose`).
#[derive(Default, Clone, Copy)]
pub struct CliDocker;

impl DockerClient for CliDocker {
    fn container_status(&self, name: &str) -> String {
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

    fn compose_run(
        &self,
        profile: &str,
        action_args: &[&str],
    ) -> std::result::Result<(), LaunchError> {
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

        let out = cmd.output().map_err(|e| LaunchError::Other {
            message: format!("falha ao executar {bin} compose: {e}"),
        })?;
        if out.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        Err(classify_compose_stderr(&stderr))
    }

    fn logs(&self, container: &str, extra: &[&str]) -> Result<String> {
        let mut cmd = Command::new("docker");
        cmd.arg("logs").args(extra).arg(container);
        let out = cmd.output()?;
        let mut s = String::from_utf8_lossy(&out.stdout).to_string();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        Ok(s)
    }

    fn logs_contains(&self, container: &str, needle: &str) -> bool {
        match self.logs(container, &[]) {
            Ok(s) => s.to_lowercase().contains(&needle.to_lowercase()),
            Err(_) => false,
        }
    }

    fn pause(&self, container: &str) -> Result<()> {
        let ok = Command::new("docker")
            .args(["pause", container])
            .status()?
            .success();
        if !ok {
            bail!("docker pause failed");
        }
        Ok(())
    }

    fn unpause(&self, container: &str) -> Result<()> {
        let ok = Command::new("docker")
            .args(["unpause", container])
            .status()?
            .success();
        if !ok {
            bail!("docker unpause failed");
        }
        Ok(())
    }

    fn stop(&self, container: &str, timeout: u32) -> Result<()> {
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

    fn kill(&self, container: &str) -> Result<()> {
        let _ = Command::new("docker").args(["kill", container]).status();
        Ok(())
    }

    fn rm_force(&self, container: &str) -> Result<()> {
        let _ = Command::new("docker")
            .args(["rm", "-f", container])
            .status();
        Ok(())
    }

    fn pull(&self, image: &str) -> std::result::Result<(), LaunchError> {
        let out = Command::new("docker")
            .args(["pull", image])
            .output()
            .map_err(|e| LaunchError::Other {
                message: format!("falha ao executar docker pull: {e}"),
            })?;
        if out.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        Err(LaunchError::ImagePullFailed {
            image: image.into(),
            stderr: stderr.lines().take(10).collect::<Vec<_>>().join("\n"),
        })
    }
}

/// Classify `docker compose` stderr into a [`LaunchError`]. Pure function
/// so it can be unit-tested without spawning a process.
pub fn classify_compose_stderr(stderr: &str) -> LaunchError {
    let lower = stderr.to_lowercase();

    if let Some(port) = extract_port_conflict(stderr) {
        return LaunchError::PortConflict { port };
    }
    if lower.contains("permission denied")
        && (lower.contains("/var/run/docker.sock") || lower.contains("dial unix"))
    {
        return LaunchError::DockerDaemonDown;
    }
    if lower.contains("cannot connect to the docker daemon")
        || lower.contains("is the docker daemon running")
    {
        return LaunchError::DockerDaemonDown;
    }
    if lower.contains("pull access denied")
        || lower.contains("not found: manifest")
        || lower.contains("error response from daemon: manifest")
        || lower.contains("toomanyrequests")
    {
        // Best-effort image extraction from compose output like
        // `Error response from daemon: pull access denied for foo/bar`
        let image = extract_image_name(stderr).unwrap_or_else(|| "<unknown>".into());
        return LaunchError::ImagePullFailed {
            image,
            stderr: stderr.lines().take(10).collect::<Vec<_>>().join("\n"),
        };
    }
    LaunchError::Other {
        message: stderr
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string(),
    }
}

fn extract_port_conflict(stderr: &str) -> Option<u16> {
    // Match `bind: address already in use` / `port is already allocated`
    // patterns. Both Docker (v2) and containerd phrasings carry a host:port
    // token immediately before the message; we extract that port.
    let lower = stderr.to_lowercase();
    if !lower.contains("address already in use") && !lower.contains("port is already allocated") {
        return None;
    }
    // Walk the string and pick the FIRST `:NNN` run where NNN parses as u16.
    let bytes = stderr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                if let Ok(p) = stderr[start..end].parse::<u16>() {
                    if p > 0 {
                        return Some(p);
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    None
}

fn extract_image_name(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        if let Some(rest) = line.split_once("pull access denied for ") {
            let candidate = rest.1.split_whitespace().next().unwrap_or("");
            if !candidate.is_empty() {
                return Some(candidate.trim_end_matches(',').to_string());
            }
        }
        if let Some(rest) = line.split_once("manifest for ") {
            let candidate = rest.1.split_whitespace().next().unwrap_or("");
            if !candidate.is_empty() {
                return Some(candidate.trim_end_matches(':').to_string());
            }
        }
    }
    None
}

/// Return ("docker", ["compose"]) or ("docker-compose", []) — whichever is on PATH.
fn compose_bin() -> (&'static str, &'static [&'static str]) {
    static CACHE: OnceLock<(&'static str, &'static [&'static str])> = OnceLock::new();
    *CACHE.get_or_init(|| {
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

// ──────────────────────────────────────────────────────────────────────────
// Module-level wrappers — preserve the existing free-function API so callers
// can migrate to the trait incrementally. Each delegates to CliDocker.
// ──────────────────────────────────────────────────────────────────────────

pub fn container_status(name: &str) -> String {
    CliDocker.container_status(name)
}
pub fn compose_run(profile: &str, action_args: &[&str]) -> Result<()> {
    CliDocker
        .compose_run(profile, action_args)
        .map_err(anyhow::Error::new)
}
pub fn logs(container: &str, extra: &[&str]) -> Result<String> {
    CliDocker.logs(container, extra)
}
pub fn logs_contains(container: &str, needle: &str) -> bool {
    CliDocker.logs_contains(container, needle)
}
pub fn pause(container: &str) -> Result<()> {
    CliDocker.pause(container)
}
pub fn unpause(container: &str) -> Result<()> {
    CliDocker.unpause(container)
}
pub fn stop(container: &str, timeout: u32) -> Result<()> {
    CliDocker.stop(container, timeout)
}
pub fn kill(container: &str) -> Result<()> {
    CliDocker.kill(container)
}
pub fn rm_force(container: &str) -> Result<()> {
    CliDocker.rm_force(container)
}
pub fn pull(image: &str) -> Result<()> {
    CliDocker.pull(image).map_err(anyhow::Error::new)
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

// ──────────────────────────────────────────────────────────────────────────
// Preflight checks that return LaunchError directly — used by the launch
// path so the frontend gets a structured error instead of a string.
// ──────────────────────────────────────────────────────────────────────────

pub fn preflight_kvm() -> std::result::Result<(), LaunchError> {
    if Path::new("/dev/kvm").exists() {
        Ok(())
    } else {
        Err(LaunchError::KvmDenied)
    }
}

pub fn preflight_docker_installed() -> std::result::Result<(), LaunchError> {
    which::which("docker")
        .map(|_| ())
        .map_err(|_| LaunchError::DockerMissing)
}

pub fn preflight_docker_daemon() -> std::result::Result<(), LaunchError> {
    let ok = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        Ok(())
    } else {
        Err(LaunchError::DockerDaemonDown)
    }
}

pub fn preflight_freerdp() -> std::result::Result<(), LaunchError> {
    which::which("xfreerdp3")
        .map(|_| ())
        .map_err(|_| LaunchError::FreeRdpMissing)
}

#[cfg(test)]
pub mod mock {
    //! Test double: records every call and returns scripted values.

    use super::{DockerClient, LaunchError};
    use anyhow::Result;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct MockDocker {
        pub statuses: RefCell<HashMap<String, String>>,
        pub logs_seed: RefCell<HashMap<(String, String), bool>>,
        pub calls: RefCell<Vec<String>>,
        pub fail_compose: RefCell<Option<LaunchError>>,
        pub fail_pull: RefCell<Option<LaunchError>>,
    }

    impl MockDocker {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn seed_status(&self, container: &str, state: &str) {
            self.statuses
                .borrow_mut()
                .insert(container.to_string(), state.to_string());
        }
        pub fn seed_logs_contains(&self, container: &str, needle: &str, value: bool) {
            self.logs_seed
                .borrow_mut()
                .insert((container.to_string(), needle.to_string()), value);
        }
        pub fn set_compose_failure(&self, err: LaunchError) {
            *self.fail_compose.borrow_mut() = Some(err);
        }
        pub fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
        fn record(&self, op: &str, target: &str) {
            self.calls.borrow_mut().push(format!("{}:{}", op, target));
        }
    }

    impl DockerClient for MockDocker {
        fn container_status(&self, name: &str) -> String {
            self.record("status", name);
            self.statuses
                .borrow()
                .get(name)
                .cloned()
                .unwrap_or_else(|| "absent".into())
        }
        fn compose_run(
            &self,
            profile: &str,
            action_args: &[&str],
        ) -> std::result::Result<(), LaunchError> {
            self.record("compose", &format!("{}:{}", profile, action_args.join(" ")));
            if let Some(err) = self.fail_compose.borrow().clone() {
                return Err(err);
            }
            if action_args.first() == Some(&"up") {
                self.statuses
                    .borrow_mut()
                    .insert(format!("winbox-{profile}"), "running".into());
            }
            Ok(())
        }
        fn logs(&self, container: &str, _extra: &[&str]) -> Result<String> {
            self.record("logs", container);
            Ok(String::new())
        }
        fn logs_contains(&self, container: &str, needle: &str) -> bool {
            self.record("logs_contains", &format!("{}:{}", container, needle));
            self.logs_seed
                .borrow()
                .get(&(container.to_string(), needle.to_string()))
                .copied()
                .unwrap_or(false)
        }
        fn pause(&self, container: &str) -> Result<()> {
            self.record("pause", container);
            Ok(())
        }
        fn unpause(&self, container: &str) -> Result<()> {
            self.record("unpause", container);
            self.statuses
                .borrow_mut()
                .insert(container.to_string(), "running".into());
            Ok(())
        }
        fn stop(&self, container: &str, _timeout: u32) -> Result<()> {
            self.record("stop", container);
            self.statuses
                .borrow_mut()
                .insert(container.to_string(), "exited".into());
            Ok(())
        }
        fn kill(&self, container: &str) -> Result<()> {
            self.record("kill", container);
            Ok(())
        }
        fn rm_force(&self, container: &str) -> Result<()> {
            self.record("rm_force", container);
            self.statuses.borrow_mut().remove(container);
            Ok(())
        }
        fn pull(&self, image: &str) -> std::result::Result<(), LaunchError> {
            self.record("pull", image);
            if let Some(err) = self.fail_pull.borrow().clone() {
                return Err(err);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockDocker;
    use super::*;

    #[test]
    fn mock_records_calls_in_order() {
        let docker = MockDocker::new();
        docker.seed_status("winbox-foo", "exited");
        assert_eq!(docker.container_status("winbox-foo"), "exited");
        docker.rm_force("winbox-foo").unwrap();
        docker.compose_run("foo", &["up", "-d"]).unwrap();
        let calls = docker.calls();
        assert_eq!(
            calls,
            vec![
                "status:winbox-foo".to_string(),
                "rm_force:winbox-foo".to_string(),
                "compose:foo:up -d".to_string(),
            ]
        );
        assert_eq!(docker.container_status("winbox-foo"), "running");
    }

    #[test]
    fn mock_status_defaults_to_absent() {
        let docker = MockDocker::new();
        assert_eq!(docker.container_status("nope"), "absent");
    }

    #[test]
    fn mock_compose_failure_propagates_launch_error() {
        let docker = MockDocker::new();
        docker.set_compose_failure(LaunchError::PortConflict { port: 3389 });
        let err = docker.compose_run("foo", &["up", "-d"]).unwrap_err();
        assert_eq!(err.code(), "port_conflict");
    }

    #[test]
    fn classify_recognizes_port_conflict() {
        let stderr = "Error response from daemon: driver failed programming external connectivity \
                      on endpoint winbox-foo: Bind for 127.0.0.1:3389 failed: port is already allocated";
        match classify_compose_stderr(stderr) {
            LaunchError::PortConflict { port } => assert_eq!(port, 3389),
            other => panic!("expected PortConflict, got {other:?}"),
        }
    }

    #[test]
    fn classify_recognizes_bind_address_already_in_use() {
        let stderr = "listen tcp 127.0.0.1:8006: bind: address already in use";
        match classify_compose_stderr(stderr) {
            LaunchError::PortConflict { port } => assert_eq!(port, 8006),
            other => panic!("expected PortConflict, got {other:?}"),
        }
    }

    #[test]
    fn classify_recognizes_daemon_down() {
        let stderr = "Cannot connect to the Docker daemon at unix:///var/run/docker.sock. \
                      Is the docker daemon running?";
        assert!(matches!(
            classify_compose_stderr(stderr),
            LaunchError::DockerDaemonDown
        ));
    }

    #[test]
    fn classify_recognizes_image_pull_failed() {
        let stderr =
            "Error response from daemon: pull access denied for dockurr/windows, repository does not exist";
        match classify_compose_stderr(stderr) {
            LaunchError::ImagePullFailed { image, .. } => assert_eq!(image, "dockurr/windows"),
            other => panic!("expected ImagePullFailed, got {other:?}"),
        }
    }

    #[test]
    fn classify_unknown_returns_other() {
        let stderr = "something totally unexpected blew up";
        assert!(matches!(
            classify_compose_stderr(stderr),
            LaunchError::Other { .. }
        ));
    }
}

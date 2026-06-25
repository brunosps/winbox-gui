use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::launch_error::LaunchError;
use super::paths;

/// One progress event emitted while a `docker pull` is in flight.
/// Used by [`DockerClient::pull_streaming`] to drive UI progress bars
/// without waiting for the whole pull to finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PullEvent {
    /// Short layer id (`abc123def456` form) emitted by Docker.
    pub layer: String,
    /// Human-readable status text from Docker: `Pulling fs layer`,
    /// `Downloading`, `Extracting`, `Pull complete`, etc.
    pub status: String,
    /// Bytes transferred so far for this layer, if Docker reported
    /// a progress bar on this line.
    pub current_bytes: Option<u64>,
    /// Total bytes for this layer, if known.
    pub total_bytes: Option<u64>,
}

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
    /// Pull an image, invoking `on_event` for each progress line Docker
    /// emits. Default impl falls back to non-streaming `pull` and emits
    /// no events (preserves source compat for existing impls/mocks).
    fn pull_streaming(
        &self,
        image: &str,
        _on_event: &mut dyn FnMut(PullEvent),
    ) -> std::result::Result<(), LaunchError> {
        self.pull(image)
    }
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

    fn pull_streaming(
        &self,
        image: &str,
        on_event: &mut dyn FnMut(PullEvent),
    ) -> std::result::Result<(), LaunchError> {
        let mut child = Command::new("docker")
            .args(["pull", image])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| LaunchError::Other {
                message: format!("falha ao executar docker pull: {e}"),
            })?;

        // Docker pull rewrites the same line with `\r` for progress bars
        // when stdout is a TTY; over a pipe it emits one line per status
        // change instead, which is what we want. Just read line-by-line.
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Some(event) = parse_pull_progress_line(&line) {
                    on_event(event);
                }
            }
        }

        let status = child.wait().map_err(|e| LaunchError::Other {
            message: format!("docker pull wait: {e}"),
        })?;
        if status.success() {
            return Ok(());
        }
        // Drain stderr for the error message.
        let stderr = child
            .stderr
            .take()
            .and_then(|mut s| {
                let mut buf = String::new();
                use std::io::Read;
                s.read_to_string(&mut buf).ok().map(|_| buf)
            })
            .unwrap_or_default();
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
    if lower.contains("wsl integration is not enabled")
        || lower.contains("docker desktop is not running, wsl-distro")
    {
        let distro = extract_wsl_distro(stderr).unwrap_or_else(|| "<unknown>".into());
        return LaunchError::DockerDesktopWslIntegrationOff { distro };
    }
    if lower.contains("there is no distribution with the supplied name")
        || lower.contains("wsl/service/createinstance/registerdistro")
        || lower.contains("error_distro_not_found")
    {
        let distro = extract_wsl_distro(stderr).unwrap_or_else(|| "<unknown>".into());
        return LaunchError::WslDistroDown { distro };
    }
    if lower.contains("no space left on device") || lower.contains("disk quota exceeded") {
        let path = extract_diskfull_path(stderr).unwrap_or_else(|| "<unknown>".into());
        return LaunchError::DiskFull {
            path,
            needed_bytes: 0,
        };
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

/// Pull a WSL distribution name out of common Docker Desktop / wsl.exe
/// error messages. Matches `wsl-distro 'Ubuntu'`, `for 'docker-desktop'`,
/// `distribution "Ubuntu-24.04"`, etc.
fn extract_wsl_distro(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        for marker in [
            "wsl-distro '",
            "wsl-distro \"",
            "distribution '",
            "distribution \"",
            "for distro '",
            "for distro \"",
        ] {
            if let Some(rest) = line.split_once(marker) {
                let close = marker.chars().last().unwrap();
                if let Some(end) = rest.1.find(close) {
                    let candidate = &rest.1[..end];
                    if !candidate.is_empty() {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Parse a single `docker pull` stdout line into a [`PullEvent`]. Pure
/// function — drives both the streaming impl and unit tests.
///
/// Lines we care about look like:
///
/// * `abc123def456: Pulling fs layer`
/// * `abc123def456: Downloading [==>           ] 12.3MB/45.6MB`
/// * `abc123def456: Download complete`
/// * `abc123def456: Extracting [==>           ] 12.3MB/45.6MB`
/// * `abc123def456: Pull complete`
///
/// Returns `None` for housekeeping lines (`Status:`, `Digest:`,
/// `Using default tag:`, empty lines).
pub fn parse_pull_progress_line(line: &str) -> Option<PullEvent> {
    let line = line.trim_end_matches('\r');
    let (head, rest) = line.split_once(':')?;
    let layer = head.trim();
    let rest = rest.trim();
    if layer.is_empty() || rest.is_empty() {
        return None;
    }
    // Skip non-layer headers.
    let lower = layer.to_lowercase();
    if matches!(
        lower.as_str(),
        "status" | "digest" | "using default tag" | "latest"
    ) {
        return None;
    }
    // Layer ids are short hex strings; reject anything that doesn't
    // look like one to avoid grabbing arbitrary "foo: bar" output.
    if !layer.chars().all(|c| c.is_ascii_hexdigit()) || layer.len() < 6 {
        return None;
    }
    // Split off the progress bar (if any) from the status text.
    let (status, progress) = match rest.find('[') {
        Some(idx) => (rest[..idx].trim(), &rest[idx..]),
        None => (rest, ""),
    };
    let (current_bytes, total_bytes) = parse_progress_bytes(progress);
    Some(PullEvent {
        layer: layer.to_string(),
        status: status.to_string(),
        current_bytes,
        total_bytes,
    })
}

/// Extract `(current, total)` from `[========>     ] 12.3MB/45.6MB`.
/// Returns `(None, None)` if no `/` separator is present.
fn parse_progress_bytes(s: &str) -> (Option<u64>, Option<u64>) {
    let Some(slash) = s.rfind('/') else {
        return (None, None);
    };
    let before = &s[..slash];
    let after = &s[slash + 1..];
    let last_token = |chunk: &str| -> Option<u64> {
        chunk
            .split_whitespace()
            .next_back()
            .and_then(parse_size_token)
    };
    let first_token =
        |chunk: &str| -> Option<u64> { chunk.split_whitespace().next().and_then(parse_size_token) };
    (last_token(before), first_token(after))
}

/// Parse `12.3MB`, `456KB`, `1.2GiB`, `789B` to bytes. Accepts both
/// SI (decimal, MB = 10^6) and IEC (`MiB` = 2^20) suffixes. Returns
/// None if the token doesn't look like a size.
fn parse_size_token(tok: &str) -> Option<u64> {
    let tok = tok.trim();
    if tok.is_empty() {
        return None;
    }
    let split = tok
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(tok.len());
    let (num, unit) = tok.split_at(split);
    let n: f64 = num.parse().ok()?;
    if n < 0.0 {
        return None;
    }
    let mult: f64 = match unit.trim() {
        "" | "B" => 1.0,
        "KB" => 1_000.0,
        "KiB" | "K" => 1_024.0,
        "MB" => 1_000_000.0,
        "MiB" | "M" => 1_048_576.0,
        "GB" => 1_000_000_000.0,
        "GiB" | "G" => 1_073_741_824.0,
        "TB" => 1_000_000_000_000.0,
        "TiB" | "T" => 1_099_511_627_776.0,
        _ => return None,
    };
    Some((n * mult) as u64)
}

/// Best-effort path extraction from `no space left on device` errors.
/// Docker tends to mention the mount point that filled, e.g.
/// `failed to write to /var/lib/docker/...`. Falls back to None if the
/// line doesn't carry a path.
fn extract_diskfull_path(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        if !line.to_lowercase().contains("no space left on device")
            && !line.to_lowercase().contains("disk quota exceeded")
        {
            continue;
        }
        for token in line.split_whitespace() {
            let candidate =
                token.trim_matches(|c: char| c == ',' || c == ':' || c == '"' || c == '\'');
            if candidate.starts_with('/') && candidate.len() > 1 {
                return Some(candidate.to_string());
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

// preflight_freerdp removed: connect path always opens the browser now.

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

    #[test]
    fn classify_recognizes_docker_desktop_wsl_integration_off() {
        let stderr =
            "error during connect: WSL integration is not enabled for distribution 'Ubuntu-24.04'.";
        match classify_compose_stderr(stderr) {
            LaunchError::DockerDesktopWslIntegrationOff { distro } => {
                assert_eq!(distro, "Ubuntu-24.04")
            }
            other => panic!("expected DockerDesktopWslIntegrationOff, got {other:?}"),
        }
    }

    #[test]
    fn classify_recognizes_wsl_distro_down() {
        let stderr =
            "There is no distribution with the supplied name. Error: WSL_E_DISTRO_NOT_FOUND";
        assert!(matches!(
            classify_compose_stderr(stderr),
            LaunchError::WslDistroDown { .. }
        ));
    }

    #[test]
    fn classify_recognizes_disk_full() {
        let stderr =
            "failed to write blob: write /var/lib/docker/tmp/abc.tar: no space left on device";
        match classify_compose_stderr(stderr) {
            LaunchError::DiskFull { path, .. } => {
                assert!(path.starts_with('/'), "expected absolute path, got {path}");
            }
            other => panic!("expected DiskFull, got {other:?}"),
        }
    }

    #[test]
    fn extract_wsl_distro_handles_quoted_forms() {
        let line = "Docker Desktop is not running, wsl-distro 'docker-desktop' missing";
        assert_eq!(extract_wsl_distro(line).as_deref(), Some("docker-desktop"));
    }

    #[test]
    fn parse_pull_progress_line_reads_downloading_with_size() {
        let line = "abc123def456: Downloading [============>                                      ]  12.3MB/45.6MB";
        let ev = parse_pull_progress_line(line).expect("should parse");
        assert_eq!(ev.layer, "abc123def456");
        assert_eq!(ev.status, "Downloading");
        assert_eq!(ev.current_bytes, Some(12_300_000));
        assert_eq!(ev.total_bytes, Some(45_600_000));
    }

    #[test]
    fn parse_pull_progress_line_reads_pulling_fs_layer() {
        let line = "abc123def456: Pulling fs layer";
        let ev = parse_pull_progress_line(line).expect("should parse");
        assert_eq!(ev.layer, "abc123def456");
        assert_eq!(ev.status, "Pulling fs layer");
        assert_eq!(ev.current_bytes, None);
        assert_eq!(ev.total_bytes, None);
    }

    #[test]
    fn parse_pull_progress_line_reads_pull_complete() {
        let line = "abc123def456: Pull complete";
        let ev = parse_pull_progress_line(line).expect("should parse");
        assert_eq!(ev.status, "Pull complete");
    }

    #[test]
    fn parse_pull_progress_line_skips_status_and_digest() {
        assert!(parse_pull_progress_line(
            "Status: Downloaded newer image for dockurr/windows:latest"
        )
        .is_none());
        assert!(parse_pull_progress_line("Digest: sha256:abc").is_none());
        assert!(parse_pull_progress_line("Using default tag: latest").is_none());
    }

    #[test]
    fn parse_pull_progress_line_skips_empty_and_malformed() {
        assert!(parse_pull_progress_line("").is_none());
        assert!(parse_pull_progress_line("not even a colon here").is_none());
        // Layer ids must be hex >= 6 chars.
        assert!(parse_pull_progress_line("xx: Downloading").is_none());
    }

    #[test]
    fn parse_size_token_handles_si_and_iec() {
        assert_eq!(parse_size_token("12.3MB"), Some(12_300_000));
        assert_eq!(
            parse_size_token("12.3MiB"),
            Some((12.3 * 1_048_576.0) as u64)
        );
        assert_eq!(parse_size_token("1.5GB"), Some(1_500_000_000));
        assert_eq!(parse_size_token("1.5GiB"), Some(1_610_612_736));
        assert_eq!(parse_size_token("456B"), Some(456));
        assert_eq!(parse_size_token("789"), Some(789));
        assert_eq!(parse_size_token("garbage"), None);
    }

    #[test]
    fn pull_streaming_default_impl_delegates_to_pull() {
        let docker = MockDocker::new();
        let mut events: Vec<PullEvent> = Vec::new();
        docker
            .pull_streaming("foo:latest", &mut |ev| events.push(ev))
            .unwrap();
        // Default impl emits no events — verifies the trait fallback.
        assert!(events.is_empty());
        assert!(docker.calls().iter().any(|c| c.starts_with("pull:")));
    }
}

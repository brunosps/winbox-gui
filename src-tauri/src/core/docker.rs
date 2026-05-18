use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use super::paths;

/// Abstraction over the Docker CLI. Real callers use [`CliDocker`]; tests
/// inject [`mock::MockDocker`] (or any custom impl) to drive the launch
/// path without a real daemon.
pub trait DockerClient {
    fn container_status(&self, name: &str) -> String;
    fn compose_run(&self, profile: &str, action_args: &[&str]) -> Result<()>;
    fn logs(&self, container: &str, extra: &[&str]) -> Result<String>;
    fn logs_contains(&self, container: &str, needle: &str) -> bool;
    fn pause(&self, container: &str) -> Result<()>;
    fn unpause(&self, container: &str) -> Result<()>;
    fn stop(&self, container: &str, timeout: u32) -> Result<()>;
    fn kill(&self, container: &str) -> Result<()>;
    fn rm_force(&self, container: &str) -> Result<()>;
    fn pull(&self, image: &str) -> Result<()>;
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

    fn compose_run(&self, profile: &str, action_args: &[&str]) -> Result<()> {
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

    fn pull(&self, image: &str) -> Result<()> {
        let ok = Command::new("docker")
            .args(["pull", image])
            .status()?
            .success();
        if !ok {
            bail!("docker pull failed");
        }
        Ok(())
    }
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
    CliDocker.compose_run(profile, action_args)
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
    CliDocker.pull(image)
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

#[cfg(test)]
pub mod mock {
    //! Test double: records every call and returns scripted values.

    use super::DockerClient;
    use anyhow::{bail, Result};
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct MockDocker {
        /// Container name → simulated `docker ps` State string (e.g. "running",
        /// "exited", "paused"). Missing entries default to "absent".
        pub statuses: RefCell<HashMap<String, String>>,
        /// Container × needle → simulated logs_contains result.
        pub logs_seed: RefCell<HashMap<(String, String), bool>>,
        /// Ordered method calls captured for assertion (e.g. `"rm_force:winbox-x"`).
        pub calls: RefCell<Vec<String>>,
        /// If set, every `compose_run` returns this error verbatim.
        pub fail_compose: RefCell<Option<String>>,
        /// If set, every `pull` returns this error.
        pub fail_pull: RefCell<Option<String>>,
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
        pub fn set_compose_failure(&self, msg: &str) {
            *self.fail_compose.borrow_mut() = Some(msg.to_string());
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
        fn compose_run(&self, profile: &str, action_args: &[&str]) -> Result<()> {
            self.record("compose", &format!("{}:{}", profile, action_args.join(" ")));
            if let Some(msg) = self.fail_compose.borrow().clone() {
                bail!("{}", msg);
            }
            // Side effect: simulate "container created and running" so subsequent
            // status() calls behave as expected without test boilerplate.
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
        fn pull(&self, image: &str) -> Result<()> {
            self.record("pull", image);
            if let Some(msg) = self.fail_pull.borrow().clone() {
                bail!("{}", msg);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockDocker;
    use super::DockerClient;

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
        // compose_run "up" should have transitioned the simulated status.
        assert_eq!(docker.container_status("winbox-foo"), "running");
    }

    #[test]
    fn mock_status_defaults_to_absent() {
        let docker = MockDocker::new();
        assert_eq!(docker.container_status("nope"), "absent");
    }

    #[test]
    fn mock_compose_failure_propagates() {
        let docker = MockDocker::new();
        docker.set_compose_failure("simulated EACCES");
        let err = docker.compose_run("foo", &["up", "-d"]).unwrap_err();
        assert!(err.to_string().contains("simulated EACCES"));
    }
}

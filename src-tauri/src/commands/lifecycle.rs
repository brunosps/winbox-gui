use anyhow::{bail, Result};
use std::path::Path;

use crate::core::env_file;
use crate::core::image_family::ImageFamily;
use crate::core::launch_error::OfficeError;
use crate::core::{desktop, docker, gpu_hooks, paths, profile, snapshots, winapps};

pub fn pause(name: &str) -> Result<()> {
    pause_with_confirm(name, None)
}

pub fn pause_with_confirm(name: &str, confirm_code: Option<&str>) -> Result<()> {
    ensure_office_lifecycle_allowed(name, LifecycleAction::Pause, confirm_code)?;
    let c = paths::profile_container(name);
    docker::pause(&c)
}

pub fn resume(name: &str) -> Result<()> {
    let c = paths::profile_container(name);
    docker::unpause(&c)
}

pub fn stop(name: &str) -> Result<()> {
    stop_with_confirm(name, None)
}

pub fn stop_with_confirm(name: &str, confirm_code: Option<&str>) -> Result<()> {
    ensure_office_lifecycle_allowed(name, LifecycleAction::Stop, confirm_code)?;
    let c = paths::profile_container(name);
    docker::stop(&c, 120)?;
    gpu_hooks::cleanup_after_stop(name);
    Ok(())
}

/// "Force kill" semantics for the UI. We learned the hard way that
/// `docker kill` (plain SIGKILL) doesn't recover containers stuck in
/// the "tried to kill container, but did not receive an exit event"
/// state — the daemon hangs forever waiting for an exit notification
/// the kernel already swallowed. `docker rm -f` does the same SIGKILL
/// but follows up with `containerd-shim` cleanup that also rescues
/// zombie states. We try kill first as a courtesy (most healthy
/// containers exit cleanly that way), then unconditionally run
/// `rm -f` to guarantee the slot is free for a re-launch.
pub fn kill(name: &str) -> Result<()> {
    let c = paths::profile_container(name);
    let _ = docker::kill(&c); // best-effort SIGKILL; ignore daemon errors
    docker::rm_force(&c)?;
    gpu_hooks::cleanup_after_stop(name);
    Ok(())
}

pub fn restart(name: &str) -> Result<()> {
    restart_with_confirm(name, None)
}

pub fn restart_with_confirm(name: &str, confirm_code: Option<&str>) -> Result<()> {
    ensure_office_lifecycle_allowed(name, LifecycleAction::Restart, confirm_code)?;
    let _ = docker::compose_run(name, &["down"]);
    gpu_hooks::cleanup_after_stop(name);
    gpu_hooks::prepare_for_start(name)?;
    docker::compose_run(name, &["up", "-d"])
}

/// snapshot + pull + recreate
pub fn update(name: &str) -> Result<()> {
    let snap = snapshots::timestamped_snapshot_name("pre-update");
    snapshots::create(name, &snap)?;
    let env = env_file::read(&paths::profile_env_file(name))?;
    let image = ImageFamily::from_env_map(&env).docker_image();
    docker::pull(image)?;
    gpu_hooks::prepare_for_start(name)?;
    docker::compose_run(name, &["up", "-d"])
}

/// Remove a profile. If `delete_storage` is true and the profile was
/// created with a custom `STORAGE_DIR` (i.e. somewhere outside the
/// default `profile_data_dir`), that directory is also deleted on disk.
/// `profile::remove_tree` already removes the default storage path
/// (which lives under `profile_data_dir`), so the custom-path branch
/// only fires when the user picked a path elsewhere.
pub fn remove(name: &str, delete_storage: bool) -> Result<()> {
    if !profile::exists(name) {
        bail!("Perfil '{}' não existe.", name);
    }

    // Capture custom storage path BEFORE remove_tree wipes the env file.
    let custom_storage = if delete_storage {
        env_file::read(&paths::profile_env_file(name))
            .ok()
            .and_then(|map| {
                let raw = env_file::get(&map, "STORAGE_DIR");
                let raw = raw.trim();
                if raw.is_empty() {
                    return None;
                }
                let custom = std::path::PathBuf::from(raw);
                let default = paths::profile_storage_dir(name);
                if custom == default {
                    None
                } else {
                    Some(custom)
                }
            })
    } else {
        None
    };

    let c = paths::profile_container(name);
    let _ = docker::compose_run(name, &["down"]);
    let _ = docker::rm_force(&c);
    gpu_hooks::cleanup_after_stop(name);
    profile::remove_tree(name)?;

    if let Some(path) = custom_storage {
        if path.exists() {
            // Best-effort: ignore errors so a broken/locked custom path
            // doesn't leave the rest of the cleanup half-done.
            let _ = std::fs::remove_dir_all(&path);
        }
    }

    if profile::get_default().as_deref() == Some(name) {
        let others = profile::list_names();
        if let Some(next) = others.into_iter().next() {
            profile::set_default(&next)?;
        } else {
            profile::clear_default()?;
        }
    }
    let _ = desktop::regenerate();
    Ok(())
}

pub fn set_default(name: &str) -> Result<()> {
    if !profile::exists(name) {
        bail!("Perfil '{}' não existe.", name);
    }
    profile::set_default(name)?;
    let _ = desktop::regenerate();
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleAction {
    Stop,
    Pause,
    Restart,
}

impl LifecycleAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stop => "stop",
            Self::Pause => "pause",
            Self::Restart => "restart",
        }
    }
}

fn ensure_office_lifecycle_allowed(
    profile: &str,
    action: LifecycleAction,
    confirm_code: Option<&str>,
) -> Result<()> {
    let docker = docker::CliDocker;
    let probe = winapps::CliRdpSessionProbe;
    ensure_office_lifecycle_allowed_at(
        profile,
        &paths::profile_env_file(profile),
        action,
        confirm_code,
        &docker,
        &probe,
    )
}

fn ensure_office_lifecycle_allowed_at(
    profile: &str,
    env_path: &Path,
    action: LifecycleAction,
    confirm_code: Option<&str>,
    docker: &dyn docker::DockerClient,
    probe: &dyn winapps::RdpSessionProbe,
) -> Result<()> {
    let Ok(env) = env_file::read(env_path) else {
        return Ok(());
    };
    if env_file::get(&env, "PROFILE_KIND").trim() != "office" {
        return Ok(());
    }
    if confirm_code == Some(OfficeError::OFFICE_APPS_MAYBE_OPEN) {
        return Ok(());
    }

    let container = paths::profile_container(profile);
    let vm_running = docker.container_status(&container) == "running";
    if !vm_running {
        return Ok(());
    }

    let active_sessions =
        winapps::detect_active_rdp_sessions(probe, env_file::get_u16(&env, "RDP_PORT"));
    let requires_confirmation = active_sessions.unwrap_or(true);
    if requires_confirmation {
        bail!(
            "{}: Perfil Office '{}' pode ter aplicativos abertos. Confirme com code '{}' para executar {}.",
            OfficeError::OFFICE_APPS_MAYBE_OPEN,
            profile,
            OfficeError::OFFICE_APPS_MAYBE_OPEN,
            action.as_str()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::docker::mock::MockDocker;
    use anyhow::Result;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MockRdpSessionProbe {
        pgrep: Result<winapps::RdpSessionCommandOutput>,
        ss: Result<winapps::RdpSessionCommandOutput>,
    }

    impl winapps::RdpSessionProbe for MockRdpSessionProbe {
        fn pgrep_xfreerdp(&self) -> Result<winapps::RdpSessionCommandOutput> {
            self.pgrep
                .as_ref()
                .map(Clone::clone)
                .map_err(|err| anyhow::anyhow!("{err:#}"))
        }

        fn ss_tcp_processes(&self) -> Result<winapps::RdpSessionCommandOutput> {
            self.ss
                .as_ref()
                .map(Clone::clone)
                .map_err(|err| anyhow::anyhow!("{err:#}"))
        }
    }

    #[test]
    fn office_lifecycle_blocks_when_apps_maybe_open() {
        let root = temp_dir("apps-open");
        let env_path = root.join("config.env");
        std::fs::write(&env_path, "PROFILE_KIND=office\nRDP_PORT=3391\n")
            .expect("env should be written");
        let docker = MockDocker::new();
        docker.seed_status("winbox-office", "running");
        let probe = active_probe();

        let err = ensure_office_lifecycle_allowed_at(
            "office",
            &env_path,
            LifecycleAction::Stop,
            None,
            &docker,
            &probe,
        )
        .expect_err("apps abertas exigem confirmação");
        assert!(err
            .to_string()
            .contains(OfficeError::OFFICE_APPS_MAYBE_OPEN));

        ensure_office_lifecycle_allowed_at(
            "office",
            &env_path,
            LifecycleAction::Pause,
            Some(OfficeError::OFFICE_APPS_MAYBE_OPEN),
            &docker,
            &probe,
        )
        .expect("confirm-code compartilhado libera a ação");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn indeterminate_session_warns_for_running_office_profile() {
        let root = temp_dir("indeterminate");
        let env_path = root.join("config.env");
        std::fs::write(&env_path, "PROFILE_KIND=office\nRDP_PORT=3391\n")
            .expect("env should be written");
        let docker = MockDocker::new();
        docker.seed_status("winbox-office", "running");
        let probe = MockRdpSessionProbe {
            pgrep: Ok(rdp_ok("1234 xfreerdp /v:127.0.0.1:3391\n")),
            ss: Err(anyhow::anyhow!("ss indisponível")),
        };

        let err = ensure_office_lifecycle_allowed_at(
            "office",
            &env_path,
            LifecycleAction::Restart,
            None,
            &docker,
            &probe,
        )
        .expect_err("detecção indeterminada não libera stop silencioso");
        assert!(err
            .to_string()
            .contains(OfficeError::OFFICE_APPS_MAYBE_OPEN));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_lifecycle_allows_non_office_or_no_active_session() {
        let root = temp_dir("inactive");
        let env_path = root.join("config.env");
        std::fs::write(&env_path, "PROFILE_KIND=standard\nRDP_PORT=3391\n")
            .expect("env should be written");
        let docker = MockDocker::new();
        docker.seed_status("winbox-office", "running");
        let probe = active_probe();

        ensure_office_lifecycle_allowed_at(
            "office",
            &env_path,
            LifecycleAction::Stop,
            None,
            &docker,
            &probe,
        )
        .expect("perfil não Office usa lifecycle existente");

        std::fs::write(&env_path, "PROFILE_KIND=office\nRDP_PORT=3391\n")
            .expect("env should be rewritten");
        let inactive = MockRdpSessionProbe {
            pgrep: Ok(winapps::RdpSessionCommandOutput {
                success: false,
                status_code: Some(1),
                stdout: String::new(),
                stderr: String::new(),
            }),
            ss: Ok(rdp_ok("")),
        };
        ensure_office_lifecycle_allowed_at(
            "office",
            &env_path,
            LifecycleAction::Stop,
            None,
            &docker,
            &inactive,
        )
        .expect("perfil Office running sem sessão ativa pode parar");

        let _ = std::fs::remove_dir_all(root);
    }

    fn active_probe() -> MockRdpSessionProbe {
        MockRdpSessionProbe {
            pgrep: Ok(rdp_ok("1234 xfreerdp /v:127.0.0.1:3391\n")),
            ss: Ok(rdp_ok(
                "ESTAB 0 0 127.0.0.1:52122 127.0.0.1:3391 users:((\"xfreerdp\",pid=1234,fd=7))\n",
            )),
        }
    }

    fn rdp_ok(stdout: &str) -> winapps::RdpSessionCommandOutput {
        winapps::RdpSessionCommandOutput {
            success: true,
            status_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "winbox-lifecycle-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }
}

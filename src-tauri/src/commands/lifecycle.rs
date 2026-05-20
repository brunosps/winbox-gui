use anyhow::{bail, Result};

use crate::core::env_file;
use crate::core::image_family::ImageFamily;
use crate::core::{desktop, docker, gpu_hooks, paths, profile, snapshots};

pub fn pause(name: &str) -> Result<()> {
    let c = paths::profile_container(name);
    docker::pause(&c)
}

pub fn resume(name: &str) -> Result<()> {
    let c = paths::profile_container(name);
    docker::unpause(&c)
}

pub fn stop(name: &str) -> Result<()> {
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

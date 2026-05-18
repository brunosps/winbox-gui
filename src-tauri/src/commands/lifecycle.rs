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

pub fn kill(name: &str) -> Result<()> {
    let c = paths::profile_container(name);
    docker::kill(&c)?;
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

pub fn remove(name: &str) -> Result<()> {
    if !profile::exists(name) {
        bail!("Perfil '{}' não existe.", name);
    }
    let c = paths::profile_container(name);
    let _ = docker::compose_run(name, &["down"]);
    let _ = docker::rm_force(&c);
    gpu_hooks::cleanup_after_stop(name);
    profile::remove_tree(name)?;

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

//! Pre-flight check for GPU passthrough. The dynamic bind/unbind era is over:
//! the GPU is either permanently dedicated to vfio-pci (configured via
//! `vfio_setup::apply` + reboot) or there's no passthrough at all.
//!
//! `prepare_for_start` exists only to fail fast and loudly when a profile
//! references a GPU that the host driver still owns — the VM would crash
//! with an opaque QEMU error otherwise.

use anyhow::{bail, Result};

use super::{env_file, gpu_bind, paths};

fn read_profile_bdf(profile: &str) -> Option<String> {
    let env_path = paths::profile_env_file(profile);
    if !env_path.is_file() {
        return None;
    }
    let map = env_file::read(&env_path).ok()?;
    let bdf = env_file::get(&map, "GPU_BDF").trim().to_string();
    if bdf.is_empty() {
        None
    } else {
        Some(bdf)
    }
}

/// Refuse to start the container if the profile asks for a GPU but the GPU
/// is not yet on vfio-pci. Surfaced to the UI as a "configure host + reboot"
/// prompt rather than a silent QEMU failure.
pub fn prepare_for_start(profile: &str) -> Result<()> {
    let Some(bdf) = read_profile_bdf(profile) else {
        return Ok(());
    };
    if gpu_bind::is_vfio(&bdf) {
        return Ok(());
    }
    let driver = gpu_bind::current_driver(&bdf).unwrap_or_else(|| "(nenhum)".into());
    bail!(
        "GPU {bdf} ainda está com o driver '{driver}' do host. \
         Abra Settings → GPU, clique em 'Configurar sistema' e reinicie."
    )
}

/// Stop-side hook is intentionally a no-op: the GPU stays dedicated until
/// the user explicitly reverts via Settings.
pub fn cleanup_after_stop(_profile: &str) {}

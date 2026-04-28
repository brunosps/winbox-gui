//! Host-side configuration so a PCI GPU is permanently dedicated to the VM
//! via vfio-pci. Replaces the dynamic bind/unbind dance: one pkexec prompt
//! at enable time, then it's just there until the user reverts it.
//!
//! What `apply` writes:
//!   - /etc/modprobe.d/winbox-vfio-<bdf>.conf  (blacklist host driver, vfio-pci ids)
//!   - /etc/initramfs-tools/modules            (vfio, vfio_iommu_type1, vfio_pci)
//!     Then runs `update-initramfs -u`. User is asked to reboot.
//!
//! Idempotent: `apply` again with the same BDF is a no-op (same content,
//! same lines already present in initramfs modules).

use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::path::PathBuf;

use super::{gpu_bind, validation};

#[derive(Debug, Serialize, Clone)]
pub struct SetupStatus {
    /// vfio-pci is currently the driver bound to this BDF.
    pub configured: bool,
    /// Modprobe drop-in for this BDF exists with the right ids.
    pub modprobe_written: bool,
    /// vfio modules are listed for inclusion in initramfs.
    pub initramfs_ready: bool,
    /// Setup files are written but the running kernel still has the host
    /// driver bound — a reboot will activate vfio-pci.
    pub needs_reboot: bool,
}

pub fn status(bdf: &str) -> SetupStatus {
    let bound_to_vfio = gpu_bind::is_vfio(bdf);
    let modprobe_written = modprobe_path(bdf).map(|p| p.is_file()).unwrap_or(false);
    let initramfs_ready = initramfs_modules_present();
    SetupStatus {
        configured: bound_to_vfio,
        modprobe_written,
        initramfs_ready,
        needs_reboot: !bound_to_vfio && modprobe_written && initramfs_ready,
    }
}

/// Validate the BDF and run the privileged setup script via pkexec.
pub fn apply(bdf: &str) -> Result<SetupStatus> {
    validation::validate_bdf(bdf)?;
    let (vendor_id, device_id) = read_pci_id(bdf)?;
    let conf_name = modprobe_filename(bdf);
    let script = build_setup_script(bdf, &vendor_id, &device_id, &conf_name);
    gpu_bind::run_as_root(&script).context("aplicando configuração de host (vfio-pci)")?;
    Ok(status(bdf))
}

/// Remove the modprobe drop-in for this BDF and refresh initramfs.
/// Leaves the global initramfs-tools/modules entries alone — they're harmless
/// without the modprobe ids and other GPUs may still want vfio.
pub fn revert(bdf: &str) -> Result<SetupStatus> {
    validation::validate_bdf(bdf)?;
    let conf_name = modprobe_filename(bdf);
    let script = build_revert_script(&conf_name);
    gpu_bind::run_as_root(&script).context("removendo configuração de host (vfio-pci)")?;
    Ok(status(bdf))
}

fn read_pci_id(bdf: &str) -> Result<(String, String)> {
    let v = std::fs::read_to_string(format!("/sys/bus/pci/devices/{bdf}/vendor"))
        .map_err(|e| anyhow!("lendo vendor de {bdf}: {e}"))?;
    let d = std::fs::read_to_string(format!("/sys/bus/pci/devices/{bdf}/device"))
        .map_err(|e| anyhow!("lendo device de {bdf}: {e}"))?;
    Ok((normalize_id(&v), normalize_id(&d)))
}

/// "0x10de\n" -> "10de"
fn normalize_id(raw: &str) -> String {
    raw.trim().trim_start_matches("0x").to_lowercase()
}

fn modprobe_filename(bdf: &str) -> String {
    let safe: String = bdf
        .chars()
        .map(|c| if c == ':' || c == '.' { '-' } else { c })
        .collect();
    format!("winbox-vfio-{safe}.conf")
}

fn modprobe_path(bdf: &str) -> Option<PathBuf> {
    Some(PathBuf::from("/etc/modprobe.d").join(modprobe_filename(bdf)))
}

fn initramfs_modules_present() -> bool {
    let Ok(text) = std::fs::read_to_string("/etc/initramfs-tools/modules") else {
        return false;
    };
    let needles = ["vfio", "vfio_iommu_type1", "vfio_pci"];
    needles.iter().all(|n| {
        text.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#') && t.split_whitespace().next() == Some(n)
        })
    })
}

fn build_setup_script(bdf: &str, vendor: &str, device: &str, conf_name: &str) -> String {
    // Sanity: the inputs were validated, but we're about to interpolate them
    // into a root shell. Re-quote defensively with single quotes around
    // everything except the heredoc body (which is single-quoted EOF).
    format!(
        r#"#!/bin/sh
set -e

CONF=/etc/modprobe.d/{conf_name}
cat > "$CONF" <<'EOF'
# managed by winbox — passthrough da GPU {bdf} ({vendor}:{device})
blacklist nouveau
blacklist nvidia
blacklist nvidia_drm
blacklist nvidia_modeset
blacklist nvidia_uvm
options vfio-pci ids={vendor}:{device}
softdep nvidia pre: vfio-pci
softdep nouveau pre: vfio-pci
EOF
chmod 0644 "$CONF"

MODFILE=/etc/initramfs-tools/modules
if [ -f "$MODFILE" ]; then
  for m in vfio vfio_iommu_type1 vfio_pci; do
    if ! grep -qE "^[[:space:]]*${{m}}([[:space:]]|$)" "$MODFILE"; then
      printf '%s\n' "$m" >> "$MODFILE"
    fi
  done
fi

if command -v update-initramfs >/dev/null 2>&1; then
  update-initramfs -u
fi
"#
    )
}

fn build_revert_script(conf_name: &str) -> String {
    format!(
        r#"#!/bin/sh
set -e
CONF=/etc/modprobe.d/{conf_name}
if [ -f "$CONF" ]; then
  rm -f "$CONF"
fi
if command -v update-initramfs >/dev/null 2>&1; then
  update-initramfs -u
fi
"#
    )
}

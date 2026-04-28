//! Read-only inspection of PCI GPU binding state, plus the privileged
//! shell-execution helper used by `vfio_setup` to write to /etc and rebuild
//! initramfs in a single pkexec prompt.
//!
//! Historical note: this module used to also do dynamic bind/unbind to
//! vfio-pci at VM start/stop. That mode is gone — it raced badly with
//! display compositors holding `/dev/nvidia0`. Passthrough is now always
//! configured statically once and persisted across boots.

use anyhow::{bail, Context, Result};
use std::process::Command;

/// Read the driver currently bound to this device, if any.
pub fn current_driver(bdf: &str) -> Option<String> {
    let link = format!("/sys/bus/pci/devices/{}/driver", bdf);
    let target = std::fs::read_link(&link).ok()?;
    target
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// True if device is already bound to vfio-pci.
pub fn is_vfio(bdf: &str) -> bool {
    current_driver(bdf).as_deref() == Some("vfio-pci")
}

/// Find the related "function 1" of a multifunction GPU (usually the HDMI audio).
/// Kept for callers that may want to apply policies to the whole device.
pub fn sibling_functions(bdf: &str) -> Vec<String> {
    let Some(prefix) = bdf.rsplit_once('.').map(|(p, _)| p) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let base = "/sys/bus/pci/devices";
    let Ok(entries) = std::fs::read_dir(base) else {
        return out;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(n) = name.to_str() else { continue };
        if n == bdf {
            continue;
        }
        if n.starts_with(&format!("{}.", prefix)) {
            out.push(n.to_string());
        }
    }
    out
}

/// Run a shell script as root via pkexec. Inherits stdout/stderr so polkit
/// error messages reach the user. Falls back to `sudo -n` only when pkexec
/// is missing — we never put up a terminal-only password prompt from the GUI.
pub fn run_as_root(script: &str) -> Result<()> {
    if which_bin("pkexec") {
        let status = Command::new("pkexec")
            .arg("sh")
            .arg("-c")
            .arg(script)
            .status()
            .context("executing pkexec")?;
        if !status.success() {
            bail!("pkexec falhou (exit={:?})", status.code());
        }
        return Ok(());
    }
    if which_bin("sudo") {
        let status = Command::new("sudo")
            .args(["-n", "sh", "-c", script])
            .status()
            .context("executing sudo -n")?;
        if !status.success() {
            bail!(
                "sudo falhou (exit={:?}) — instale pkexec ou rode winbox-gui como root",
                status.code()
            );
        }
        return Ok(());
    }
    bail!("pkexec e sudo não encontrados — impossível escalar privilégio")
}

fn which_bin(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    path.split(':')
        .any(|p| std::path::Path::new(p).join(name).is_file())
}

/// Heuristic: is this a laptop with NVIDIA Optimus (iGPU + dGPU both present)?
/// Used to warn users that passthrough on such systems often fails despite
/// correct config.
pub fn is_likely_optimus() -> bool {
    let chassis = std::fs::read_to_string("/sys/class/dmi/id/chassis_type")
        .unwrap_or_default()
        .trim()
        .to_string();
    let is_laptop = matches!(chassis.as_str(), "8" | "9" | "10" | "14");
    if !is_laptop {
        return false;
    }
    let Ok(out) = Command::new("lspci").output() else {
        return false;
    };
    let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
    let has_nvidia = s.contains("nvidia");
    let has_integrated = s.contains("intel") || s.contains("amd") || s.contains("advanced micro");
    has_nvidia && has_integrated
}

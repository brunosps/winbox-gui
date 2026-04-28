use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize, Clone)]
pub struct GpuInfo {
    /// PCI bus:device:function, e.g. "0000:01:00.0"
    pub bdf: String,
    pub vendor: String,
    pub model: String,
    pub driver: Option<String>,
    pub iommu_group: Option<u32>,
    /// True if this GPU is currently bound to vfio-pci.
    pub vfio_ready: bool,
    /// True if the host has IOMMU enabled (the only hard prerequisite for
    /// configuring vfio-pci passthrough).
    pub vfio_capable: bool,
    /// Heuristic: laptop with iGPU + NVIDIA dGPU (Optimus). Passthrough there
    /// is notoriously unreliable — surface it to the user so they know why.
    pub likely_optimus: bool,
    /// True if this GPU is currently driving the host display (has a
    /// connected DRM output and isn't on vfio-pci yet). Frontend should
    /// disable selection when this is the *only* display GPU on the host.
    pub is_primary_display: bool,
    pub vfio_notes: Vec<String>,
}

/// Detect GPUs on the host via `lspci -nn -D -k`.
pub fn list() -> Vec<GpuInfo> {
    let out = match Command::new("lspci").args(["-nn", "-D", "-k"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let iommu_ok = Path::new("/sys/kernel/iommu_groups").exists();
    let vfio_loaded = is_module_loaded("vfio_pci") || is_module_loaded("vfio-pci");

    let mut gpus = Vec::new();
    // Parse records: each GPU entry is an lspci record "<bdf> <class>: <vendor> <model>"
    // followed by indented lines "Kernel driver in use:" / "Kernel modules:".
    let mut current: Option<(String, String, String)> = None; // (bdf, vendor, model)
    let mut driver: Option<String> = None;
    for line in text.lines() {
        if !line.starts_with('\t') && !line.is_empty() {
            // Flush previous
            if let Some((bdf, vendor, model)) = current.take() {
                gpus.push(build(
                    bdf,
                    vendor,
                    model,
                    driver.take(),
                    iommu_ok,
                    vfio_loaded,
                ));
            }
            // New record — check if it's a display class (0300/0302/0380)
            // Line format: "0000:01:00.0 VGA compatible controller [0300]: NVIDIA ..."
            if let Some((bdf_part, rest)) = line.split_once(' ') {
                // Look for class code in brackets near the first colon
                let lower = rest.to_lowercase();
                let is_display = lower.contains("[0300]")
                    || lower.contains("[0302]")
                    || lower.contains("[0380]");
                if !is_display {
                    continue;
                }
                // Split off class portion: ": <vendor/model>"
                if let Some(after_colon) = rest.split_once(": ").map(|p| p.1) {
                    // after_colon = "NVIDIA Corporation GP107M [GeForce GTX 1050 Ti Mobile] [10de:1c8c] (rev a1)"
                    let (vendor, model) = split_vendor_model(after_colon);
                    current = Some((bdf_part.to_string(), vendor, model));
                }
            }
        } else if current.is_some() {
            let trimmed = line.trim();
            if let Some(v) = trimmed.strip_prefix("Kernel driver in use:") {
                driver = Some(v.trim().to_string());
            }
        }
    }
    if let Some((bdf, vendor, model)) = current.take() {
        gpus.push(build(
            bdf,
            vendor,
            model,
            driver.take(),
            iommu_ok,
            vfio_loaded,
        ));
    }
    mark_primary_displays(&mut gpus);
    gpus
}

fn strip_vendor(before: &str, vendor: &str) -> String {
    // Remove leading "<vendor> Corporation " / "<vendor> Inc " / just "<vendor> "
    let b = before.trim();
    for prefix in [
        format!("{} Corporation ", vendor),
        format!("{} Inc. ", vendor),
        format!("{} Inc ", vendor),
        format!("{} Technology ", vendor),
        format!("{} ", vendor),
    ] {
        if let Some(rest) = b.strip_prefix(&prefix) {
            return rest.trim().to_string();
        }
    }
    b.to_string()
}

fn split_vendor_model(s: &str) -> (String, String) {
    // "NVIDIA Corporation GP107M [GeForce GTX 1050 Ti Mobile] [10de:1c8c] (rev a1)"
    // Take up to first "[" as raw text, then bracketed model if present.
    let s = s.trim();
    if let Some(first_bracket) = s.find('[') {
        let before = s[..first_bracket].trim();
        // Split vendor from the rest — vendor is usually the first 1-2 words.
        // Simple heuristic: take up to " Corporation", " Inc", " Technology" as vendor.
        let vendor = if let Some(pos) = before.find(" Corporation") {
            before[..pos].to_string()
        } else if let Some(pos) = before.find(" Inc") {
            before[..pos].to_string()
        } else if let Some(pos) = before.find(" Technology") {
            before[..pos].to_string()
        } else {
            before.split_whitespace().next().unwrap_or("").to_string()
        };
        // Model: the text inside the first brackets if it's not a vendor:device id,
        // else fall back to the remaining before-bracket text minus vendor.
        let rest = &s[first_bracket..];
        let model = if let Some(end) = rest.find(']') {
            let inside = &rest[1..end];
            if inside.contains(':') && inside.len() <= 12 {
                // it's a vendor:device id like [10de:1c8c] — use the "before" text
                // but strip vendor so we don't end up with "Intel Intel Corp ..."
                strip_vendor(before, &vendor)
            } else {
                inside.to_string()
            }
        } else {
            strip_vendor(before, &vendor)
        };
        (vendor, model)
    } else {
        (
            s.split_whitespace().next().unwrap_or("").to_string(),
            s.to_string(),
        )
    }
}

fn build(
    bdf: String,
    vendor: String,
    model: String,
    driver: Option<String>,
    iommu_ok: bool,
    _vfio_loaded: bool,
) -> GpuInfo {
    let iommu_group = read_iommu_group(&bdf);
    let mut notes = Vec::new();
    if !iommu_ok {
        notes.push(
            "IOMMU desligado no kernel (adicione intel_iommu=on ou amd_iommu=on ao cmdline)."
                .into(),
        );
    }
    let bound_to_vfio = driver.as_deref() == Some("vfio-pci");
    let likely_optimus = super::gpu_bind::is_likely_optimus();
    if likely_optimus && vendor.to_lowercase().contains("nvidia") {
        notes.push(
            "Laptop NVIDIA Optimus detectado: passthrough pode falhar com Code 43 \
             mesmo com tudo configurado. Mitigação Hyper-V já vai no compose."
                .into(),
        );
    }
    GpuInfo {
        bdf,
        vendor,
        model,
        driver,
        iommu_group,
        vfio_ready: bound_to_vfio,
        vfio_capable: iommu_ok,
        likely_optimus,
        is_primary_display: false, // post-processed in `list()`
        vfio_notes: notes,
    }
}

/// True if `bdf` has at least one connected DRM output (i.e. is currently
/// driving a physical display).
fn has_connected_output(bdf: &str) -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return false;
    };
    // First find the cardN whose `device` symlink resolves to this BDF.
    let mut card_name: Option<String> = None;
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(n) = name.to_str() else { continue };
        if !n.starts_with("card") || n.contains('-') {
            continue; // only top-level "cardN", not "cardN-OUT-x"
        }
        let dev_link = format!("/sys/class/drm/{n}/device");
        if let Ok(target) = std::fs::read_link(&dev_link) {
            if target
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s == bdf)
                .unwrap_or(false)
            {
                card_name = Some(n.to_string());
                break;
            }
        }
    }
    let Some(card) = card_name else { return false };

    // Then look for sibling connector dirs cardN-* with status=connected.
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return false;
    };
    let prefix = format!("{card}-");
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(n) = name.to_str() else { continue };
        if !n.starts_with(&prefix) {
            continue;
        }
        let status_path = format!("/sys/class/drm/{n}/status");
        if let Ok(s) = std::fs::read_to_string(&status_path) {
            if s.trim() == "connected" {
                return true;
            }
        }
    }
    false
}

/// Decide which GPUs are currently driving the host display. Marks a GPU as
/// primary when it has a connected output and isn't already on vfio-pci.
/// Falls back to "the lone display GPU" when nothing else qualifies.
fn mark_primary_displays(gpus: &mut [GpuInfo]) {
    if gpus.is_empty() {
        return;
    }
    let mut any_marked = false;
    for g in gpus.iter_mut() {
        if g.driver.as_deref() == Some("vfio-pci") {
            continue;
        }
        if has_connected_output(&g.bdf) {
            g.is_primary_display = true;
            any_marked = true;
        }
    }
    if !any_marked && gpus.len() == 1 {
        gpus[0].is_primary_display = true;
    }
}

fn read_iommu_group(bdf: &str) -> Option<u32> {
    let link = format!("/sys/bus/pci/devices/{}/iommu_group", bdf);
    let target = std::fs::read_link(&link).ok()?;
    target
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
}

fn is_module_loaded(name: &str) -> bool {
    let Ok(text) = std::fs::read_to_string("/proc/modules") else {
        return false;
    };
    let needle = name.replace('-', "_");
    text.lines()
        .any(|l| l.split_whitespace().next().map(|n| n.replace('-', "_")) == Some(needle.clone()))
}

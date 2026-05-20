use serde::Serialize;
use std::path::Path;
use sysinfo::{Disks, System};

#[derive(Debug, Serialize)]
pub struct HostInfo {
    pub ram_gb: u32,
    pub cpu_cores: u32,
    pub free_gb: u32,
}

pub fn info() -> HostInfo {
    // RAM via sysinfo (cross-platform: /proc on Linux, WMI on Windows).
    let mut sys = System::new();
    sys.refresh_memory();
    let ram_gb = bytes_to_gb(sys.total_memory());

    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);

    // Free space on the filesystem that holds the profiles' data dir.
    let free_gb = free_space_gb_for(&super::paths::data_dir());

    HostInfo {
        ram_gb,
        cpu_cores,
        free_gb,
    }
}

fn bytes_to_gb(bytes: u64) -> u32 {
    (bytes / 1024 / 1024 / 1024) as u32
}

/// Available space (GB) on the disk whose mount point is the longest
/// prefix of `path`. Cross-platform via sysinfo. Returns 0 if no disk
/// matches (degrades gracefully instead of erroring).
fn free_space_gb_for(path: &Path) -> u32 {
    let disks = Disks::new_with_refreshed_list();
    let mut best: Option<(usize, u64)> = None;
    for d in disks.list() {
        let mp = d.mount_point();
        if path.starts_with(mp) {
            let len = mp.as_os_str().len();
            if best.map(|(l, _)| len > l).unwrap_or(true) {
                best = Some((len, d.available_space()));
            }
        }
    }
    best.map(|(_, b)| bytes_to_gb(b)).unwrap_or(0)
}

/// Host timezone in IANA form (e.g. "America/Sao_Paulo"), passed to the
/// guest. Linux reads it from `timedatectl`; on other platforms we fall
/// back to UTC (the user can override per-profile locale).
pub fn tz() -> String {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("timedatectl")
            .args(["show", "-p", "Timezone", "--value"])
            .output()
            .ok()
            .and_then(|o| {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            })
            .unwrap_or_else(|| "UTC".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        "UTC".to_string()
    }
}

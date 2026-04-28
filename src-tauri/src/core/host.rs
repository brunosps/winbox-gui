use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct HostInfo {
    pub ram_gb: u32,
    pub cpu_cores: u32,
    pub free_gb: u32,
}

pub fn info() -> HostInfo {
    let ram_gb = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemTotal:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        })
        .map(|kb| (kb / 1024 / 1024) as u32)
        .unwrap_or(0);

    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1);

    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let free_gb = Command::new("df")
        .args(["-BG", &home])
        .output()
        .ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            s.lines().nth(1).map(|l| {
                l.split_whitespace()
                    .nth(3)
                    .unwrap_or("0G")
                    .trim_end_matches('G')
                    .parse::<u32>()
                    .unwrap_or(0)
            })
        })
        .unwrap_or(0);

    HostInfo {
        ram_gb,
        cpu_cores,
        free_gb,
    }
}

pub fn tz() -> String {
    // timedatectl show -p Timezone --value
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

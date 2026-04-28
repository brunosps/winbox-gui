use anyhow::{bail, Context, Result};
use chrono::Local;
use serde::Serialize;
use std::process::Command;
use std::time::UNIX_EPOCH;

use super::docker;
use super::paths;
use super::validation;

#[derive(Debug, Serialize, Clone)]
pub struct SnapshotInfo {
    pub name: String,
    pub created: String,
    pub size: String,
}

pub fn list(profile: &str) -> Vec<SnapshotInfo> {
    let dir = paths::profile_snapshots_dir(profile);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return out;
    };
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let Some(name) = e.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let created = e
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| {
                let secs = d.as_secs() as i64;
                chrono::DateTime::<Local>::from(
                    std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64),
                )
                .format("%Y-%m-%d %H:%M")
                .to_string()
            })
            .unwrap_or_default();
        let size = du_h(&e.path()).unwrap_or_else(|| "?".into());
        out.push(SnapshotInfo {
            name,
            created,
            size,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn du_h(p: &std::path::Path) -> Option<String> {
    let out = Command::new("du").args(["-sh"]).arg(p).output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.split_whitespace().next().map(str::to_string)
}

/// Stops the container if it is running, returning whether it was.
fn stop_if_running(profile: &str) -> bool {
    let c = paths::profile_container(profile);
    let was = matches!(docker::container_status(&c).as_str(), "running" | "paused");
    if was {
        let _ = docker::compose_run(profile, &["down"]);
    }
    was
}

fn restart_if(profile: &str, was_up: bool) {
    if was_up {
        let _ = docker::compose_run(profile, &["up", "-d"]);
    }
}

pub fn create(profile: &str, snap_name: &str) -> Result<()> {
    validation::validate_snapshot_name(snap_name)?;
    let dst = paths::profile_snapshots_dir(profile).join(snap_name);
    if dst.exists() {
        bail!("Snapshot '{}' já existe em '{}'.", snap_name, profile);
    }
    let was = stop_if_running(profile);
    std::fs::create_dir_all(&dst)?;
    let src = paths::profile_storage_dir(profile);
    let ok = Command::new("cp")
        .args(["--sparse=always", "-a"])
        .arg(format!("{}/.", src.display()))
        .arg(&dst)
        .status()
        .with_context(|| format!("cp {} → {}", src.display(), dst.display()))?
        .success();
    restart_if(profile, was);
    if !ok {
        bail!("cp snapshot falhou");
    }
    Ok(())
}

pub fn rollback(profile: &str, snap_name: &str) -> Result<()> {
    validation::validate_snapshot_name(snap_name)?;
    let src = paths::profile_snapshots_dir(profile).join(snap_name);
    if !src.is_dir() {
        bail!("Snapshot '{}' não encontrado em '{}'.", snap_name, profile);
    }
    let was = stop_if_running(profile);
    let storage = paths::profile_storage_dir(profile);
    let _ = std::fs::remove_dir_all(&storage);
    std::fs::create_dir_all(&storage)?;
    let ok = Command::new("cp")
        .args(["--sparse=always", "-a"])
        .arg(format!("{}/.", src.display()))
        .arg(&storage)
        .status()?
        .success();
    restart_if(profile, was);
    if !ok {
        bail!("cp rollback falhou");
    }
    Ok(())
}

pub fn timestamped_snapshot_name(prefix: &str) -> String {
    format!("{}-{}", prefix, Local::now().format("%Y%m%d-%H%M%S"))
}

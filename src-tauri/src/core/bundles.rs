use anyhow::{anyhow, Result};
use include_dir::{include_dir, Dir};
use serde::Serialize;

use super::{paths, validation};

static BUILTIN: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/bundles-builtin");

#[derive(Debug, Serialize, Clone)]
pub struct Bundle {
    pub name: String,
    pub custom: bool,
}

pub fn list() -> Vec<Bundle> {
    let mut out: Vec<Bundle> = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = Default::default();

    for entry in BUILTIN.files() {
        if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
            if validation::validate_bundle_name(name).is_ok() && seen.insert(name.to_string()) {
                out.push(Bundle {
                    name: name.to_string(),
                    custom: false,
                });
            }
        }
    }

    let user_dir = paths::bundles_user_dir();
    if let Ok(rd) = std::fs::read_dir(&user_dir) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ps1") {
                continue;
            }
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                if validation::validate_bundle_name(name).is_err() {
                    continue;
                }
                // user bundles override builtin
                if let Some(existing) = out.iter_mut().find(|b| b.name == name) {
                    existing.custom = true;
                } else {
                    out.push(Bundle {
                        name: name.to_string(),
                        custom: true,
                    });
                }
                seen.insert(name.to_string());
            }
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Return PS1 source for the given bundle. User override wins.
pub fn resolve(name: &str) -> Result<String> {
    validation::validate_bundle_name(name)?;
    let user_path = paths::bundles_user_dir().join(format!("{}.ps1", name));
    if user_path.is_file() {
        return Ok(std::fs::read_to_string(&user_path)?);
    }
    if let Some(file) = BUILTIN.get_file(format!("{}.ps1", name)) {
        return Ok(file.contents_utf8().unwrap_or_default().to_string());
    }
    Err(anyhow!("bundle '{}' not found", name))
}

use anyhow::{anyhow, bail, Context, Result};

use super::{env_file, paths, validation};

pub fn list_names() -> Vec<String> {
    let dir = paths::profiles_cfg_dir();
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(n) = e.file_name().to_str() {
                    out.push(n.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

pub fn get_default() -> Option<String> {
    let f = paths::default_file();
    std::fs::read_to_string(&f)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn set_default(name: &str) -> Result<()> {
    let f = paths::default_file();
    if let Some(parent) = f.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&f, format!("{}\n", name))
        .with_context(|| format!("writing {}", f.display()))?;
    Ok(())
}

pub fn clear_default() -> Result<()> {
    let f = paths::default_file();
    let _ = std::fs::remove_file(f);
    Ok(())
}

pub fn exists(name: &str) -> bool {
    paths::profile_cfg_dir(name).is_dir()
}

/// Resolve an optional profile arg to a concrete name:
/// - given explicit name, check it exists
/// - empty → default profile, or the sole profile if only one exists
pub fn resolve(arg: Option<&str>) -> Result<String> {
    match arg.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => {
            if !exists(name) {
                bail!("Perfil '{}' não existe.", name);
            }
            Ok(name.to_string())
        }
        None => {
            if let Some(def) = get_default() {
                if exists(&def) {
                    return Ok(def);
                }
            }
            let all = list_names();
            match all.len() {
                0 => Err(anyhow!("Nenhum perfil. Rode: winbox install <nome>")),
                1 => Ok(all.into_iter().next().unwrap()),
                _ => Err(anyhow!(
                    "Múltiplos perfis — especifique o nome ou rode 'winbox default <nome>'"
                )),
            }
        }
    }
}

pub fn validate_name(name: &str) -> Result<()> {
    validation::validate_profile_name(name)
}

/// Full paths for a profile (useful for removal).
pub fn all_paths(name: &str) -> Vec<std::path::PathBuf> {
    vec![paths::profile_cfg_dir(name), paths::profile_data_dir(name)]
}

pub fn remove_tree(name: &str) -> Result<()> {
    for p in all_paths(name) {
        if p.exists() {
            std::fs::remove_dir_all(&p).with_context(|| format!("rm -rf {}", p.display()))?;
        }
    }
    Ok(())
}

pub fn config_env_path(name: &str) -> std::path::PathBuf {
    paths::profile_env_file(name)
}

pub fn read_config_env(name: &str) -> Result<env_file::EnvMap> {
    env_file::read(&config_env_path(name))
}

pub fn read_office_config(name: &str) -> Result<validation::OfficeEnvConfig> {
    let map = read_config_env(name)?;
    let config = validation::office_env_config_from_map(&map);
    validation::validate_office_config(&config)?;
    Ok(config)
}

pub fn set_config_key(name: &str, key: &str, value: &str) -> Result<()> {
    validation::validate_env_value(key, value)?;
    env_file::set_key(&config_env_path(name), key, value)
}

pub fn ensure_dirs(name: &str) -> Result<()> {
    for p in [
        paths::profile_cfg_dir(name),
        paths::profile_storage_dir(name),
        paths::profile_snapshots_dir(name),
        paths::profile_shared_dir(name),
        paths::profile_oem_dir(name),
    ] {
        std::fs::create_dir_all(&p).with_context(|| format!("mkdir {}", p.display()))?;
    }
    Ok(())
}

pub fn is_env_present(name: &str) -> bool {
    paths::profile_env_file(name).is_file()
}

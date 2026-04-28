use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub type EnvMap = BTreeMap<String, String>;

pub fn read(path: &Path) -> Result<EnvMap> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(parse(&content))
}

pub fn parse(content: &str) -> EnvMap {
    let mut map = EnvMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.to_string());
        }
    }
    map
}

/// Update a single KEY= line in a config.env, preserving format and order.
/// If KEY does not exist, append it.
pub fn set_key(path: &Path, key: &str, value: &str) -> Result<()> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut out = String::with_capacity(content.len() + key.len() + value.len() + 2);
    let mut found = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(eq) = trimmed.find('=') {
            if trimmed[..eq].trim() == key {
                out.push_str(key);
                out.push('=');
                out.push_str(value);
                out.push('\n');
                found = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !found {
        out.push_str(key);
        out.push('=');
        out.push_str(value);
        out.push('\n');
    }
    std::fs::write(path, out).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn get<'a>(map: &'a EnvMap, key: &str) -> &'a str {
    map.get(key).map(String::as_str).unwrap_or("")
}

pub fn get_u16(map: &EnvMap, key: &str) -> u16 {
    get(map, key).parse().unwrap_or(0)
}

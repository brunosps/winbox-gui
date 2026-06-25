use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortForward {
    pub host: u16,
    pub container: u16,
    pub proto: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySpec {
    pub env_value: String,
    pub gib: u32,
}

pub fn validate_profile_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Nome do perfil vazio.");
    }
    let Some(first) = name.chars().next() else {
        bail!("Nome do perfil vazio.");
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        bail!("Nome inválido '{}' — deve começar com [a-z0-9]", name);
    }
    for c in name.chars() {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            bail!("Nome inválido '{}' — use apenas [a-z0-9_-]", name);
        }
    }
    Ok(())
}

pub fn validate_env_value(label: &str, value: &str) -> Result<()> {
    if value.contains('\n') || value.contains('\r') || value.contains('\0') {
        bail!("{label} contém caracteres inválidos.");
    }
    Ok(())
}

pub fn require_non_empty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} é obrigatório.");
    }
    Ok(())
}

pub fn validate_password(value: &str, required: bool) -> Result<()> {
    if required {
        require_non_empty("Senha", value)?;
    }
    validate_env_value("Senha", value)?;
    if value.chars().any(|c| c.is_control()) {
        bail!("Senha contém caracteres de controle inválidos.");
    }
    Ok(())
}

pub fn normalize_ram(value: &str) -> Result<MemorySpec> {
    let gib = parse_gib(value, "RAM", 1, 1024)?;
    Ok(MemorySpec {
        env_value: format!("{gib}G"),
        gib,
    })
}

pub fn normalize_disk(value: &str) -> Result<String> {
    let gib = parse_gib(value, "Disco", 8, 8192)?;
    Ok(format!("{gib}G"))
}

pub fn normalize_cpu(value: &str) -> Result<String> {
    let v = value.trim();
    require_non_empty("CPU", v)?;
    if !v.chars().all(|c| c.is_ascii_digit()) {
        bail!("CPU inválida '{value}' — use um número inteiro.");
    }
    let n: u16 = v
        .parse()
        .map_err(|_| anyhow::anyhow!("CPU inválida '{value}'."))?;
    if !(1..=512).contains(&n) {
        bail!("CPU inválida '{value}' — use um valor entre 1 e 512.");
    }
    Ok(n.to_string())
}

fn parse_gib(value: &str, label: &str, min: u32, max: u32) -> Result<u32> {
    let v = value.trim();
    require_non_empty(label, v)?;
    let raw = v.trim_end_matches(['G', 'g']);
    if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit()) {
        bail!("{label} inválido '{value}' — use formato como 8G.");
    }
    let n: u32 = raw
        .parse()
        .map_err(|_| anyhow::anyhow!("{label} inválido '{value}'."))?;
    if !(min..=max).contains(&n) {
        bail!("{label} inválido '{value}' — use um valor entre {min}G e {max}G.");
    }
    Ok(n)
}

/// Outcome of a successful [`validate_storage_path`] call. Carries
/// non-fatal warnings the UI may want to surface (e.g. WSL drvfs path).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct StoragePathCheck {
    /// Soft advisory the UI should show as a yellow note. `None` means
    /// the path is unremarkable.
    pub warning: Option<String>,
}

/// Validate a user-supplied custom storage directory for a VM profile.
///
/// Rules:
///
/// * Empty input → `Ok(None)` (caller falls back to the default path).
/// * Absolute path required; relative paths are rejected.
/// * **Paths under `/mnt/<drive>/` (WSL drvfs) are REJECTED.** drvfs/9p
///   delivers ~104 MB/s vs. ~3 GB/s on the distro's native ext4, and the
///   slow I/O makes Windows installs take hours and stall — incompatible
///   with VM disk images. The user must pick a path inside the distro
///   (e.g. `~/winbox-disks/...`), which still lands on the same physical
///   drive (the ext4.vhdx) but via fast native I/O.
/// * Directory is created if missing (parent must exist + be writable).
/// * Write permission is probed with a tiny temp file.
/// * Free space must be at least `disk_size_env` (e.g. `"128G"`).
pub fn validate_storage_path(value: &str, disk_size_env: &str) -> Result<Option<StoragePathCheck>> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(None);
    }
    validate_env_value("Local de armazenamento", v)?;
    let path = Path::new(v);
    if !path.is_absolute() {
        bail!("Local de armazenamento deve usar caminho absoluto: '{v}'");
    }

    // Hard reject drvfs mounts — they're too slow for VM disk images and
    // cause installs to stall. Done before any filesystem work so we never
    // create a directory on an incompatible mount.
    if is_wsl_drvfs(path) {
        bail!(
            "'{v}' está em /mnt/ (drvfs do Windows), que é lento demais para discos de VM \
             (~104 MB/s vs. ~3 GB/s no disco da distro). Escolha um caminho dentro do WSL, \
             como /home/bruno/winbox-disks — ele grava no mesmo disco físico, mas rápido."
        );
    }

    let need_gib = parse_gib(disk_size_env, "Disco", 1, 8192)?;

    if path.exists() {
        if !path.is_dir() {
            bail!("'{v}' existe mas não é diretório.");
        }
    } else {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Não foi possível criar diretório '{v}'"))?;
    }

    let probe = path.join(".winbox-write-test");
    std::fs::write(&probe, b"ok").with_context(|| format!("Sem permissão de escrita em '{v}'"))?;
    let _ = std::fs::remove_file(&probe);

    if let Some(free_bytes) = query_free_bytes(path) {
        let need_bytes: u64 = (need_gib as u64) * 1024 * 1024 * 1024;
        if free_bytes < need_bytes {
            let free_gib = free_bytes / (1024 * 1024 * 1024);
            bail!("Espaço insuficiente em '{v}': preciso de {need_gib}G, disponível {free_gib}G.");
        }
    }

    Ok(Some(StoragePathCheck { warning: None }))
}

/// Available bytes on the filesystem hosting `path`, via `df`. Returns
/// `None` if df is missing or the output cannot be parsed — callers
/// should treat that as "unknown" and skip the space check rather than
/// fail closed.
fn query_free_bytes(path: &Path) -> Option<u64> {
    let out = Command::new("df")
        .args(["--output=avail", "-B", "1"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines().nth(1)?.trim().parse().ok()
}

/// Heuristic: paths shaped like `/mnt/<letter>/...` (one ASCII char
/// after `/mnt/`) are almost always WSL drvfs mounts of Windows drives.
fn is_wsl_drvfs(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if !s.starts_with("/mnt/") {
        return false;
    }
    let rest = &s["/mnt/".len()..];
    let mut chars = rest.chars();
    let first = chars.next();
    let second = chars.next();
    matches!(first, Some(c) if c.is_ascii_alphabetic()) && matches!(second, Some('/') | None)
}

pub fn validate_iso_path(value: &str) -> Result<()> {
    let v = value.trim();
    require_non_empty("ISO", v)?;
    validate_env_value("ISO", v)?;
    let path = Path::new(v);
    if !path.is_absolute() {
        bail!("ISO deve usar caminho absoluto.");
    }
    if !path.is_file() {
        bail!("ISO não encontrada em '{}'.", v);
    }
    Ok(())
}

pub fn validate_bdf(bdf: &str) -> Result<()> {
    let parts: Vec<&str> = bdf.split([':', '.']).collect();
    if parts.len() != 4
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
        || parts[3].len() != 1
        || !bdf
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
    {
        bail!("BDF inválido: {bdf:?}");
    }
    Ok(())
}

pub fn validate_bundle_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("Bundle inválido '{name}' — use apenas [A-Za-z0-9_-].");
    }
    Ok(())
}

pub fn validate_snapshot_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        || name == "."
        || name == ".."
        || name.contains("..")
    {
        bail!("Snapshot inválido '{name}' — use apenas [A-Za-z0-9_.-].");
    }
    Ok(())
}

pub fn parse_bundle_csv(csv: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for raw in csv.split(',') {
        let name: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        if name.is_empty() {
            continue;
        }
        validate_bundle_name(&name)?;
        if seen.insert(name.clone()) {
            out.push(name);
        }
    }
    Ok(out)
}

pub fn parse_extra_ports(spec: &str) -> Result<Vec<PortForward>> {
    let mut out = Vec::new();
    if spec.trim().is_empty() {
        return Ok(out);
    }
    for raw in spec.split(',') {
        let part: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
        if part.is_empty() {
            continue;
        }
        let Some((host, rest)) = part.split_once(':') else {
            bail!("Porta extra inválida '{raw}' — use host:container[/tcp|udp].");
        };
        let (container, proto) = rest.split_once('/').unwrap_or((rest, "tcp"));
        let host = parse_port(host, "host", raw)?;
        let container = parse_port(container, "container", raw)?;
        let proto = proto.to_ascii_lowercase();
        if proto != "tcp" && proto != "udp" {
            bail!("Protocolo inválido em '{raw}' — use tcp ou udp.");
        }
        out.push(PortForward {
            host,
            container,
            proto,
        });
    }
    Ok(out)
}

pub fn format_extra_ports(ports: &[PortForward]) -> String {
    ports
        .iter()
        .map(|p| format!("{}:{}/{}", p.host, p.container, p.proto))
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_port(value: &str, label: &str, raw: &str) -> Result<u16> {
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
        bail!("Porta {label} inválida em '{raw}'.");
    }
    let port: u16 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("Porta {label} inválida em '{raw}'."))?;
    if port == 0 {
        bail!("Porta {label} inválida em '{raw}' — use 1..65535.");
    }
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extra_ports() {
        let ports = parse_extra_ports("8080:80, 5353:53/udp").unwrap();
        assert_eq!(
            ports,
            vec![
                PortForward {
                    host: 8080,
                    container: 80,
                    proto: "tcp".into(),
                },
                PortForward {
                    host: 5353,
                    container: 53,
                    proto: "udp".into(),
                },
            ]
        );
        assert_eq!(format_extra_ports(&ports), "8080:80/tcp,5353:53/udp");
    }

    #[test]
    fn rejects_bad_extra_ports() {
        assert!(parse_extra_ports("8080").is_err());
        assert!(parse_extra_ports("8080:80/http").is_err());
        assert!(parse_extra_ports("0:80").is_err());
    }

    #[test]
    fn normalizes_resources() {
        assert_eq!(normalize_ram("8g").unwrap().env_value, "8G");
        assert_eq!(normalize_disk("128").unwrap(), "128G");
        assert_eq!(normalize_cpu("4").unwrap(), "4");
        assert!(normalize_ram("0G").is_err());
        assert!(normalize_cpu("0").is_err());
    }

    #[test]
    fn validates_names_and_bdf() {
        assert!(validate_profile_name("banking_1").is_ok());
        assert!(validate_profile_name("../bad").is_err());
        assert!(validate_bdf("0000:01:00.0").is_ok());
        assert!(validate_bdf("0000:01:00").is_err());
    }

    #[test]
    fn storage_path_empty_returns_none() {
        let out = validate_storage_path("", "128G").unwrap();
        assert!(out.is_none());
        let out = validate_storage_path("   ", "128G").unwrap();
        assert!(out.is_none());
    }

    #[test]
    fn storage_path_rejects_relative() {
        let err = validate_storage_path("foo/bar", "8G").unwrap_err();
        assert!(format!("{err}").contains("absoluto"));
    }

    #[test]
    fn storage_path_creates_missing_dir() {
        let tmp = std::env::temp_dir().join(format!("winbox-test-storage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let out = validate_storage_path(tmp.to_str().unwrap(), "1G").unwrap();
        assert!(out.is_some(), "expected Some(check), got {out:?}");
        assert!(tmp.exists() && tmp.is_dir());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn storage_path_drvfs_heuristic() {
        // Doesn't need to actually exist — we only exercise the heuristic.
        assert!(is_wsl_drvfs(Path::new("/mnt/c/Users/foo")));
        assert!(is_wsl_drvfs(Path::new("/mnt/e/disks")));
        assert!(is_wsl_drvfs(Path::new("/mnt/c"))); // bare letter, no trailing slash
        assert!(!is_wsl_drvfs(Path::new("/home/bruno/storage")));
        assert!(!is_wsl_drvfs(Path::new("/mnt/wsl/instances"))); // multi-letter -> not a drive
    }

    #[test]
    fn storage_path_rejects_drvfs_mount() {
        // drvfs paths must be rejected outright (not just warned) — they're
        // too slow for VM disks. Error fires before any fs work, so a
        // non-existent /mnt/e path still errors deterministically.
        let err = validate_storage_path("/mnt/e/winbox", "128G").unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("drvfs"),
            "expected drvfs rejection, got: {msg}"
        );
    }
}

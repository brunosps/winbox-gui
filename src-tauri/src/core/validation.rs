use anyhow::{bail, Result};
use serde::Serialize;
use std::path::Path;

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
}

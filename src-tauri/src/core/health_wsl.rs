//! WSL2-aware host health probes — A5 of the backend roadmap.
//!
//! On a Windows host the existing `core::health` checks (which look at
//! `/dev/kvm`, the docker UNIX socket, etc.) are meaningless — the
//! interesting state lives in `wsl.exe --status`, `wsl.exe --list
//! --verbose`, and `%USERPROFILE%\.wslconfig`. This module exposes
//! pure parsers + a thin `cfg(target_os = "windows")` invoker so the
//! parsing logic is testable on Linux while the Windows binary picks
//! up the real data.
//!
//! Each parser is deliberately tolerant: WSL output uses UTF-16-with-BOM
//! on older Windows versions; `.wslconfig` is an INI variant that
//! accepts `=`/spaces freely. We accept what works and ignore what
//! doesn't.

use serde::Serialize;

/// Subset of `wsl.exe --list --verbose` output we care about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WslDistro {
    pub name: String,
    pub state: WslState,
    pub version: u8,
    pub is_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WslState {
    Running,
    Stopped,
    Installing,
    Other,
}

impl WslState {
    fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "installing" => Self::Installing,
            _ => Self::Other,
        }
    }
}

/// Parse the output of `wsl.exe --list --verbose`. Tolerates the
/// UTF-16-with-null-bytes encoding that older `wsl.exe` versions emit:
/// callers should normalize to plain UTF-8 first (strip BOM + null
/// bytes between ASCII chars) — `decode_wsl_output` below does that.
pub fn parse_wsl_list_verbose(text: &str) -> Vec<WslDistro> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        // Skip blank / pure-separator lines.
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            continue;
        }
        // First non-blank, non-asterisk-only column starts the row.
        // Columns are space-aligned: optional leading `*`, NAME, STATE, VERSION.
        let rest = trimmed.trim_start();
        let is_default = rest.starts_with('*');
        let body = rest.trim_start_matches('*').trim_start();
        let parts: Vec<&str> = body.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].to_string();
        let state = WslState::from_str(parts[1]);
        let version: u8 = parts[2].parse().unwrap_or(0);
        out.push(WslDistro {
            name,
            state,
            version,
            is_default,
        });
    }
    out
}

/// Convert raw bytes from `wsl.exe` into a usable UTF-8 string. Older
/// `wsl.exe` (pre WSL_UTF8=1) emits UTF-16 LE with a BOM; newer ones
/// emit UTF-8. We detect the BOM and strip embedded null bytes from
/// the UTF-16 case rather than pulling in a full decoder.
pub fn decode_wsl_output(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16 LE BOM: strip BOM, then drop every other byte
        // (best-effort ASCII-down — WSL output is always ASCII for
        // names/states).
        let body = &bytes[2..];
        let mut out = Vec::with_capacity(body.len() / 2);
        for chunk in body.chunks(2) {
            if !chunk.is_empty() && chunk[0] != 0 {
                out.push(chunk[0]);
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    } else {
        // Strip a UTF-8 BOM if present.
        let body = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &bytes[3..]
        } else {
            bytes
        };
        String::from_utf8_lossy(body).into_owned()
    }
}

/// Parsed view of `~\.wslconfig` (a Windows INI variant — sectioned,
/// `key = value` pairs, `#` comments). Only the keys we care about.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct WslConfig {
    /// `[wsl2] vmIdleTimeout=N` — milliseconds before WSL shuts the VM
    /// down after the last session closes. `-1` disables it. None
    /// means "unset" (default applies: 60000ms in current WSL builds).
    pub vm_idle_timeout_ms: Option<i64>,
    /// `[wsl2] memory=8GB` — parsed to bytes. None means unset.
    pub memory_bytes: Option<u64>,
    /// `[wsl2] processors=N`. None means unset.
    pub processors: Option<u32>,
    /// `[wsl2] nestedVirtualization=true`. None means unset.
    pub nested_virt: Option<bool>,
    /// `[wsl2] networkingMode=mirrored` — Windows 11 22H2+ feature.
    pub networking_mode: Option<String>,
    /// `[wsl2] swap=8GB`.
    pub swap_bytes: Option<u64>,
}

/// Parse a `~/.wslconfig` body. Accepts inline comments after `#` and
/// `;`. Trailing whitespace and unknown keys are ignored. Returns
/// `WslConfig::default()` for unparseable input rather than erroring —
/// the file is optional from WSL's perspective too.
pub fn parse_wslconfig_ini(text: &str) -> WslConfig {
    let mut cfg = WslConfig::default();
    let mut in_wsl2 = false;
    for raw in text.lines() {
        // Strip comments (`#` first, then `;`). Each step must feed
        // the previous step's output back in — using `unwrap_or(raw)`
        // would silently re-inject the original line.
        let after_hash = raw.split_once('#').map(|(a, _)| a).unwrap_or(raw);
        let after_semi = after_hash
            .split_once(';')
            .map(|(a, _)| a)
            .unwrap_or(after_hash);
        let line = after_semi.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_wsl2 = line.eq_ignore_ascii_case("[wsl2]");
            continue;
        }
        if !in_wsl2 {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim().trim_matches('"');
        match key.as_str() {
            "vmidletimeout" => {
                cfg.vm_idle_timeout_ms = value.parse::<i64>().ok();
            }
            "memory" => cfg.memory_bytes = parse_size_suffix(value),
            "swap" => cfg.swap_bytes = parse_size_suffix(value),
            "processors" => cfg.processors = value.parse::<u32>().ok(),
            "nestedvirtualization" => cfg.nested_virt = parse_bool(value),
            "networkingmode" => cfg.networking_mode = Some(value.to_string()),
            _ => {}
        }
    }
    cfg
}

/// Accept `8GB`, `8G`, `8192MB`, `8192m`, `512K`, `1024` (treated as bytes).
fn parse_size_suffix(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (digits, suffix): (String, String) =
        s.chars().partition(|c| c.is_ascii_digit() || *c == '_');
    let n: u64 = digits.replace('_', "").parse().ok()?;
    let mult: u64 = match suffix.trim().to_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" => 1024,
        "M" | "MB" => 1024 * 1024,
        "G" | "GB" => 1024 * 1024 * 1024,
        "T" | "TB" => 1024_u64.pow(4),
        _ => return None,
    };
    n.checked_mul(mult)
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

/// Findings the UI can render. Each item is "what's wrong + why it
/// matters + what to do" — UI side just formats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WslFinding {
    pub id: String,
    pub severity: WslSeverity,
    pub message: String,
    pub hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WslSeverity {
    Ok,
    Warn,
    Error,
}

/// Inspect the parsed config + distro list and return any findings.
/// Pure function — easy to drive from tests on Linux.
pub fn evaluate(distros: &[WslDistro], cfg: &WslConfig, target_distro: &str) -> Vec<WslFinding> {
    let mut out = Vec::new();

    // 1. Does the target distro exist?
    let target = distros.iter().find(|d| d.name == target_distro);
    match target {
        None => out.push(WslFinding {
            id: "wsl_distro_missing".into(),
            severity: WslSeverity::Error,
            message: format!("distro '{target_distro}' não está registrada"),
            hint: format!(
                "execute: wsl --install -d Ubuntu-24.04 --location E:\\WSL\\{target_distro}"
            ),
        }),
        Some(d) if d.version != 2 => out.push(WslFinding {
            id: "wsl_distro_v1".into(),
            severity: WslSeverity::Error,
            message: format!("distro '{target_distro}' está em WSL v1, precisa de v2"),
            hint: format!("execute: wsl --set-version {target_distro} 2"),
        }),
        Some(d) if d.state != WslState::Running => out.push(WslFinding {
            id: "wsl_distro_stopped".into(),
            severity: WslSeverity::Warn,
            message: format!("distro '{target_distro}' parada"),
            hint: format!("acorde-a: wsl -d {target_distro} -- true"),
        }),
        Some(_) => out.push(WslFinding {
            id: "wsl_distro_ok".into(),
            severity: WslSeverity::Ok,
            message: format!("distro '{target_distro}' rodando em WSL2"),
            hint: String::new(),
        }),
    }

    // 2. vmIdleTimeout — explicit -1 (never shut down) is what we want
    // for the autostart-plus-containers usage pattern.
    match cfg.vm_idle_timeout_ms {
        Some(-1) => out.push(WslFinding {
            id: "wsl_idle_timeout_disabled".into(),
            severity: WslSeverity::Ok,
            message: "vmIdleTimeout = -1 (distro nunca desliga por inatividade)".into(),
            hint: String::new(),
        }),
        Some(n) if n < 300_000 => out.push(WslFinding {
            id: "wsl_idle_timeout_low".into(),
            severity: WslSeverity::Warn,
            message: format!(
                "vmIdleTimeout = {n}ms — distro vai desligar em menos de 5min sem comandos"
            ),
            hint: "edite C:\\Users\\<user>\\.wslconfig: [wsl2] vmIdleTimeout=-1".into(),
        }),
        Some(_) => {} // High but finite is acceptable.
        None => out.push(WslFinding {
            id: "wsl_idle_timeout_default".into(),
            severity: WslSeverity::Warn,
            message: "vmIdleTimeout não configurado — distro desliga em ~60s sem comandos".into(),
            hint: "edite C:\\Users\\<user>\\.wslconfig: [wsl2] vmIdleTimeout=-1".into(),
        }),
    }

    // 3. Nested virt — required by dockurr/QEMU inside the distro.
    match cfg.nested_virt {
        Some(true) => out.push(WslFinding {
            id: "wsl_nested_virt_ok".into(),
            severity: WslSeverity::Ok,
            message: "nestedVirtualization habilitado".into(),
            hint: String::new(),
        }),
        Some(false) => out.push(WslFinding {
            id: "wsl_nested_virt_off".into(),
            severity: WslSeverity::Error,
            message: "nestedVirtualization=false — QEMU/dockurr não conseguem acelerar".into(),
            hint: "edite .wslconfig: [wsl2] nestedVirtualization=true".into(),
        }),
        None => out.push(WslFinding {
            id: "wsl_nested_virt_default".into(),
            severity: WslSeverity::Warn,
            message: "nestedVirtualization não configurado".into(),
            hint: "adicione em .wslconfig: [wsl2] nestedVirtualization=true".into(),
        }),
    }

    // 4. Networking mode — mirrored é melhor pra RDP/web direto em IP host.
    if cfg
        .networking_mode
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("mirrored"))
        .unwrap_or(false)
    {
        out.push(WslFinding {
            id: "wsl_networking_mirrored".into(),
            severity: WslSeverity::Ok,
            message: "networkingMode=mirrored — VMs visíveis no IP do host".into(),
            hint: String::new(),
        });
    }

    // 5. Memória reservada — abaixo de 4G é apertado pra Win11 guest.
    if let Some(b) = cfg.memory_bytes {
        if b < 4 * 1024 * 1024 * 1024 {
            out.push(WslFinding {
                id: "wsl_memory_low".into(),
                severity: WslSeverity::Warn,
                message: format!(
                    "memory={}MB no .wslconfig — pouco para guest Windows",
                    b / (1024 * 1024)
                ),
                hint: "considere [wsl2] memory=16GB".into(),
            });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wslconfig_reads_known_keys() {
        let body = r#"
# WSL2 config
[wsl2]
nestedVirtualization=true
vmIdleTimeout=-1
memory=16GB
processors=8
swap=8GB
networkingMode=mirrored
"#;
        let cfg = parse_wslconfig_ini(body);
        assert_eq!(cfg.nested_virt, Some(true));
        assert_eq!(cfg.vm_idle_timeout_ms, Some(-1));
        assert_eq!(cfg.memory_bytes, Some(16 * 1024 * 1024 * 1024));
        assert_eq!(cfg.processors, Some(8));
        assert_eq!(cfg.swap_bytes, Some(8 * 1024 * 1024 * 1024));
        assert_eq!(cfg.networking_mode.as_deref(), Some("mirrored"));
    }

    #[test]
    fn parse_wslconfig_ignores_unknown_sections() {
        let body = "[experimental]\nfoo=bar\n[wsl2]\nmemory=4GB\n";
        let cfg = parse_wslconfig_ini(body);
        assert_eq!(cfg.memory_bytes, Some(4 * 1024 * 1024 * 1024));
    }

    #[test]
    fn parse_wslconfig_handles_inline_comments() {
        let body = "[wsl2]\nmemory=8GB # bumped from 4G\n";
        let cfg = parse_wslconfig_ini(body);
        assert_eq!(cfg.memory_bytes, Some(8 * 1024 * 1024 * 1024));
    }

    #[test]
    fn parse_size_suffix_accepts_common_units() {
        assert_eq!(parse_size_suffix("1024"), Some(1024));
        assert_eq!(parse_size_suffix("1KB"), Some(1024));
        assert_eq!(parse_size_suffix("1MB"), Some(1024 * 1024));
        assert_eq!(parse_size_suffix("1G"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size_suffix("1TB"), Some(1024_u64.pow(4)));
        assert_eq!(parse_size_suffix("bad"), None);
    }

    #[test]
    fn parse_wsl_list_verbose_reads_running_distro() {
        let body = "  NAME            STATE           VERSION\n\
                    * Ubuntu-24.04   Running         2\n  \
                    Ubuntu-22.04    Stopped         2\n";
        let parsed = parse_wsl_list_verbose(body);
        assert_eq!(parsed.len(), 2);
        let ub24 = &parsed[0];
        assert_eq!(ub24.name, "Ubuntu-24.04");
        assert_eq!(ub24.state, WslState::Running);
        assert_eq!(ub24.version, 2);
        assert!(ub24.is_default);
        let ub22 = &parsed[1];
        assert_eq!(ub22.name, "Ubuntu-22.04");
        assert_eq!(ub22.state, WslState::Stopped);
        assert!(!ub22.is_default);
    }

    #[test]
    fn decode_wsl_output_handles_utf16_with_bom() {
        // Encode "OK\n" as UTF-16 LE with BOM.
        let mut bytes = vec![0xFF, 0xFE];
        for ch in "OK\n".chars() {
            bytes.push(ch as u8);
            bytes.push(0);
        }
        let decoded = decode_wsl_output(&bytes);
        assert_eq!(decoded, "OK\n");
    }

    #[test]
    fn decode_wsl_output_passes_utf8_through() {
        let s = "Ubuntu-24.04\n".as_bytes();
        let decoded = decode_wsl_output(s);
        assert_eq!(decoded, "Ubuntu-24.04\n");
    }

    #[test]
    fn evaluate_flags_missing_distro() {
        let findings = evaluate(&[], &WslConfig::default(), "Ubuntu-24.04");
        assert!(findings
            .iter()
            .any(|f| f.id == "wsl_distro_missing" && f.severity == WslSeverity::Error));
    }

    #[test]
    fn evaluate_flags_low_idle_timeout() {
        let distros = vec![WslDistro {
            name: "Ubuntu-24.04".into(),
            state: WslState::Running,
            version: 2,
            is_default: true,
        }];
        let cfg = WslConfig {
            vm_idle_timeout_ms: Some(60_000),
            ..Default::default()
        };
        let findings = evaluate(&distros, &cfg, "Ubuntu-24.04");
        assert!(findings.iter().any(|f| f.id == "wsl_idle_timeout_low"));
    }

    #[test]
    fn evaluate_passes_clean_setup() {
        let distros = vec![WslDistro {
            name: "Ubuntu-24.04".into(),
            state: WslState::Running,
            version: 2,
            is_default: true,
        }];
        let cfg = WslConfig {
            nested_virt: Some(true),
            vm_idle_timeout_ms: Some(-1),
            memory_bytes: Some(16 * 1024 * 1024 * 1024),
            networking_mode: Some("mirrored".into()),
            ..Default::default()
        };
        let findings = evaluate(&distros, &cfg, "Ubuntu-24.04");
        // No Error severity allowed in a clean setup.
        assert!(
            !findings.iter().any(|f| f.severity == WslSeverity::Error),
            "unexpected errors: {findings:?}"
        );
    }

    #[test]
    fn evaluate_flags_nested_virt_disabled() {
        let distros = vec![WslDistro {
            name: "Ubuntu-24.04".into(),
            state: WslState::Running,
            version: 2,
            is_default: true,
        }];
        let cfg = WslConfig {
            nested_virt: Some(false),
            ..Default::default()
        };
        let findings = evaluate(&distros, &cfg, "Ubuntu-24.04");
        assert!(findings
            .iter()
            .any(|f| { f.id == "wsl_nested_virt_off" && f.severity == WslSeverity::Error }));
    }
}

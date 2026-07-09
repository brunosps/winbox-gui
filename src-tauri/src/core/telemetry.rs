use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use super::paths;

pub const TELEMETRY_SCHEMA_VERSION: &str = "1.0";
pub const TELEMETRY_PREF_FILE: &str = "telemetry.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPreferences {
    pub schema_version: String,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pseudonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl Default for TelemetryPreferences {
    fn default() -> Self {
        Self {
            schema_version: TELEMETRY_SCHEMA_VERSION.to_string(),
            enabled: false,
            pseudonym: None,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEvent {
    pub schema_version: String,
    pub event: String,
    pub release: String,
    pub pseudonym: String,
    pub profile_kind: String,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub host: TelemetryHost,
    pub options: TelemetryOptions,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryHost {
    pub os_family: String,
    pub distro_family: String,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryOptions {
    pub language: String,
    pub product_family: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEventInput {
    pub event: String,
    pub phase: String,
    pub duration_ms: Option<u64>,
    pub error_code: Option<String>,
    pub language: String,
    pub product_id: String,
}

pub trait TelemetrySink {
    fn send(&self, event: &TelemetryEvent) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct CurlTelemetrySink {
    endpoint: String,
}

impl Default for CurlTelemetrySink {
    fn default() -> Self {
        Self {
            endpoint: paths::TELEMETRY_ENDPOINT_URL.to_string(),
        }
    }
}

impl CurlTelemetrySink {
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl TelemetrySink for CurlTelemetrySink {
    fn send(&self, event: &TelemetryEvent) -> Result<()> {
        let payload = serde_json::to_string(event).context("serializando evento de telemetria")?;
        Command::new("curl")
            .args([
                "--max-time",
                "5",
                "--fail",
                "--silent",
                "--show-error",
                "--request",
                "POST",
                "--header",
                "Content-Type: application/json",
                "--data-binary",
            ])
            .arg(payload)
            .arg(&self.endpoint)
            .spawn()
            .context("iniciando curl de telemetria")?;
        Ok(())
    }
}

#[derive(Default)]
pub struct MockTelemetrySink {
    pub fail: bool,
    pub sent: Mutex<Vec<TelemetryEvent>>,
}

impl TelemetrySink for MockTelemetrySink {
    fn send(&self, event: &TelemetryEvent) -> Result<()> {
        if self.fail {
            return Err(anyhow!("curl falhou"));
        }
        self.sent
            .lock()
            .expect("telemetry mock mutex")
            .push(event.clone());
        Ok(())
    }
}

pub fn telemetry_preferences_path() -> PathBuf {
    paths::config_dir().join(TELEMETRY_PREF_FILE)
}

pub fn load_preferences() -> Result<TelemetryPreferences> {
    load_preferences_at(&telemetry_preferences_path())
}

pub fn load_preferences_at(path: &Path) -> Result<TelemetryPreferences> {
    if !path.exists() {
        return Ok(TelemetryPreferences::default());
    }
    let raw = fs::read_to_string(path).with_context(|| format!("lendo {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parseando {}", path.display()))
}

pub fn set_opt_in(enabled: bool) -> Result<TelemetryPreferences> {
    let pseudonym = if enabled {
        Some(generate_pseudonym()?)
    } else {
        None
    };
    set_opt_in_at(&telemetry_preferences_path(), enabled, pseudonym)
}

pub fn set_opt_in_at(
    path: &Path,
    enabled: bool,
    new_pseudonym: Option<String>,
) -> Result<TelemetryPreferences> {
    if !enabled {
        if path.exists() {
            fs::remove_file(path).with_context(|| format!("removendo {}", path.display()))?;
        }
        return Ok(TelemetryPreferences::default());
    }
    let now = Utc::now().to_rfc3339();
    let current = load_preferences_at(path).unwrap_or_default();
    let pseudonym = current
        .pseudonym
        .filter(|_| current.enabled)
        .or(new_pseudonym)
        .ok_or_else(|| anyhow!("pseudônimo de telemetria ausente"))?;
    let prefs = TelemetryPreferences {
        schema_version: TELEMETRY_SCHEMA_VERSION.to_string(),
        enabled: true,
        pseudonym: Some(pseudonym),
        created_at: current.created_at.or_else(|| Some(now.clone())),
        updated_at: Some(now),
    };
    write_preferences_at(path, &prefs)?;
    Ok(prefs)
}

pub fn build_telemetry_event(
    prefs: &TelemetryPreferences,
    input: TelemetryEventInput,
) -> Option<TelemetryEvent> {
    if !prefs.enabled {
        return None;
    }
    let pseudonym = prefs.pseudonym.as_ref()?.clone();
    Some(TelemetryEvent {
        schema_version: TELEMETRY_SCHEMA_VERSION.to_string(),
        event: sanitize_event(&input.event),
        release: paths::WINBOX_VERSION.to_string(),
        pseudonym,
        profile_kind: "office".to_string(),
        phase: sanitize_token(&input.phase),
        duration_ms: input.duration_ms,
        error_code: input.error_code.as_deref().map(sanitize_token),
        host: current_host(),
        options: TelemetryOptions {
            language: sanitize_language(&input.language),
            product_family: product_family(&input.product_id).to_string(),
        },
        timestamp: Utc::now().to_rfc3339(),
    })
}

pub fn send_telemetry_event_best_effort(sink: &dyn TelemetrySink, event: &TelemetryEvent) {
    let _ = sink.send(event);
}

pub fn send_if_enabled_at(
    path: &Path,
    input: TelemetryEventInput,
    sink: &dyn TelemetrySink,
) -> Result<()> {
    let Ok(prefs) = load_preferences_at(path) else {
        return Ok(());
    };
    if let Some(event) = build_telemetry_event(&prefs, input) {
        send_telemetry_event_best_effort(sink, &event);
    }
    Ok(())
}

fn write_preferences_at(path: &Path, prefs: &TelemetryPreferences) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(prefs).context("serializando preferências")?;
    fs::write(&tmp, body).with_context(|| format!("escrevendo {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("renomeando {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn generate_pseudonym() -> Result<String> {
    let mut bytes = [0_u8; 16];
    fill_os_random(&mut bytes)?;
    Ok(hex_128(bytes))
}

fn fill_os_random(bytes: &mut [u8; 16]) -> Result<()> {
    let mut file = fs::File::open("/dev/urandom").context("abrindo /dev/urandom")?;
    file.read_exact(bytes).context("lendo /dev/urandom")?;
    Ok(())
}

fn hex_128(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn current_host() -> TelemetryHost {
    TelemetryHost {
        os_family: std::env::consts::OS.to_string(),
        distro_family: linux_distro_family(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

fn linux_distro_family() -> String {
    #[cfg(target_os = "linux")]
    {
        fs::read_to_string("/etc/os-release")
            .ok()
            .map(|raw| distro_family_from_os_release(&raw))
            .unwrap_or_else(|| "linux_unknown".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::consts::OS.to_string()
    }
}

fn distro_family_from_os_release(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("ubuntu") || lower.contains("linuxmint") || lower.contains("mint") {
        return "ubuntu_or_mint".to_string();
    }
    if lower.contains("debian") {
        return "debian".to_string();
    }
    if lower.contains("fedora") {
        return "fedora".to_string();
    }
    if lower.contains("arch") {
        return "arch".to_string();
    }
    "linux_other".to_string()
}

fn sanitize_event(value: &str) -> String {
    match value {
        "wizard_started" | "phase_started" | "phase_completed" | "phase_failed"
        | "wizard_completed" | "wizard_abandoned" => value.to_string(),
        _ => "unknown".to_string(),
    }
}

fn sanitize_token(value: &str) -> String {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    {
        return "unknown".to_string();
    }
    let token: String = value
        .chars()
        .filter(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == '_' || *ch == '-')
        .collect();
    if token.is_empty() {
        "unknown".to_string()
    } else {
        token
    }
}

fn sanitize_language(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "pt-br" => "pt-br".to_string(),
        "en-us" => "en-us".to_string(),
        _ => "unknown".to_string(),
    }
}

fn product_family(product_id: &str) -> &'static str {
    match product_id {
        "O365BusinessRetail" => "business",
        "O365HomePremRetail" => "home",
        _ => "enterprise",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn telemetry_opt_in_defaults_to_false() {
        let path = test_path("missing-default");
        let prefs = load_preferences_at(&path).expect("missing prefs should default");

        assert!(!prefs.enabled);
        assert!(prefs.pseudonym.is_none());
        assert!(build_telemetry_event(&prefs, input()).is_none());
    }

    #[test]
    fn telemetry_payload_has_no_machine_identifiers() {
        let prefs = TelemetryPreferences {
            enabled: true,
            pseudonym: Some("00112233445566778899aabbccddeeff".to_string()),
            ..TelemetryPreferences::default()
        };
        let event = build_telemetry_event(
            &prefs,
            TelemetryEventInput {
                event: "phase_failed".to_string(),
                phase: "/home/bruno/secret-hostname".to_string(),
                duration_ms: Some(42),
                error_code: Some("machine-id:abc".to_string()),
                language: "../../pt-br".to_string(),
                product_id: "O365BusinessRetail".to_string(),
            },
        )
        .expect("enabled prefs should build event");
        let json = serde_json::to_string(&event).expect("serialize event");

        assert!(!json.contains("bruno"));
        assert!(!json.contains("secret-hostname"));
        assert!(!json.contains("machine-id"));
        assert!(!json.contains("/home"));
        assert!(!json.contains("username"));
        assert!(!json.contains("hostname"));
        assert_eq!(event.phase, "unknown");
        assert_eq!(event.error_code.as_deref(), Some("unknown"));
        assert_eq!(event.options.language, "unknown");
        assert_eq!(event.options.product_family, "business");
    }

    #[test]
    fn telemetry_endpoint_url_comes_from_paths_constant() {
        let sink = CurlTelemetrySink::default();

        assert_eq!(sink.endpoint(), paths::TELEMETRY_ENDPOINT_URL);
        assert!(paths::TELEMETRY_ENDPOINT_URL.starts_with("https://"));
        assert!(paths::TELEMETRY_ENDPOINT_URL.contains(".invalid"));
    }

    #[test]
    fn telemetry_curl_failure_is_best_effort() {
        let prefs = TelemetryPreferences {
            enabled: true,
            pseudonym: Some("00112233445566778899aabbccddeeff".to_string()),
            ..TelemetryPreferences::default()
        };
        let event = build_telemetry_event(&prefs, input()).expect("enabled");
        let sink = MockTelemetrySink {
            fail: true,
            sent: Mutex::new(Vec::new()),
        };

        send_telemetry_event_best_effort(&sink, &event);
    }

    #[test]
    fn telemetry_opt_out_deletes_pseudonym_and_next_opt_in_rotates() {
        let path = test_path("opt-out");
        let first = set_opt_in_at(
            &path,
            true,
            Some("00112233445566778899aabbccddeeff".to_string()),
        )
        .expect("enable");
        assert!(path.exists());
        assert_eq!(
            first.pseudonym.as_deref(),
            Some("00112233445566778899aabbccddeeff")
        );

        let disabled = set_opt_in_at(&path, false, None).expect("disable");
        assert!(!disabled.enabled);
        assert!(!path.exists());

        let second = set_opt_in_at(
            &path,
            true,
            Some("ffeeddccbbaa99887766554433221100".to_string()),
        )
        .expect("re-enable");
        assert_eq!(
            second.pseudonym.as_deref(),
            Some("ffeeddccbbaa99887766554433221100")
        );
    }

    fn input() -> TelemetryEventInput {
        TelemetryEventInput {
            event: "phase_started".to_string(),
            phase: "office_odt_install".to_string(),
            duration_ms: None,
            error_code: None,
            language: "pt-br".to_string(),
            product_id: "O365ProPlusRetail".to_string(),
        }
    }

    fn test_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join("winbox-telemetry-tests")
            .join(format!("{name}-{suffix}.json"))
    }
}

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use super::paths;

pub const TELEMETRY_SCHEMA_VERSION: &str = "1.0";
pub const TELEMETRY_PREF_FILE: &str = "telemetry.json";
pub const TELEMETRY_EVENTS_FILE: &str = "events.ndjson";
pub const TELEMETRY_MAX_EVENT_BYTES: usize = 16 * 1024;
pub const TELEMETRY_RETENTION_DAYS: i64 = 30;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryHost {
    pub os_family: String,
    pub distro_family: String,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryHttpResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct TelemetryEndpoint {
    events_path: PathBuf,
    max_event_bytes: usize,
}

impl TelemetryEndpoint {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            events_path: telemetry_events_path(data_dir),
            max_event_bytes: TELEMETRY_MAX_EVENT_BYTES,
        }
    }

    pub fn with_limit(data_dir: &Path, max_event_bytes: usize) -> Self {
        Self {
            events_path: telemetry_events_path(data_dir),
            max_event_bytes,
        }
    }

    pub fn post_events(&self, body: &str) -> TelemetryHttpResponse {
        match validate_telemetry_event_payload(body, self.max_event_bytes).and_then(|event| {
            append_telemetry_event(&self.events_path, &event)?;
            self.apply_retention(Utc::now()).map(|_| ())
        }) {
            Ok(()) => TelemetryHttpResponse {
                status: 202,
                body: "{\"ok\":true}".to_string(),
            },
            Err(err) if err.to_string().contains("payload maior") => TelemetryHttpResponse {
                status: 413,
                body: "{\"ok\":false,\"error\":\"payload_too_large\"}".to_string(),
            },
            Err(err)
                if err.to_string().contains("JSON") || err.to_string().contains("inválido") =>
            {
                TelemetryHttpResponse {
                    status: 400,
                    body: "{\"ok\":false,\"error\":\"invalid_event\"}".to_string(),
                }
            }
            Err(_) => TelemetryHttpResponse {
                status: 500,
                body: "{\"ok\":false,\"error\":\"write_failed\"}".to_string(),
            },
        }
    }

    pub fn handle_request(&self, method: &str, path: &str, body: &str) -> TelemetryHttpResponse {
        match (method, path) {
            ("POST", "/events") => self.post_events(body),
            ("GET", "/export.ndjson") => self.export_response(Self::export_ndjson),
            ("GET", "/export.csv") => self.export_response(Self::export_csv),
            ("GET", "/metrics.csv") => self.export_response(Self::export_metrics_csv),
            ("POST", _) | ("GET", _) => TelemetryHttpResponse {
                status: 404,
                body: "{\"ok\":false,\"error\":\"not_found\"}".to_string(),
            },
            _ => TelemetryHttpResponse {
                status: 405,
                body: "{\"ok\":false,\"error\":\"method_not_allowed\"}".to_string(),
            },
        }
    }

    pub fn apply_retention(&self, now: DateTime<Utc>) -> Result<usize> {
        apply_telemetry_retention_at(&self.events_path, now)
    }

    pub fn export_ndjson(&self) -> Result<String> {
        let events = read_telemetry_events(&self.events_path)?;
        export_telemetry_ndjson(&events)
    }

    pub fn export_csv(&self) -> Result<String> {
        let events = read_telemetry_events(&self.events_path)?;
        Ok(export_telemetry_csv(&events))
    }

    pub fn export_metrics_csv(&self) -> Result<String> {
        let events = read_telemetry_events(&self.events_path)?;
        Ok(export_telemetry_metrics_csv(&aggregate_telemetry_metrics(
            &events,
        )))
    }

    fn export_response(&self, export: fn(&Self) -> Result<String>) -> TelemetryHttpResponse {
        match export(self) {
            Ok(body) => TelemetryHttpResponse { status: 200, body },
            Err(_) => TelemetryHttpResponse {
                status: 500,
                body: "{\"ok\":false,\"error\":\"export_failed\"}".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryMetrics {
    pub total_pseudonyms: usize,
    pub completed: usize,
    pub abandoned_by_phase: BTreeMap<String, u64>,
    pub phase_metrics: Vec<TelemetryPhaseMetric>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryPhaseMetric {
    pub phase: String,
    pub started: u64,
    pub completed: u64,
    pub failed: u64,
    pub abandoned: u64,
    pub p50_duration_ms: Option<u64>,
    pub p90_duration_ms: Option<u64>,
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

pub fn telemetry_events_path(data_dir: &Path) -> PathBuf {
    data_dir.join(TELEMETRY_EVENTS_FILE)
}

pub fn validate_telemetry_event_payload(raw: &str, max_bytes: usize) -> Result<TelemetryEvent> {
    if raw.len() > max_bytes {
        return Err(anyhow!("payload maior que o limite de telemetria"));
    }
    let event: TelemetryEvent = serde_json::from_str(raw).context("JSON de telemetria inválido")?;
    validate_telemetry_event(&event)?;
    Ok(event)
}

pub fn append_telemetry_event(path: &Path, event: &TelemetryEvent) -> Result<()> {
    validate_telemetry_event(event)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let line = serde_json::to_string(event).context("serializando evento NDJSON")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("abrindo {}", path.display()))?;
    writeln!(file, "{line}").with_context(|| format!("gravando {}", path.display()))?;
    Ok(())
}

pub fn read_telemetry_events(path: &Path) -> Result<Vec<TelemetryEvent>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).with_context(|| format!("lendo {}", path.display()))?;
    raw.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(idx, line)| {
            let event: TelemetryEvent = serde_json::from_str(line)
                .with_context(|| format!("JSON inválido em {}:{}", path.display(), idx + 1))?;
            validate_telemetry_event(&event)?;
            Ok(event)
        })
        .collect()
}

pub fn apply_telemetry_retention_at(path: &Path, now: DateTime<Utc>) -> Result<usize> {
    let cutoff = now - Duration::days(TELEMETRY_RETENTION_DAYS);
    let kept: Vec<TelemetryEvent> = read_telemetry_events(path)?
        .into_iter()
        .filter(|event| {
            DateTime::parse_from_rfc3339(&event.timestamp)
                .map(|timestamp| timestamp.with_timezone(&Utc) >= cutoff)
                .unwrap_or(false)
        })
        .collect();
    write_telemetry_events_atomic(path, &kept)?;
    Ok(kept.len())
}

pub fn export_telemetry_ndjson(events: &[TelemetryEvent]) -> Result<String> {
    let mut out = String::new();
    for event in events {
        validate_telemetry_event(event)?;
        out.push_str(&serde_json::to_string(event).context("serializando evento NDJSON")?);
        out.push('\n');
    }
    Ok(out)
}

pub fn export_telemetry_csv(events: &[TelemetryEvent]) -> String {
    let mut out =
        "release,pseudonym,event,phase,durationMs,errorCode,timestamp,profileKind,productFamily,language,osFamily,distroFamily,arch\n"
            .to_string();
    for event in events {
        out.push_str(
            &[
                csv_escape(&event.release),
                csv_escape(&event.pseudonym),
                csv_escape(&event.event),
                csv_escape(&event.phase),
                event
                    .duration_ms
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                csv_escape(event.error_code.as_deref().unwrap_or("")),
                csv_escape(&event.timestamp),
                csv_escape(&event.profile_kind),
                csv_escape(&event.options.product_family),
                csv_escape(&event.options.language),
                csv_escape(&event.host.os_family),
                csv_escape(&event.host.distro_family),
                csv_escape(&event.host.arch),
            ]
            .join(","),
        );
        out.push('\n');
    }
    out
}

pub fn aggregate_telemetry_metrics(events: &[TelemetryEvent]) -> TelemetryMetrics {
    #[derive(Default)]
    struct PhaseAccumulator {
        started: u64,
        completed: u64,
        failed: u64,
        abandoned: u64,
        durations: Vec<u64>,
    }

    let mut latest_by_pseudonym: BTreeMap<String, &TelemetryEvent> = BTreeMap::new();
    let mut phases: BTreeMap<String, PhaseAccumulator> = BTreeMap::new();

    for event in events {
        let phase = phases.entry(event.phase.clone()).or_default();
        match event.event.as_str() {
            "phase_started" | "wizard_started" => phase.started += 1,
            "phase_completed" | "wizard_completed" => phase.completed += 1,
            "phase_failed" => phase.failed += 1,
            _ => {}
        }
        if let Some(duration) = event.duration_ms {
            phase.durations.push(duration);
        }
        latest_by_pseudonym
            .entry(event.pseudonym.clone())
            .and_modify(|current| {
                if event.timestamp >= current.timestamp {
                    *current = event;
                }
            })
            .or_insert(event);
    }

    let mut completed = 0;
    let mut abandoned_by_phase = BTreeMap::new();
    for last in latest_by_pseudonym.values() {
        if last.event == "wizard_completed" {
            completed += 1;
        } else {
            *abandoned_by_phase.entry(last.phase.clone()).or_insert(0) += 1;
            phases.entry(last.phase.clone()).or_default().abandoned += 1;
        }
    }

    let phase_metrics = phases
        .into_iter()
        .map(|(phase, mut acc)| {
            acc.durations.sort_unstable();
            TelemetryPhaseMetric {
                phase,
                started: acc.started,
                completed: acc.completed,
                failed: acc.failed,
                abandoned: acc.abandoned,
                p50_duration_ms: percentile_nearest_rank(&acc.durations, 50),
                p90_duration_ms: percentile_nearest_rank(&acc.durations, 90),
            }
        })
        .collect();

    TelemetryMetrics {
        total_pseudonyms: latest_by_pseudonym.len(),
        completed,
        abandoned_by_phase,
        phase_metrics,
    }
}

pub fn export_telemetry_metrics_csv(metrics: &TelemetryMetrics) -> String {
    let mut out =
        "phase,started,completed,failed,abandoned,p50DurationMs,p90DurationMs\n".to_string();
    for phase in &metrics.phase_metrics {
        out.push_str(
            &[
                csv_escape(&phase.phase),
                phase.started.to_string(),
                phase.completed.to_string(),
                phase.failed.to_string(),
                phase.abandoned.to_string(),
                phase
                    .p50_duration_ms
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                phase
                    .p90_duration_ms
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ]
            .join(","),
        );
        out.push('\n');
    }
    out
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

fn write_telemetry_events_atomic(path: &Path, events: &[TelemetryEvent]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let tmp = path.with_extension("ndjson.tmp");
    fs::write(&tmp, export_telemetry_ndjson(events)?)
        .with_context(|| format!("escrevendo {}", tmp.display()))?;
    fs::rename(&tmp, path)
        .with_context(|| format!("renomeando {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn validate_telemetry_event(event: &TelemetryEvent) -> Result<()> {
    if event.schema_version != TELEMETRY_SCHEMA_VERSION {
        return Err(anyhow!("schemaVersion de telemetria inválido"));
    }
    if event.profile_kind != "office" {
        return Err(anyhow!("profileKind de telemetria inválido"));
    }
    if !is_hex_128(&event.pseudonym) {
        return Err(anyhow!("pseudônimo de telemetria inválido"));
    }
    if !is_allowed_event(&event.event) {
        return Err(anyhow!("evento de telemetria inválido"));
    }
    ensure_safe_token("release", &event.release, true)?;
    ensure_safe_token("phase", &event.phase, false)?;
    if let Some(error_code) = &event.error_code {
        ensure_safe_token("errorCode", error_code, false)?;
    }
    ensure_safe_token("osFamily", &event.host.os_family, false)?;
    ensure_safe_token("distroFamily", &event.host.distro_family, false)?;
    ensure_safe_token("arch", &event.host.arch, false)?;
    match event.options.language.as_str() {
        "pt-br" | "en-us" | "unknown" => {}
        _ => return Err(anyhow!("language de telemetria inválido")),
    }
    match event.options.product_family.as_str() {
        "enterprise" | "business" | "home" | "unknown" => {}
        _ => return Err(anyhow!("productFamily de telemetria inválido")),
    }
    DateTime::parse_from_rfc3339(&event.timestamp).context("timestamp de telemetria inválido")?;
    Ok(())
}

fn is_allowed_event(value: &str) -> bool {
    matches!(
        value,
        "wizard_started"
            | "phase_started"
            | "phase_completed"
            | "phase_failed"
            | "wizard_completed"
            | "wizard_abandoned"
    )
}

fn is_hex_128(value: &str) -> bool {
    value.len() == 32 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn ensure_safe_token(label: &str, value: &str, allow_semver_plus: bool) -> Result<()> {
    if value.is_empty() || value.len() > 128 {
        return Err(anyhow!("{label} de telemetria inválido"));
    }
    let valid = value.chars().all(|ch| {
        ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
            || ch == '_'
            || ch == '-'
            || ch == '.'
            || (allow_semver_plus && ch == '+')
    });
    if !valid {
        return Err(anyhow!("{label} de telemetria inválido"));
    }
    Ok(())
}

fn percentile_nearest_rank(sorted_values: &[u64], percentile: usize) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let rank = (percentile * sorted_values.len()).div_ceil(100);
    sorted_values.get(rank.saturating_sub(1)).copied()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
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

    #[test]
    fn telemetry_endpoint_accepts_valid_beta_event_locally() {
        let dir = test_dir("endpoint-valid");
        let endpoint = TelemetryEndpoint::new(&dir);
        let event = event(
            "00112233445566778899aabbccddeeff",
            "phase_started",
            "office_byol",
            None,
        );
        let payload = serde_json::to_string(&event).expect("serialize");

        let response = endpoint.post_events(&payload);
        let stored = read_telemetry_events(&telemetry_events_path(&dir)).expect("read stored");

        assert_eq!(response.status, 202);
        assert_eq!(stored, vec![event]);
    }

    #[test]
    fn telemetry_endpoint_rejects_oversized_payload_locally() {
        let dir = test_dir("endpoint-large");
        let endpoint = TelemetryEndpoint::with_limit(&dir, 8);

        let response = endpoint.post_events("{\"schemaVersion\":\"1.0\"}");

        assert_eq!(response.status, 413);
        assert!(!telemetry_events_path(&dir).exists());
    }

    #[test]
    fn telemetry_endpoint_exports_ndjson_csv_and_metrics_locally() {
        let dir = test_dir("endpoint-export");
        let endpoint = TelemetryEndpoint::new(&dir);
        let event = event(
            "99999999999999999999999999999999",
            "phase_completed",
            "office_install",
            Some(45_000),
        );
        let payload = serde_json::to_string(&event).expect("serialize");

        assert_eq!(
            endpoint.handle_request("POST", "/events", &payload).status,
            202
        );
        let ndjson = endpoint.handle_request("GET", "/export.ndjson", "");
        let csv = endpoint.handle_request("GET", "/export.csv", "");
        let metrics = endpoint.handle_request("GET", "/metrics.csv", "");

        assert_eq!(ndjson.status, 200);
        assert!(ndjson.body.contains("\"office_install\""));
        assert_eq!(csv.status, 200);
        assert!(csv.body.starts_with("release,pseudonym,event"));
        assert_eq!(metrics.status, 200);
        assert!(metrics.body.contains("p50DurationMs"));
        assert_eq!(endpoint.handle_request("GET", "/missing", "").status, 404);
    }

    #[test]
    fn abandonment_is_derived_server_side_from_last_event() {
        let events = vec![
            event(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "phase_started",
                "office_byol",
                None,
            ),
            event(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "phase_completed",
                "office_byol",
                Some(100),
            ),
            event(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "wizard_completed",
                "office_final_verify",
                Some(1_000),
            ),
            event(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "phase_started",
                "office_byol",
                None,
            ),
            event(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "phase_completed",
                "office_byol",
                Some(300),
            ),
            event(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "phase_failed",
                "office_install",
                Some(900),
            ),
            event(
                "cccccccccccccccccccccccccccccccc",
                "phase_completed",
                "office_byol",
                Some(500),
            ),
        ];

        let metrics = aggregate_telemetry_metrics(&events);
        let byol = metrics
            .phase_metrics
            .iter()
            .find(|phase| phase.phase == "office_byol")
            .expect("byol metrics");

        assert_eq!(metrics.total_pseudonyms, 3);
        assert_eq!(metrics.completed, 1);
        assert_eq!(
            metrics.abandoned_by_phase.get("office_install").copied(),
            Some(1)
        );
        assert_eq!(
            metrics.abandoned_by_phase.get("office_byol").copied(),
            Some(1)
        );
        assert_eq!(byol.p50_duration_ms, Some(300));
        assert_eq!(byol.p90_duration_ms, Some(500));
    }

    #[test]
    fn telemetry_retention_keeps_only_last_thirty_days() {
        let path = test_path("retention");
        let now = Utc::now();
        let mut old = event(
            "dddddddddddddddddddddddddddddddd",
            "phase_started",
            "office_byol",
            None,
        );
        old.timestamp = (now - Duration::days(31)).to_rfc3339();
        let mut recent = event(
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "phase_started",
            "office_byol",
            None,
        );
        recent.timestamp = (now - Duration::days(1)).to_rfc3339();
        append_telemetry_event(&path, &old).expect("append old");
        append_telemetry_event(&path, &recent).expect("append recent");

        let kept = apply_telemetry_retention_at(&path, now).expect("retention");
        let events = read_telemetry_events(&path).expect("read retained");

        assert_eq!(kept, 1);
        assert_eq!(events, vec![recent]);
    }

    #[test]
    fn telemetry_export_csv_escapes_values_and_metrics() {
        let mut event = event(
            "ffffffffffffffffffffffffffffffff",
            "phase_failed",
            "office_install",
            Some(42),
        );
        event.error_code = Some("guest_phase_timeout".to_string());
        let raw_csv = export_telemetry_csv(&[event.clone()]);
        let metrics_csv =
            export_telemetry_metrics_csv(&aggregate_telemetry_metrics(&[event.clone()]));
        let ndjson = export_telemetry_ndjson(&[event]).expect("ndjson");

        assert!(raw_csv.contains("guest_phase_timeout"));
        assert!(metrics_csv.contains("office_install"));
        assert!(ndjson.ends_with('\n'));
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

    fn test_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir()
            .join("winbox-telemetry-tests")
            .join(format!("{name}-{suffix}"))
    }

    fn event(
        pseudonym: &str,
        event_name: &str,
        phase: &str,
        duration_ms: Option<u64>,
    ) -> TelemetryEvent {
        let prefs = TelemetryPreferences {
            enabled: true,
            pseudonym: Some(pseudonym.to_string()),
            ..TelemetryPreferences::default()
        };
        build_telemetry_event(
            &prefs,
            TelemetryEventInput {
                event: event_name.to_string(),
                phase: phase.to_string(),
                duration_ms,
                error_code: None,
                language: "pt-br".to_string(),
                product_id: "O365ProPlusRetail".to_string(),
            },
        )
        .expect("event")
    }
}

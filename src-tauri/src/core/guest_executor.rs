use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::{env_file, paths};

pub const REMOTEAPP_PREPARE_MARKER: &str = "remoteapp_prepare.json";
pub const REMOTEAPP_PREPARE_SCRIPT: &str = "winbox-remoteapp-prepare.ps1";
pub const REMOTEAPP_NOOP_SCRIPT: &str = "winbox-remoteapp-noop.ps1";
pub const GUEST_REMOTEAPP_NOT_PREPARED_CODE: &str = "guest_remoteapp_not_prepared";
pub const GUEST_EXECUTOR_FAILED_CODE: &str = "guest_executor_failed";
pub const GUEST_PHASE_TIMEOUT_CODE: &str = "guest_phase_timeout";
pub const GUEST_DISK_FULL_CODE: &str = "guest_disk_full";
pub const OFFICE_ODT_FAILED_CODE: &str = "office_odt_failed";
pub const OFFICE_DETECTION_FAILED_CODE: &str = "office_detection_failed";
pub const REMOTEAPP_ACTION_HINT: &str =
    "Abra o desktop via noVNC e execute C:\\OEM\\install.bat; se necessário, aplique as chaves RemoteApp por sessão RDP full-desktop.";
pub const OFFICE_INSTALL_TIMEOUT_SECS: u64 = 60 * 60;

const OFFICE_SHARE_DIR: &str = "winbox-office";
const SCRIPTS_DIR: &str = "scripts";
const MARKERS_DIR: &str = "markers";
const LOGS_DIR: &str = "logs";
const GUEST_SHARE_ROOT: &str = r"\\host.lan\Data";
const POWERSHELL_REMOTEAPP: &str = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
const FLATPAK_FREERDP_COMMAND: &str = "flatpak run --command=xfreerdp com.freerdp.FreeRDP";

const ALLOWED_REGISTRY_VALUES: [(&str, &str, u32); 5] = [
    (
        r"HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server",
        "fDenyTSConnections",
        0,
    ),
    (
        r"HKLM\SYSTEM\CurrentControlSet\Control\Terminal Server\WinStations\RDP-Tcp",
        "UserAuthentication",
        1,
    ),
    (
        r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Terminal Server\TSAppAllowList",
        "fDisabledAllowList",
        1,
    ),
    (
        r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Terminal Server\TSAppAllowList",
        "fAllowUnlistedRemotePrograms",
        1,
    ),
    (
        r"HKLM\SOFTWARE\Microsoft\Terminal Server Client\Default\AddIns\RDPDR",
        "IgnoreRemoteKeyboardLayout",
        1,
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestScript {
    pub phase: String,
    pub file_name: String,
    pub contents: String,
}

impl GuestScript {
    pub fn remoteapp_noop() -> Self {
        Self {
            phase: "remoteapp_prepare".to_string(),
            file_name: REMOTEAPP_NOOP_SCRIPT.to_string(),
            contents: crlf(&[
                "$ErrorActionPreference = 'Stop'",
                "Write-Host 'winbox remoteapp noop'",
                "exit 0",
            ]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestRun {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuestPhaseTimeout {
    pub phase: &'static str,
    pub marker: &'static str,
    pub timeout: Duration,
    pub poll_interval: Duration,
}

impl GuestPhaseTimeout {
    pub fn office_install() -> Self {
        Self {
            phase: "office_install",
            marker: "office_install.json",
            timeout: Duration::from_secs(OFFICE_INSTALL_TIMEOUT_SECS),
            poll_interval: Duration::from_secs(5),
        }
    }

    pub fn immediate(phase: &'static str, marker: &'static str) -> Self {
        Self {
            phase,
            marker,
            timeout: Duration::from_secs(0),
            poll_interval: Duration::from_secs(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestPhaseError {
    ExecutorFailed {
        phase: String,
        detail: String,
    },
    PhaseTimeout {
        phase: String,
        marker: String,
        timeout_secs: u64,
    },
    DiskFull {
        phase: String,
        detail: String,
    },
    OfficeOdtFailed {
        phase: String,
        detail: String,
    },
    OfficeDetectionFailed {
        phase: String,
        detail: String,
    },
}

impl GuestPhaseError {
    pub fn code(&self) -> &'static str {
        match self {
            GuestPhaseError::ExecutorFailed { .. } => GUEST_EXECUTOR_FAILED_CODE,
            GuestPhaseError::PhaseTimeout { .. } => GUEST_PHASE_TIMEOUT_CODE,
            GuestPhaseError::DiskFull { .. } => GUEST_DISK_FULL_CODE,
            GuestPhaseError::OfficeOdtFailed { .. } => OFFICE_ODT_FAILED_CODE,
            GuestPhaseError::OfficeDetectionFailed { .. } => OFFICE_DETECTION_FAILED_CODE,
        }
    }

    pub fn phase(&self) -> &str {
        match self {
            GuestPhaseError::ExecutorFailed { phase, .. }
            | GuestPhaseError::PhaseTimeout { phase, .. }
            | GuestPhaseError::DiskFull { phase, .. }
            | GuestPhaseError::OfficeOdtFailed { phase, .. }
            | GuestPhaseError::OfficeDetectionFailed { phase, .. } => phase,
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            GuestPhaseError::ExecutorFailed { detail, .. }
            | GuestPhaseError::DiskFull { detail, .. }
            | GuestPhaseError::OfficeOdtFailed { detail, .. }
            | GuestPhaseError::OfficeDetectionFailed { detail, .. } => detail,
            GuestPhaseError::PhaseTimeout { marker, .. } => marker,
        }
    }

    pub fn details_json(&self) -> serde_json::Value {
        match self {
            GuestPhaseError::PhaseTimeout {
                marker,
                timeout_secs,
                ..
            } => serde_json::json!({
                "marker": marker,
                "timeoutSeconds": timeout_secs,
            }),
            GuestPhaseError::ExecutorFailed { detail, .. }
            | GuestPhaseError::DiskFull { detail, .. }
            | GuestPhaseError::OfficeOdtFailed { detail, .. }
            | GuestPhaseError::OfficeDetectionFailed { detail, .. } => {
                serde_json::json!({ "detail": detail })
            }
        }
    }
}

impl std::fmt::Display for GuestPhaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuestPhaseError::ExecutorFailed { phase, detail } => {
                write!(f, "{}: fase {} falhou: {}", self.code(), phase, detail)
            }
            GuestPhaseError::PhaseTimeout {
                phase,
                marker,
                timeout_secs,
            } => write!(
                f,
                "{}: marker {} da fase {} não apareceu em {}s",
                self.code(),
                marker,
                phase,
                timeout_secs
            ),
            GuestPhaseError::DiskFull { phase, detail }
            | GuestPhaseError::OfficeOdtFailed { phase, detail }
            | GuestPhaseError::OfficeDetectionFailed { phase, detail } => {
                write!(f, "{}: fase {} falhou: {}", self.code(), phase, detail)
            }
        }
    }
}

impl std::error::Error for GuestPhaseError {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestMarker {
    pub phase: String,
    pub status: GuestMarkerStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<GuestMarkerError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub office: Option<OfficeInstallEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuestMarkerStatus {
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestMarkerError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeInstallEvidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_release_ids: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_to_report: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe_paths: Option<OfficeExePaths>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeExePaths {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winword: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub powerpnt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeInstallVerification {
    pub exit_code: i32,
    pub office: OfficeInstallEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteappBootstrapFiles {
    pub install_bat: PathBuf,
    pub prepare_script: PathBuf,
    pub marker: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestRemoteappNotPrepared {
    pub detail: String,
}

impl GuestRemoteappNotPrepared {
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        GUEST_REMOTEAPP_NOT_PREPARED_CODE
    }

    pub fn action_hint(&self) -> &'static str {
        REMOTEAPP_ACTION_HINT
    }
}

impl std::fmt::Display for GuestRemoteappNotPrepared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} Próxima ação: {}",
            self.code(),
            self.detail,
            self.action_hint()
        )
    }
}

impl std::error::Error for GuestRemoteappNotPrepared {}

pub trait GuestExecutor {
    fn run_script(&self, profile: &str, script: GuestScript) -> Result<GuestRun>;
    fn start_detached_script(&self, profile: &str, script: GuestScript) -> Result<GuestRun>;
    fn read_marker(&self, profile: &str, marker: &str) -> Result<Option<GuestMarker>>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CliGuestExecutor;

impl GuestExecutor for CliGuestExecutor {
    fn run_script(&self, profile: &str, script: GuestScript) -> Result<GuestRun> {
        validate_script_name(&script.file_name)?;
        let shared_dir = profile_shared_dir_from_env(profile)?;
        let script_path = guest_scripts_dir(&shared_dir).join(&script.file_name);
        write_crlf_file(&script_path, &script.contents)?;

        let env_path = paths::profile_env_file(profile);
        let map = env_file::read(&env_path)?;
        let (program, mut args) = freerdp_command_parts(env_file::get(&map, "FREERDP_COMMAND"))?;
        args.extend([
            format!("/v:127.0.0.1:{}", env_file::get(&map, "RDP_PORT")),
            format!("/u:{}", env_file::get(&map, "USERNAME")),
            format!("/p:{}", env_file::get(&map, "PASSWORD")),
            "/cert:ignore".to_string(),
            "+home-drive".to_string(),
            format!("/app:{POWERSHELL_REMOTEAPP}"),
            format!(
                "/app-cmd:-NoProfile -ExecutionPolicy Bypass -File \"{}\"",
                guest_script_unc(&script.file_name)
            ),
        ]);

        let output = Command::new(&program)
            .args(args)
            .output()
            .with_context(|| format!("executando RemoteApp via {program}"))?;
        Ok(GuestRun {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    fn start_detached_script(&self, profile: &str, script: GuestScript) -> Result<GuestRun> {
        validate_script_name(&script.file_name)?;
        let shared_dir = profile_shared_dir_from_env(profile)?;
        write_crlf_file(
            &guest_scripts_dir(&shared_dir).join(&script.file_name),
            &script.contents,
        )?;

        let launcher = GuestScript {
            phase: script.phase.clone(),
            file_name: detached_launcher_name(&script.phase),
            contents: render_detached_launcher_script(&script.phase, &script.file_name)?,
        };
        self.run_script(profile, launcher)
    }

    fn read_marker(&self, profile: &str, marker: &str) -> Result<Option<GuestMarker>> {
        validate_script_name(marker)?;
        let shared_dir = profile_shared_dir_from_env(profile)?;
        let marker_path = guest_markers_dir(&shared_dir).join(marker);
        if !marker_path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&marker_path)
            .with_context(|| format!("lendo marker {}", marker_path.display()))?;
        Ok(Some(parse_guest_marker(&json)?))
    }
}

pub fn start_detached_and_wait_for_marker(
    profile: &str,
    executor: &dyn GuestExecutor,
    script: GuestScript,
    timeout: GuestPhaseTimeout,
) -> std::result::Result<GuestMarker, GuestPhaseError> {
    let phase = script.phase.clone();
    let run = executor
        .start_detached_script(profile, script)
        .map_err(|err| GuestPhaseError::ExecutorFailed {
            phase: phase.clone(),
            detail: format!("{err:#}"),
        })?;
    if run.exit_code != 0 {
        return Err(GuestPhaseError::ExecutorFailed {
            phase,
            detail: format!(
                "launcher detached retornou {}: {}{}",
                run.exit_code, run.stdout, run.stderr
            ),
        });
    }

    poll_marker_with(timeout, || {
        executor
            .read_marker(profile, timeout.marker)
            .map_err(|err| GuestPhaseError::ExecutorFailed {
                phase: timeout.phase.to_string(),
                detail: format!("{err:#}"),
            })
    })
}

pub fn poll_marker_with<F>(
    timeout: GuestPhaseTimeout,
    mut read_marker: F,
) -> std::result::Result<GuestMarker, GuestPhaseError>
where
    F: FnMut() -> std::result::Result<Option<GuestMarker>, GuestPhaseError>,
{
    let started = Instant::now();
    loop {
        if let Some(marker) = read_marker()? {
            if marker.status != GuestMarkerStatus::Running {
                return Ok(marker);
            }
        }
        if started.elapsed() >= timeout.timeout {
            return Err(GuestPhaseError::PhaseTimeout {
                phase: timeout.phase.to_string(),
                marker: timeout.marker.to_string(),
                timeout_secs: timeout.timeout.as_secs(),
            });
        }
        std::thread::sleep(timeout.poll_interval);
    }
}

pub fn verify_office_install_marker(
    marker: &GuestMarker,
) -> std::result::Result<OfficeInstallVerification, GuestPhaseError> {
    if marker.phase != "office_install" {
        return Err(GuestPhaseError::ExecutorFailed {
            phase: "office_install".to_string(),
            detail: format!("marker de fase inesperada '{}'", marker.phase),
        });
    }

    if marker.status == GuestMarkerStatus::Failed {
        return Err(error_from_failed_marker(marker));
    }
    if marker.status != GuestMarkerStatus::Done {
        return Err(GuestPhaseError::PhaseTimeout {
            phase: marker.phase.clone(),
            marker: "office_install.json".to_string(),
            timeout_secs: OFFICE_INSTALL_TIMEOUT_SECS,
        });
    }

    let exit_code = marker.exit_code.unwrap_or(-1);
    if exit_code != 0 {
        return Err(GuestPhaseError::OfficeOdtFailed {
            phase: marker.phase.clone(),
            detail: format!("ODT retornou exit code {exit_code}"),
        });
    }

    let office = marker
        .office
        .clone()
        .ok_or_else(|| GuestPhaseError::OfficeDetectionFailed {
            phase: marker.phase.clone(),
            detail: "marker não contém evidência office".to_string(),
        })?;
    validate_office_evidence(&marker.phase, &office)?;
    Ok(OfficeInstallVerification { exit_code, office })
}

pub fn render_detached_launcher_script(phase: &str, script_file_name: &str) -> Result<String> {
    validate_script_name(script_file_name)?;
    validate_script_name(&detached_launcher_name(phase))?;
    let task_name = format!("WinboxOffice-{}", sanitize_task_segment(phase));
    let script_unc = guest_script_unc(script_file_name);
    Ok(crlf(&[
        "$ErrorActionPreference = 'Stop'",
        &format!("$taskName = '{}'", powershell_single_quoted(&task_name)),
        &format!("$scriptPath = '{}'", powershell_single_quoted(&script_unc)),
        "$runAt = (Get-Date).AddMinutes(1).ToString('HH:mm')",
        "$taskAction = 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"' + $scriptPath + '\"'",
        "schtasks.exe /Delete /TN $taskName /F 2>$null | Out-Null",
        "schtasks.exe /Create /TN $taskName /SC ONCE /ST $runAt /TR $taskAction /F /RL HIGHEST | Out-Null",
        "schtasks.exe /Run /TN $taskName | Out-Null",
        "Write-Host \"winbox detached task started: $taskName\"",
        "exit 0",
    ]))
}

pub fn stage_remoteapp_bootstrap(
    oem_dir: &Path,
    shared_dir: &Path,
) -> Result<RemoteappBootstrapFiles> {
    std::fs::create_dir_all(oem_dir).with_context(|| format!("mkdir {}", oem_dir.display()))?;
    std::fs::create_dir_all(guest_scripts_dir(shared_dir))
        .with_context(|| format!("mkdir {}", guest_scripts_dir(shared_dir).display()))?;
    std::fs::create_dir_all(guest_markers_dir(shared_dir))
        .with_context(|| format!("mkdir {}", guest_markers_dir(shared_dir).display()))?;
    std::fs::create_dir_all(guest_logs_dir(shared_dir))
        .with_context(|| format!("mkdir {}", guest_logs_dir(shared_dir).display()))?;

    let install_bat = oem_dir.join("install.bat");
    let prepare_script = oem_dir.join(REMOTEAPP_PREPARE_SCRIPT);
    let marker = guest_markers_dir(shared_dir).join(REMOTEAPP_PREPARE_MARKER);

    write_crlf_file(&install_bat, &render_remoteapp_install_bat())?;
    write_crlf_file(&prepare_script, &render_remoteapp_bootstrap())?;

    Ok(RemoteappBootstrapFiles {
        install_bat,
        prepare_script,
        marker,
    })
}

pub fn render_remoteapp_install_bat() -> String {
    crlf(&[
        "@echo off",
        "setlocal",
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:\\OEM\\winbox-remoteapp-prepare.ps1",
        "exit /b %ERRORLEVEL%",
    ])
}

pub fn render_remoteapp_bootstrap() -> String {
    let mut lines = vec![
        "# winbox RemoteApp bootstrap clean-room".to_string(),
        "$ErrorActionPreference = 'Stop'".to_string(),
        "$shareRoot = '\\\\host.lan\\Data\\winbox-office'".to_string(),
        "$markerDir = Join-Path $shareRoot 'markers'".to_string(),
        "$logDir = Join-Path $shareRoot 'logs'".to_string(),
        "New-Item -ItemType Directory -Force -Path $markerDir, $logDir | Out-Null".to_string(),
        "$markerPath = Join-Path $markerDir 'remoteapp_prepare.json'".to_string(),
        "$logPath = Join-Path $logDir 'remoteapp_prepare.log'".to_string(),
        "function Write-WinboxMarker($status, $exitCode, $errorCode, $message) {".to_string(),
        "  $payload = [ordered]@{ phase = 'remoteapp_prepare'; status = $status; exitCode = $exitCode; updatedAt = (Get-Date).ToUniversalTime().ToString('o') }".to_string(),
        "  if ($errorCode) { $payload.error = [ordered]@{ code = $errorCode; message = $message; logPath = 'logs/remoteapp_prepare.log' } }".to_string(),
        "  $payload | ConvertTo-Json -Compress | Set-Content -Path $markerPath -Encoding UTF8".to_string(),
        "}".to_string(),
        "try {".to_string(),
        "  Start-Transcript -Path $logPath -Append -ErrorAction SilentlyContinue | Out-Null".to_string(),
    ];
    for (path, name, value) in ALLOWED_REGISTRY_VALUES {
        lines.push(format!(
            "  & reg.exe add \"{path}\" /v {name} /t REG_DWORD /d {value} /f | Out-Null"
        ));
    }
    lines.extend([
        "  Write-WinboxMarker 'done' 0 $null $null".to_string(),
        "  Stop-Transcript -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "  exit 0".to_string(),
        "} catch {".to_string(),
        "  $message = $_.Exception.Message".to_string(),
        "  Write-WinboxMarker 'failed' 1 'guest_remoteapp_not_prepared' $message".to_string(),
        "  Stop-Transcript -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "  exit 1".to_string(),
        "}".to_string(),
    ]);
    let refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
    crlf(&refs)
}

pub fn verify_remoteapp_marker(marker: Option<&GuestMarker>) -> Result<GuestMarker> {
    let Some(marker) = marker else {
        bail!("Marker remoteapp_prepare.json ausente.");
    };
    if marker.phase != "remoteapp_prepare" {
        bail!("Marker remoteapp_prepare.json tem fase '{}'.", marker.phase);
    }
    if marker.status != GuestMarkerStatus::Done {
        bail!(
            "Marker remoteapp_prepare.json não concluído: {:?}.",
            marker.status
        );
    }
    Ok(marker.clone())
}

pub fn parse_guest_marker(json: &str) -> Result<GuestMarker> {
    serde_json::from_str(json).context("marker guest inválido")
}

pub fn probe_remoteapp_channel(profile: &str, executor: &dyn GuestExecutor) -> Result<GuestRun> {
    let run = executor
        .run_script(profile, GuestScript::remoteapp_noop())
        .map_err(|err| anyhow!(GuestRemoteappNotPrepared::new(err.to_string())))?;
    if run.exit_code != 0 {
        return Err(anyhow!(GuestRemoteappNotPrepared::new(format!(
            "script no-op RemoteApp retornou exit code {}",
            run.exit_code
        ))));
    }
    Ok(run)
}

fn error_from_failed_marker(marker: &GuestMarker) -> GuestPhaseError {
    let Some(error) = &marker.error else {
        return GuestPhaseError::ExecutorFailed {
            phase: marker.phase.clone(),
            detail: "marker failed sem error.code".to_string(),
        };
    };
    let detail = if error.message.trim().is_empty() {
        error.code.clone()
    } else {
        error.message.clone()
    };
    match error.code.as_str() {
        GUEST_DISK_FULL_CODE => GuestPhaseError::DiskFull {
            phase: marker.phase.clone(),
            detail,
        },
        OFFICE_ODT_FAILED_CODE => GuestPhaseError::OfficeOdtFailed {
            phase: marker.phase.clone(),
            detail,
        },
        OFFICE_DETECTION_FAILED_CODE => GuestPhaseError::OfficeDetectionFailed {
            phase: marker.phase.clone(),
            detail,
        },
        _ => GuestPhaseError::ExecutorFailed {
            phase: marker.phase.clone(),
            detail: format!("{}: {}", error.code, detail),
        },
    }
}

fn validate_office_evidence(
    phase: &str,
    office: &OfficeInstallEvidence,
) -> std::result::Result<(), GuestPhaseError> {
    require_non_empty(
        phase,
        "ProductReleaseIds",
        office.product_release_ids.as_deref(),
    )?;
    require_non_empty(
        phase,
        "VersionToReport",
        office.version_to_report.as_deref(),
    )?;
    let platform = require_non_empty(phase, "Platform", office.platform.as_deref())?;
    if !platform.eq_ignore_ascii_case("x64") {
        return Err(GuestPhaseError::OfficeDetectionFailed {
            phase: phase.to_string(),
            detail: format!("Platform esperado x64, obtido {platform}"),
        });
    }

    let exe_paths =
        office
            .exe_paths
            .as_ref()
            .ok_or_else(|| GuestPhaseError::OfficeDetectionFailed {
                phase: phase.to_string(),
                detail: "exePaths ausente no marker Office".to_string(),
            })?;
    require_non_empty(phase, "EXCEL.EXE", exe_paths.excel.as_deref())?;
    require_non_empty(phase, "WINWORD.EXE", exe_paths.winword.as_deref())?;
    require_non_empty(phase, "POWERPNT.EXE", exe_paths.powerpnt.as_deref())?;
    Ok(())
}

fn require_non_empty<'a>(
    phase: &str,
    label: &str,
    value: Option<&'a str>,
) -> std::result::Result<&'a str, GuestPhaseError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| GuestPhaseError::OfficeDetectionFailed {
            phase: phase.to_string(),
            detail: format!("{label} ausente na verificação Office"),
        })
}

pub fn office_share_dir(shared_dir: &Path) -> PathBuf {
    shared_dir.join(OFFICE_SHARE_DIR)
}

pub fn guest_scripts_dir(shared_dir: &Path) -> PathBuf {
    office_share_dir(shared_dir).join(SCRIPTS_DIR)
}

pub fn guest_markers_dir(shared_dir: &Path) -> PathBuf {
    office_share_dir(shared_dir).join(MARKERS_DIR)
}

pub fn guest_logs_dir(shared_dir: &Path) -> PathBuf {
    office_share_dir(shared_dir).join(LOGS_DIR)
}

pub fn guest_script_unc(file_name: &str) -> String {
    format!(r"{GUEST_SHARE_ROOT}\{OFFICE_SHARE_DIR}\{SCRIPTS_DIR}\{file_name}")
}

fn profile_shared_dir_from_env(profile: &str) -> Result<PathBuf> {
    let env_path = paths::profile_env_file(profile);
    let map = env_file::read(&env_path)?;
    let shared = env_file::get(&map, "SHARED_DIR").trim();
    if shared.is_empty() {
        return Ok(paths::profile_shared_dir(profile));
    }
    Ok(PathBuf::from(shared))
}

fn freerdp_command_parts(raw: &str) -> Result<(String, Vec<String>)> {
    let raw = raw.trim();
    match raw {
        "" | "xfreerdp" => Ok(("xfreerdp".to_string(), Vec::new())),
        "xfreerdp3" => Ok(("xfreerdp3".to_string(), Vec::new())),
        FLATPAK_FREERDP_COMMAND => Ok((
            "flatpak".to_string(),
            vec![
                "run".to_string(),
                "--command=xfreerdp".to_string(),
                "com.freerdp.FreeRDP".to_string(),
            ],
        )),
        other => bail!("FREERDP_COMMAND não suportado para Office: {other}"),
    }
}

fn detached_launcher_name(phase: &str) -> String {
    format!("winbox-detached-{}.ps1", sanitize_task_segment(phase))
}

fn sanitize_task_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn validate_script_name(file_name: &str) -> Result<()> {
    if file_name.is_empty()
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains("..")
    {
        bail!("Nome de script/marker inválido: {file_name}");
    }
    Ok(())
}

fn write_crlf_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(path, normalize_crlf(contents))
        .with_context(|| format!("escrevendo {}", path.display()))
}

fn normalize_crlf(contents: &str) -> String {
    contents.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn crlf(lines: &[&str]) -> String {
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

#[cfg(test)]
pub mod mock {
    use super::{GuestExecutor, GuestMarker, GuestRun, GuestScript};
    use anyhow::{bail, Result};
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    #[derive(Default)]
    pub struct MockGuestExecutor {
        markers: RefCell<BTreeMap<String, Option<GuestMarker>>>,
        runs: RefCell<Vec<GuestScript>>,
        detached_runs: RefCell<Vec<GuestScript>>,
        fail_next_run: RefCell<Option<String>>,
        fail_next_detached_run: RefCell<Option<String>>,
        exit_code: RefCell<i32>,
        detached_exit_code: RefCell<i32>,
    }

    impl MockGuestExecutor {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn seed_marker(&self, marker: &str, value: Option<GuestMarker>) {
            self.markers.borrow_mut().insert(marker.to_string(), value);
        }

        pub fn fail_next_run(&self, message: &str) {
            *self.fail_next_run.borrow_mut() = Some(message.to_string());
        }

        pub fn fail_next_detached_run(&self, message: &str) {
            *self.fail_next_detached_run.borrow_mut() = Some(message.to_string());
        }

        pub fn seed_exit_code(&self, code: i32) {
            *self.exit_code.borrow_mut() = code;
        }

        pub fn seed_detached_exit_code(&self, code: i32) {
            *self.detached_exit_code.borrow_mut() = code;
        }

        pub fn runs(&self) -> Vec<GuestScript> {
            self.runs.borrow().clone()
        }

        pub fn detached_runs(&self) -> Vec<GuestScript> {
            self.detached_runs.borrow().clone()
        }
    }

    impl GuestExecutor for MockGuestExecutor {
        fn run_script(&self, _profile: &str, script: GuestScript) -> Result<GuestRun> {
            self.runs.borrow_mut().push(script);
            if let Some(message) = self.fail_next_run.borrow_mut().take() {
                bail!("{message}");
            }
            Ok(GuestRun {
                exit_code: *self.exit_code.borrow(),
                stdout: String::new(),
                stderr: String::new(),
            })
        }

        fn start_detached_script(&self, _profile: &str, script: GuestScript) -> Result<GuestRun> {
            self.detached_runs.borrow_mut().push(script);
            if let Some(message) = self.fail_next_detached_run.borrow_mut().take() {
                bail!("{message}");
            }
            Ok(GuestRun {
                exit_code: *self.detached_exit_code.borrow(),
                stdout: String::new(),
                stderr: String::new(),
            })
        }

        fn read_marker(&self, _profile: &str, marker: &str) -> Result<Option<GuestMarker>> {
            Ok(self.markers.borrow().get(marker).cloned().unwrap_or(None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockGuestExecutor;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn remoteapp_bootstrap_is_clean_room_crlf() {
        let script = render_remoteapp_bootstrap();

        assert!(script.ends_with("\r\n"));
        assert_no_lf_without_cr(&script);
        for (path, name, value) in ALLOWED_REGISTRY_VALUES {
            assert!(script.contains(path), "missing registry path {path}");
            assert!(script.contains(name), "missing registry value {name}");
            assert!(
                script.contains(&format!("/d {value}")),
                "missing registry data for {name}"
            );
        }
        assert_eq!(
            script.matches("reg.exe add").count(),
            ALLOWED_REGISTRY_VALUES.len()
        );
        assert!(!script.contains("RDPApps.reg"));
        assert!(!script.contains("WinApps"));
    }

    #[test]
    fn verify_remoteapp_marker_accepts_done_and_rejects_missing_or_invalid() {
        let marker = parse_guest_marker(
            r#"{"phase":"remoteapp_prepare","status":"done","exitCode":0,"updatedAt":"2026-07-08T00:00:00Z"}"#,
        )
        .expect("marker json should parse");

        let verified = verify_remoteapp_marker(Some(&marker)).expect("done marker should pass");

        assert_eq!(verified.phase, "remoteapp_prepare");
        assert_eq!(verified.status, GuestMarkerStatus::Done);
        assert!(verify_remoteapp_marker(None).is_err());
        assert!(parse_guest_marker("{not-json").is_err());

        let failed = GuestMarker {
            status: GuestMarkerStatus::Failed,
            ..marker
        };
        assert!(verify_remoteapp_marker(Some(&failed)).is_err());
    }

    #[test]
    fn stage_remoteapp_bootstrap_writes_oem_and_share_layout() {
        let root = temp_dir("stage");
        let oem = root.join("oem");
        let shared = root.join("shared");

        let files = stage_remoteapp_bootstrap(&oem, &shared).expect("bootstrap should stage");

        assert_eq!(files.install_bat, oem.join("install.bat"));
        assert_eq!(files.prepare_script, oem.join(REMOTEAPP_PREPARE_SCRIPT));
        assert_eq!(
            files.marker,
            shared
                .join(OFFICE_SHARE_DIR)
                .join(MARKERS_DIR)
                .join(REMOTEAPP_PREPARE_MARKER)
        );
        let install = std::fs::read_to_string(files.install_bat).expect("install.bat exists");
        let prepare = std::fs::read_to_string(files.prepare_script).expect("ps1 exists");
        assert_no_lf_without_cr(&install);
        assert_no_lf_without_cr(&prepare);
        assert!(shared.join(OFFICE_SHARE_DIR).join(SCRIPTS_DIR).is_dir());
        assert!(shared.join(OFFICE_SHARE_DIR).join(MARKERS_DIR).is_dir());
        assert!(shared.join(OFFICE_SHARE_DIR).join(LOGS_DIR).is_dir());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn probe_remoteapp_channel_maps_failures_to_actionable_code() {
        let executor = MockGuestExecutor::new();
        executor.fail_next_run("xfreerdp /app falhou");

        let err = probe_remoteapp_channel("office", &executor)
            .expect_err("RemoteApp failure should be actionable");
        let message = format!("{err:#}");

        assert!(message.contains(GUEST_REMOTEAPP_NOT_PREPARED_CODE));
        assert!(message.contains("C:\\OEM\\install.bat"));
    }

    #[test]
    fn guest_execution_is_detached_and_marker_driven() {
        let executor = MockGuestExecutor::new();
        executor.seed_marker("office_install.json", Some(office_done_marker()));
        let script = GuestScript {
            phase: "office_install".to_string(),
            file_name: "winbox-office-install.ps1".to_string(),
            contents: "Write-Host ok\r\n".to_string(),
        };

        let marker = start_detached_and_wait_for_marker(
            "office",
            &executor,
            script,
            GuestPhaseTimeout::immediate("office_install", "office_install.json"),
        )
        .expect("done marker should complete detached phase");

        assert_eq!(marker.status, GuestMarkerStatus::Done);
        assert_eq!(executor.detached_runs().len(), 1);
        assert_eq!(
            executor.detached_runs()[0].file_name,
            "winbox-office-install.ps1"
        );
        assert!(executor.runs().is_empty());

        let launcher =
            render_detached_launcher_script("office_install", "winbox-office-install.ps1")
                .expect("launcher should render");
        assert!(launcher.contains("schtasks.exe /Create"));
        assert!(launcher.contains("schtasks.exe /Run"));
        assert!(launcher.contains("winbox-office-install.ps1"));
        assert_no_lf_without_cr(&launcher);
    }

    #[test]
    fn office_install_timeout_returns_guest_phase_timeout() {
        let executor = MockGuestExecutor::new();
        assert!(GuestPhaseTimeout::office_install().timeout >= Duration::from_secs(60 * 60));
        let script = GuestScript {
            phase: "office_install".to_string(),
            file_name: "winbox-office-install.ps1".to_string(),
            contents: "Write-Host ok\r\n".to_string(),
        };

        let err = start_detached_and_wait_for_marker(
            "office",
            &executor,
            script,
            GuestPhaseTimeout::immediate("office_install", "office_install.json"),
        )
        .expect_err("missing marker should timeout");

        assert_eq!(err.code(), GUEST_PHASE_TIMEOUT_CODE);
        assert_eq!(err.phase(), "office_install");
    }

    #[test]
    fn office_ready_requires_clicktorun_and_three_exes() {
        let marker = office_done_marker();
        let verification =
            verify_office_install_marker(&marker).expect("complete marker should verify");

        assert_eq!(verification.exit_code, 0);
        assert_eq!(
            verification.office.platform.as_deref(),
            Some("x64"),
            "readiness records x64 ClickToRun platform"
        );

        let mut missing_registry = marker.clone();
        missing_registry
            .office
            .as_mut()
            .unwrap()
            .product_release_ids = None;
        let err = verify_office_install_marker(&missing_registry)
            .expect_err("missing registry should fail readiness");
        assert_eq!(err.code(), OFFICE_DETECTION_FAILED_CODE);

        let mut wrong_platform = marker.clone();
        wrong_platform.office.as_mut().unwrap().platform = Some("x86".to_string());
        let err = verify_office_install_marker(&wrong_platform)
            .expect_err("wrong platform should fail readiness");
        assert_eq!(err.code(), OFFICE_DETECTION_FAILED_CODE);

        let mut missing_exe = marker;
        missing_exe
            .office
            .as_mut()
            .unwrap()
            .exe_paths
            .as_mut()
            .unwrap()
            .excel = None;
        let err = verify_office_install_marker(&missing_exe)
            .expect_err("missing Excel executable should fail readiness");
        assert_eq!(err.code(), OFFICE_DETECTION_FAILED_CODE);
    }

    #[test]
    fn office_install_failed_marker_maps_specific_codes() {
        let disk = failed_office_marker(GUEST_DISK_FULL_CODE, "sem espaço no disco");
        let err = verify_office_install_marker(&disk).expect_err("disk marker should fail");
        assert_eq!(err.code(), GUEST_DISK_FULL_CODE);

        let odt = failed_office_marker(OFFICE_ODT_FAILED_CODE, "ODT retornou 1");
        let err = verify_office_install_marker(&odt).expect_err("odt marker should fail");
        assert_eq!(err.code(), OFFICE_ODT_FAILED_CODE);
    }

    fn assert_no_lf_without_cr(s: &str) {
        let bytes = s.as_bytes();
        for (idx, byte) in bytes.iter().enumerate() {
            if *byte == b'\n' {
                assert!(idx > 0 && bytes[idx - 1] == b'\r');
            }
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "winbox-guest-executor-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn office_done_marker() -> GuestMarker {
        GuestMarker {
            phase: "office_install".to_string(),
            status: GuestMarkerStatus::Done,
            exit_code: Some(0),
            error: None,
            office: Some(OfficeInstallEvidence {
                product_release_ids: Some("O365ProPlusRetail".to_string()),
                version_to_report: Some("16.0.12345.67890".to_string()),
                platform: Some("x64".to_string()),
                exe_paths: Some(OfficeExePaths {
                    excel: Some(
                        r"C:\Program Files\Microsoft Office\root\Office16\EXCEL.EXE".to_string(),
                    ),
                    winword: Some(
                        r"C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE".to_string(),
                    ),
                    powerpnt: Some(
                        r"C:\Program Files\Microsoft Office\root\Office16\POWERPNT.EXE".to_string(),
                    ),
                }),
            }),
            updated_at: Some("2026-07-08T00:00:00Z".to_string()),
        }
    }

    fn failed_office_marker(code: &str, message: &str) -> GuestMarker {
        GuestMarker {
            phase: "office_install".to_string(),
            status: GuestMarkerStatus::Failed,
            exit_code: Some(1),
            error: Some(GuestMarkerError {
                code: code.to_string(),
                message: message.to_string(),
                log_path: Some("logs/office_install.log".to_string()),
            }),
            office: None,
            updated_at: Some("2026-07-08T00:00:00Z".to_string()),
        }
    }
}

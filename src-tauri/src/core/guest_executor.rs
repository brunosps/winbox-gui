use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{env_file, paths};

pub const REMOTEAPP_PREPARE_MARKER: &str = "remoteapp_prepare.json";
pub const REMOTEAPP_PREPARE_SCRIPT: &str = "winbox-remoteapp-prepare.ps1";
pub const REMOTEAPP_NOOP_SCRIPT: &str = "winbox-remoteapp-noop.ps1";
pub const GUEST_REMOTEAPP_NOT_PREPARED_CODE: &str = "guest_remoteapp_not_prepared";
pub const REMOTEAPP_ACTION_HINT: &str =
    "Abra o desktop via noVNC e execute C:\\OEM\\install.bat; se necessário, aplique as chaves RemoteApp por sessão RDP full-desktop.";

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

fn guest_scripts_dir(shared_dir: &Path) -> PathBuf {
    shared_dir.join(OFFICE_SHARE_DIR).join(SCRIPTS_DIR)
}

fn guest_markers_dir(shared_dir: &Path) -> PathBuf {
    shared_dir.join(OFFICE_SHARE_DIR).join(MARKERS_DIR)
}

fn guest_logs_dir(shared_dir: &Path) -> PathBuf {
    shared_dir.join(OFFICE_SHARE_DIR).join(LOGS_DIR)
}

fn guest_script_unc(file_name: &str) -> String {
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
        fail_next_run: RefCell<Option<String>>,
        exit_code: RefCell<i32>,
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

        pub fn seed_exit_code(&self, code: i32) {
            *self.exit_code.borrow_mut() = code;
        }

        pub fn runs(&self) -> Vec<GuestScript> {
            self.runs.borrow().clone()
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

        fn read_marker(&self, _profile: &str, marker: &str) -> Result<Option<GuestMarker>> {
            Ok(self.markers.borrow().get(marker).cloned().unwrap_or(None))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock::MockGuestExecutor;
    use std::time::{SystemTime, UNIX_EPOCH};

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
}

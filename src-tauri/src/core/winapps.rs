use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    docker::DockerClient, env_file, flatpak::FREERDP_FLATPAK_COMMAND, launch_error::OfficeError,
    office_state::OfficePhase, paths,
};

pub const WINAPPS_REPO_URL: &str = "https://github.com/winapps-org/winapps.git";
pub const WINAPPS_PIN_COMMIT: &str = "5cbf7381f9a12af630e5a289d6dab5f7adc70e5d";
pub const WINAPPS_CONF_RELATIVE: &str = ".config/winapps/winapps.conf";
pub const WINAPPS_RDP_FLAGS: &str = "/cert:ignore +home-drive";
pub const WINAPPS_WAFLAVOR: &str = "manual";
pub const WINAPPS_RDP_IP: &str = "127.0.0.1";
pub const WINAPPS_PORT_TIMEOUT: u32 = 30;
pub const WINAPPS_RDP_TIMEOUT: u32 = 120;
pub const WINAPPS_APP_SCAN_TIMEOUT: u32 = 300;
pub const WINAPPS_BOOT_TIMEOUT: u32 = 600;
pub const OFFICE_LAUNCHERS: [&str; 3] = ["excel-o365", "word-o365", "powerpoint-o365"];
pub const OFFICE_OBSERVABLE_DESKTOP_FIELDS: [&str; 5] =
    ["Icon", "Name", "StartupWMClass", "Categories", "MimeType"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeDesktopApp {
    pub launcher: &'static str,
    pub app_id: &'static str,
    pub executable: &'static str,
}

pub const OFFICE_DESKTOP_APPS: [OfficeDesktopApp; 3] = [
    OfficeDesktopApp {
        launcher: "excel-o365",
        app_id: "excel",
        executable: r"C:\Program Files\Microsoft Office\root\Office16\EXCEL.EXE",
    },
    OfficeDesktopApp {
        launcher: "word-o365",
        app_id: "word",
        executable: r"C:\Program Files\Microsoft Office\root\Office16\WINWORD.EXE",
    },
    OfficeDesktopApp {
        launcher: "powerpoint-o365",
        app_id: "powerpoint",
        executable: r"C:\Program Files\Microsoft Office\root\Office16\POWERPNT.EXE",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeMimeAssociation {
    pub extension: &'static str,
    pub mime_type: &'static str,
    pub launcher: &'static str,
}

pub const OFFICE_MIME_ASSOCIATIONS: [OfficeMimeAssociation; 6] = [
    OfficeMimeAssociation {
        extension: ".xls",
        mime_type: "application/vnd.ms-excel",
        launcher: "excel-o365",
    },
    OfficeMimeAssociation {
        extension: ".xlsx",
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        launcher: "excel-o365",
    },
    OfficeMimeAssociation {
        extension: ".doc",
        mime_type: "application/msword",
        launcher: "word-o365",
    },
    OfficeMimeAssociation {
        extension: ".docx",
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        launcher: "word-o365",
    },
    OfficeMimeAssociation {
        extension: ".ppt",
        mime_type: "application/vnd.ms-powerpoint",
        launcher: "powerpoint-o365",
    },
    OfficeMimeAssociation {
        extension: ".pptx",
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        launcher: "powerpoint-o365",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WinAppsCommandOutput {
    pub success: bool,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait WinAppsClient {
    fn git(&self, cwd: Option<&Path>, args: &[String]) -> Result<WinAppsCommandOutput>;
    fn setup(&self, source_dir: &Path, args: &[String]) -> Result<WinAppsCommandOutput>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CliWinAppsClient;

impl WinAppsClient for CliWinAppsClient {
    fn git(&self, cwd: Option<&Path>, args: &[String]) -> Result<WinAppsCommandOutput> {
        let mut command = Command::new("git");
        command.args(args);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        command
            .output()
            .map(command_output)
            .with_context(|| format!("executando git {}", args.join(" ")))
    }

    fn setup(&self, source_dir: &Path, args: &[String]) -> Result<WinAppsCommandOutput> {
        let mut command = Command::new("bash");
        command.arg(source_dir.join("setup.sh")).args(args);
        command
            .current_dir(source_dir)
            .output()
            .map(command_output)
            .with_context(|| format!("executando WinApps setup em {}", source_dir.display()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinAppsPaths {
    pub source_dir: PathBuf,
    pub config_path: PathBuf,
}

impl WinAppsPaths {
    pub fn managed() -> Self {
        Self {
            source_dir: managed_source_dir(),
            config_path: winapps_conf_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinAppsConfig {
    pub rdp_user: String,
    pub rdp_pass: String,
    pub rdp_port: u16,
    pub freerdp_command: String,
    pub rdp_flags: String,
    pub port_timeout: u32,
    pub rdp_timeout: u32,
    pub app_scan_timeout: u32,
    pub boot_timeout: u32,
}

impl WinAppsConfig {
    pub fn from_env_map(map: &env_file::EnvMap) -> Result<Self> {
        let user = env_file::get(map, "USERNAME").trim();
        let pass = env_file::get(map, "PASSWORD");
        let rdp_port = env_file::get_u16(map, "RDP_PORT");
        if user.is_empty() {
            bail!("winapps_no_config: USERNAME ausente no config.env");
        }
        if pass.is_empty() {
            bail!("winapps_no_config: PASSWORD ausente no config.env");
        }
        if rdp_port == 0 {
            bail!("winapps_no_config: RDP_PORT ausente ou inválida no config.env");
        }
        Ok(Self {
            rdp_user: user.to_string(),
            rdp_pass: pass.to_string(),
            rdp_port,
            freerdp_command: FREERDP_FLATPAK_COMMAND.to_string(),
            rdp_flags: WINAPPS_RDP_FLAGS.to_string(),
            port_timeout: WINAPPS_PORT_TIMEOUT,
            rdp_timeout: WINAPPS_RDP_TIMEOUT,
            app_scan_timeout: WINAPPS_APP_SCAN_TIMEOUT,
            boot_timeout: WINAPPS_BOOT_TIMEOUT,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinAppsClone {
    pub source_dir: PathBuf,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WinAppsSetup {
    pub source_dir: PathBuf,
    pub config_path: PathBuf,
    pub commit: String,
    pub launchers: Vec<String>,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRegistration {
    pub launchers: Vec<String>,
    pub desktop_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAssociationRegistration {
    pub extensions: Vec<String>,
    pub mime_types: Vec<String>,
    pub desktop_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalVerifyReport {
    pub office_present: bool,
    pub rdp_ready: bool,
    pub winapps_ready: bool,
    pub launchers: Vec<String>,
    pub desktop_files: Vec<String>,
    pub mime_types: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionFindingKind {
    Container,
    Profile,
    WinappsConf,
    WinappsClone,
    DesktopEntry,
    RdpPort,
    OfficeInstall,
    OdtAsset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionFindingStatus {
    Compatible,
    Partial,
    Unsafe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionFinding {
    pub id: String,
    pub kind: AdoptionFindingKind,
    pub status: AdoptionFindingStatus,
    pub evidence: String,
    pub managed_by_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionManagedScope {
    pub manage_profile_config: bool,
    #[serde(rename = "manageWinAppsConf")]
    pub manage_winapps_conf: bool,
    pub manage_desktop_entries: bool,
    pub manage_file_associations: bool,
    pub manage_disk_lifecycle: bool,
    #[serde(rename = "preserveExistingWinAppsClone")]
    pub preserve_existing_winapps_clone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptionProbePaths {
    pub winapps_conf_path: PathBuf,
    pub winapps_clone_dir: PathBuf,
    pub applications_dir: PathBuf,
    pub profile_env_file: PathBuf,
}

impl AdoptionProbePaths {
    pub fn for_profile(profile: &str) -> Self {
        Self {
            winapps_conf_path: winapps_conf_path(),
            winapps_clone_dir: managed_source_dir(),
            applications_dir: desktop_applications_dir(),
            profile_env_file: paths::profile_env_file(profile),
        }
    }
}

pub fn managed_source_dir() -> PathBuf {
    paths::data_dir().join("winapps")
}

pub fn winapps_conf_path() -> PathBuf {
    paths::home().join(WINAPPS_CONF_RELATIVE)
}

pub fn desktop_applications_dir() -> PathBuf {
    paths::apps_dir()
}

pub fn mimeapps_path() -> PathBuf {
    #[cfg(windows)]
    {
        paths::config_dir().join("mimeapps.list")
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| paths::home().join(".config"))
            .join("mimeapps.list")
    }
}

pub fn detect_adoption_findings(
    profile: &str,
    docker: &dyn DockerClient,
    probe_paths: &AdoptionProbePaths,
) -> Vec<AdoptionFinding> {
    let mut findings = Vec::new();
    findings.extend(detect_container_findings(profile, docker));
    if probe_paths.winapps_conf_path.is_file() {
        findings.push(AdoptionFinding {
            id: "winapps_conf".to_string(),
            kind: AdoptionFindingKind::WinappsConf,
            status: AdoptionFindingStatus::Compatible,
            evidence: probe_paths.winapps_conf_path.display().to_string(),
            managed_by_default: false,
        });
    }
    if probe_paths.winapps_clone_dir.is_dir() {
        findings.push(AdoptionFinding {
            id: "winapps_clone".to_string(),
            kind: AdoptionFindingKind::WinappsClone,
            status: AdoptionFindingStatus::Compatible,
            evidence: probe_paths.winapps_clone_dir.display().to_string(),
            managed_by_default: false,
        });
    }
    if let Some(finding) = detect_desktop_entry_finding(&probe_paths.applications_dir) {
        findings.push(finding);
    }
    if let Some(finding) = detect_rdp_port_finding(&probe_paths.profile_env_file) {
        findings.push(finding);
    }
    findings
}

pub fn classify_adoption_managed_scope(findings: &[AdoptionFinding]) -> AdoptionManagedScope {
    let has_container = has_finding(findings, AdoptionFindingKind::Container);
    let has_profile = has_finding(findings, AdoptionFindingKind::RdpPort)
        || has_finding(findings, AdoptionFindingKind::Profile);
    let has_winapps_conf = has_finding(findings, AdoptionFindingKind::WinappsConf);
    let has_clone = has_finding(findings, AdoptionFindingKind::WinappsClone);
    let has_desktop = has_finding(findings, AdoptionFindingKind::DesktopEntry);

    AdoptionManagedScope {
        manage_profile_config: !has_profile,
        manage_winapps_conf: !has_winapps_conf,
        manage_desktop_entries: !has_desktop,
        manage_file_associations: !has_desktop,
        manage_disk_lifecycle: !has_container,
        preserve_existing_winapps_clone: has_clone,
    }
}

pub fn configure_winapps(
    config: &WinAppsConfig,
    paths: &WinAppsPaths,
    client: &dyn WinAppsClient,
) -> std::result::Result<WinAppsSetup, OfficeError> {
    let clone = ensure_pinned_clone(client, &paths.source_dir)?;
    write_winapps_conf(&paths.config_path, config).map_err(|err| {
        winapps_error(
            OfficeError::WINAPPS_NO_CONFIG,
            Some(serde_json::json!({
                "detail": format!("{err:#}"),
                "configPath": paths.config_path,
            })),
        )
    })?;
    setup_office_apps(client, &clone.source_dir, &paths.config_path)
}

pub fn ensure_pinned_clone(
    client: &dyn WinAppsClient,
    source_dir: &Path,
) -> std::result::Result<WinAppsClone, OfficeError> {
    if !source_dir.join(".git").is_dir() {
        std::fs::create_dir_all(source_dir.parent().unwrap_or_else(|| Path::new("."))).map_err(
            |err| {
                winapps_error(
                    OfficeError::WINAPPS_CLONE_FAILED,
                    Some(serde_json::json!({
                        "detail": err.to_string(),
                        "sourceDir": source_dir,
                    })),
                )
            },
        )?;
        run_git_checked(
            client,
            None,
            &[
                "clone",
                "--recurse-submodules",
                "--remote-submodules",
                WINAPPS_REPO_URL,
                &source_dir.to_string_lossy(),
            ],
        )?;
    } else {
        run_git_checked(client, Some(source_dir), &["fetch", "origin"])?;
    }

    run_git_checked(client, Some(source_dir), &["checkout", WINAPPS_PIN_COMMIT])?;
    run_git_checked(
        client,
        Some(source_dir),
        &["submodule", "update", "--init", "--recursive"],
    )?;
    let actual = current_commit(client, source_dir)?;
    if actual != WINAPPS_PIN_COMMIT {
        return Err(winapps_error(
            OfficeError::WINAPPS_PIN_MISMATCH,
            Some(serde_json::json!({
                "expected": WINAPPS_PIN_COMMIT,
                "actual": actual,
                "sourceDir": source_dir,
            })),
        ));
    }
    Ok(WinAppsClone {
        source_dir: source_dir.to_path_buf(),
        commit: actual,
    })
}

pub fn setup_office_apps(
    client: &dyn WinAppsClient,
    source_dir: &Path,
    config_path: &Path,
) -> std::result::Result<WinAppsSetup, OfficeError> {
    let output = client
        .setup(
            source_dir,
            &[
                "--user".to_string(),
                "--setupAllOfficiallySupportedApps".to_string(),
            ],
        )
        .map_err(|err| {
            winapps_error(
                OfficeError::WINAPPS_APP_SCAN_FAILED,
                Some(serde_json::json!({ "detail": format!("{err:#}") })),
            )
        })?;
    if !output.success {
        return Err(map_winapps_exit_output(&output));
    }
    let actual = current_commit(client, source_dir)?;
    if actual != WINAPPS_PIN_COMMIT {
        return Err(winapps_error(
            OfficeError::WINAPPS_PIN_MISMATCH,
            Some(serde_json::json!({
                "expected": WINAPPS_PIN_COMMIT,
                "actual": actual,
                "sourceDir": source_dir,
            })),
        ));
    }
    Ok(WinAppsSetup {
        source_dir: source_dir.to_path_buf(),
        config_path: config_path.to_path_buf(),
        commit: actual,
        launchers: OFFICE_LAUNCHERS
            .iter()
            .map(|launcher| launcher.to_string())
            .collect(),
        exit_code: output.status_code.unwrap_or(0),
    })
}

pub fn uninstall(
    client: &dyn WinAppsClient,
    source_dir: &Path,
) -> std::result::Result<(), OfficeError> {
    let output = client
        .setup(
            source_dir,
            &["--user".to_string(), "--uninstall".to_string()],
        )
        .map_err(|err| {
            winapps_error(
                OfficeError::WINAPPS_APP_SCAN_FAILED,
                Some(serde_json::json!({ "detail": format!("{err:#}") })),
            )
        })?;
    if output.success {
        Ok(())
    } else {
        Err(map_winapps_exit_output(&output))
    }
}

pub fn office_app_for_launcher(launcher: &str) -> Option<&'static str> {
    OFFICE_DESKTOP_APPS
        .iter()
        .find(|app| app.launcher == launcher)
        .map(|app| app.app_id)
}

pub fn desktop_exec_command(profile: &str, launcher: &str) -> Result<String> {
    let app_id = office_app_for_launcher(launcher)
        .with_context(|| format!("launcher Office desconhecido: {launcher}"))?;
    Ok(format!(
        "winbox office launch {profile} {app_id} --gui-progress -- %F"
    ))
}

pub fn patch_desktop_exec(content: &str, profile: &str, launcher: &str) -> Result<String> {
    let exec = desktop_exec_command(profile, launcher)?;
    let mut out = String::with_capacity(content.len() + exec.len());
    let mut found_exec = false;

    for raw_line in split_lines_preserving_newline(content) {
        let (line, newline) = raw_line;
        if line.starts_with("Exec=") {
            if found_exec {
                bail!("desktop possui mais de uma linha Exec");
            }
            out.push_str("Exec=");
            out.push_str(&exec);
            out.push_str(newline);
            found_exec = true;
        } else {
            out.push_str(line);
            out.push_str(newline);
        }
    }

    if !found_exec {
        bail!("desktop sem linha Exec");
    }
    Ok(out)
}

pub fn register_desktop_launchers(
    profile: &str,
    applications_dir: &Path,
) -> std::result::Result<DesktopRegistration, OfficeError> {
    for app in OFFICE_DESKTOP_APPS {
        let path = desktop_file_path(applications_dir, app.launcher);
        let content = std::fs::read_to_string(&path).map_err(|err| {
            desktop_registration_error(
                OfficePhase::DesktopRegistration,
                app.launcher,
                &path,
                format!("não foi possível ler o desktop: {err}"),
            )
        })?;
        let patched = patch_desktop_exec(&content, profile, app.launcher).map_err(|err| {
            desktop_registration_error(
                OfficePhase::DesktopRegistration,
                app.launcher,
                &path,
                format!("{err:#}"),
            )
        })?;
        if patched != content {
            std::fs::write(&path, patched).map_err(|err| {
                desktop_registration_error(
                    OfficePhase::DesktopRegistration,
                    app.launcher,
                    &path,
                    format!("não foi possível escrever o desktop: {err}"),
                )
            })?;
        }
    }
    verify_desktop_entries(profile, applications_dir)
}

pub fn verify_desktop_entries(
    profile: &str,
    applications_dir: &Path,
) -> std::result::Result<DesktopRegistration, OfficeError> {
    verify_desktop_entries_for_phase(profile, applications_dir, OfficePhase::DesktopRegistration)
}

pub fn render_mimeapps_list(existing: &str) -> String {
    let required = required_mime_defaults();
    let mut out = Vec::new();
    let mut in_default_applications = false;
    let mut saw_default_applications = false;
    let mut inserted_defaults = false;

    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_default_applications && !inserted_defaults {
                append_required_mime_defaults(&mut out);
                inserted_defaults = true;
            }
            in_default_applications = trimmed == "[Default Applications]";
            saw_default_applications |= in_default_applications;
            out.push(line.to_string());
            continue;
        }

        if in_default_applications {
            let key = line.split_once('=').map(|(key, _)| key.trim());
            if key.is_some_and(|key| required.contains_key(key)) {
                continue;
            }
        }
        out.push(line.to_string());
    }

    if in_default_applications && !inserted_defaults {
        append_required_mime_defaults(&mut out);
    }
    if !saw_default_applications {
        if out.last().is_some_and(|line| !line.is_empty()) {
            out.push(String::new());
        }
        out.push("[Default Applications]".to_string());
        append_required_mime_defaults(&mut out);
    }

    let mut rendered = out.join("\n");
    rendered.push('\n');
    rendered
}

pub fn register_mime_associations(
    mimeapps_path: &Path,
) -> std::result::Result<FileAssociationRegistration, OfficeError> {
    let existing = match std::fs::read_to_string(mimeapps_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => {
            return Err(file_association_error(
                OfficePhase::FileAssociation,
                format!("não foi possível ler {}: {err}", mimeapps_path.display()),
            ));
        }
    };
    let rendered = render_mimeapps_list(&existing);
    if let Some(parent) = mimeapps_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            file_association_error(
                OfficePhase::FileAssociation,
                format!("não foi possível criar {}: {err}", parent.display()),
            )
        })?;
    }
    std::fs::write(mimeapps_path, &rendered).map_err(|err| {
        file_association_error(
            OfficePhase::FileAssociation,
            format!(
                "não foi possível escrever {}: {err}",
                mimeapps_path.display()
            ),
        )
    })?;
    verify_mime_associations_content(&rendered)
}

pub fn verify_mime_associations_content(
    content: &str,
) -> std::result::Result<FileAssociationRegistration, OfficeError> {
    verify_mime_associations_content_for_phase(content, OfficePhase::FileAssociation)
}

pub fn final_verify(
    state: &super::office_state::OfficeProvisioningState,
    applications_dir: &Path,
    mimeapps_path: &Path,
) -> std::result::Result<FinalVerifyReport, OfficeError> {
    ensure_office_install_evidence(state)?;
    ensure_phase_done(
        state,
        OfficePhase::RemoteappPrepare,
        OfficeError::GUEST_REMOTEAPP_NOT_PREPARED,
    )?;
    ensure_phase_done(
        state,
        OfficePhase::WinappsConfig,
        OfficeError::APP_NOT_REGISTERED,
    )?;
    ensure_winapps_launchers(state)?;

    let desktop = verify_desktop_entries_for_phase(
        &state.profile,
        applications_dir,
        OfficePhase::FinalVerify,
    )?;
    let mime_content = std::fs::read_to_string(mimeapps_path).map_err(|err| {
        file_association_error(
            OfficePhase::FinalVerify,
            format!("não foi possível ler {}: {err}", mimeapps_path.display()),
        )
    })?;
    let mime = verify_mime_associations_content_for_phase(&mime_content, OfficePhase::FinalVerify)?;

    Ok(FinalVerifyReport {
        office_present: true,
        rdp_ready: true,
        winapps_ready: true,
        launchers: desktop.launchers,
        desktop_files: desktop.desktop_files,
        mime_types: mime.mime_types,
    })
}

pub fn render_winapps_conf(config: &WinAppsConfig) -> Result<String> {
    let entries = [
        ("RDP_USER", shell_single_quote(&config.rdp_user)?),
        ("RDP_PASS", shell_single_quote(&config.rdp_pass)?),
        ("RDP_ASKPASS", shell_single_quote("")?),
        ("RDP_DOMAIN", shell_single_quote("")?),
        ("RDP_IP", shell_single_quote(WINAPPS_RDP_IP)?),
        (
            "RDP_PORT",
            shell_single_quote(&config.rdp_port.to_string())?,
        ),
        ("WAFLAVOR", shell_single_quote(WINAPPS_WAFLAVOR)?),
        (
            "FREERDP_COMMAND",
            shell_single_quote(&config.freerdp_command)?,
        ),
        ("RDP_FLAGS", shell_single_quote(&config.rdp_flags)?),
        (
            "PORT_TIMEOUT",
            shell_single_quote(&config.port_timeout.to_string())?,
        ),
        (
            "RDP_TIMEOUT",
            shell_single_quote(&config.rdp_timeout.to_string())?,
        ),
        (
            "APP_SCAN_TIMEOUT",
            shell_single_quote(&config.app_scan_timeout.to_string())?,
        ),
        (
            "BOOT_TIMEOUT",
            shell_single_quote(&config.boot_timeout.to_string())?,
        ),
        ("DEBUG", shell_single_quote("true")?),
        ("AUTOPAUSE", shell_single_quote("off")?),
        ("HIDEF", shell_single_quote("on")?),
    ];
    let mut out =
        String::from("# Generated by winbox. Do not edit while Office provisioning is running.\n");
    for (key, value) in entries {
        out.push_str(key);
        out.push('=');
        out.push_str(&value);
        out.push('\n');
    }
    Ok(out)
}

pub fn map_winapps_exit_code(exit_code: i32) -> OfficeError {
    map_winapps_exit_output(&WinAppsCommandOutput {
        success: false,
        status_code: Some(exit_code),
        stdout: String::new(),
        stderr: String::new(),
    })
}

pub fn winapps_error_message(code: &str) -> &'static str {
    match code {
        OfficeError::WINAPPS_CLONE_FAILED => "Falha ao clonar ou atualizar WinApps upstream.",
        OfficeError::WINAPPS_NO_CONFIG => "Configuração WinApps ausente ou inválida.",
        OfficeError::WINAPPS_MISSING_DEPS => "Dependências do WinApps estão ausentes no host.",
        OfficeError::WINAPPS_BAD_PORT => "Porta RDP configurada para WinApps não respondeu.",
        OfficeError::WINAPPS_RDP_FAILED => "WinApps não conseguiu autenticar via RDP.",
        OfficeError::WINAPPS_APP_SCAN_FAILED => "WinApps não conseguiu detectar apps instalados.",
        OfficeError::WINAPPS_PIN_MISMATCH => "WinApps não permaneceu no commit pinado.",
        OfficeError::DESKTOP_REGISTRATION_FAILED => {
            "Falha ao registrar atalhos .desktop do Office."
        }
        OfficeError::FILE_ASSOCIATION_FAILED => {
            "Falha ao registrar associações de arquivo do Office."
        }
        OfficeError::APP_NOT_REGISTERED => "WinApps não registrou os launchers Office esperados.",
        OfficeError::OFFICE_DETECTION_FAILED => {
            "Office não passou na verificação ClickToRun e executáveis."
        }
        OfficeError::GUEST_REMOTEAPP_NOT_PREPARED => "RemoteApp/RDP não está preparado.",
        _ => "Falha na configuração WinApps.",
    }
}

fn verify_desktop_entries_for_phase(
    profile: &str,
    applications_dir: &Path,
    phase: OfficePhase,
) -> std::result::Result<DesktopRegistration, OfficeError> {
    let mut launchers = Vec::with_capacity(OFFICE_DESKTOP_APPS.len());
    let mut desktop_files = Vec::with_capacity(OFFICE_DESKTOP_APPS.len());

    for app in OFFICE_DESKTOP_APPS {
        let path = desktop_file_path(applications_dir, app.launcher);
        let content = std::fs::read_to_string(&path).map_err(|err| {
            desktop_registration_error(
                phase,
                app.launcher,
                &path,
                format!("não foi possível ler o desktop: {err}"),
            )
        })?;
        let missing_fields = missing_observable_desktop_fields(&content);
        if !missing_fields.is_empty() {
            return Err(desktop_registration_error(
                phase,
                app.launcher,
                &path,
                format!("campos observáveis ausentes: {}", missing_fields.join(", ")),
            ));
        }
        if !desktop_has_wrapper_exec(&content, profile, app.launcher).map_err(|err| {
            desktop_registration_error(phase, app.launcher, &path, format!("{err:#}"))
        })? {
            return Err(desktop_registration_error(
                phase,
                app.launcher,
                &path,
                "Exec não aponta para o wrapper winbox office launch".to_string(),
            ));
        }
        launchers.push(app.launcher.to_string());
        desktop_files.push(path.display().to_string());
    }

    Ok(DesktopRegistration {
        launchers,
        desktop_files,
    })
}

fn verify_mime_associations_content_for_phase(
    content: &str,
    phase: OfficePhase,
) -> std::result::Result<FileAssociationRegistration, OfficeError> {
    let defaults = parse_mime_defaults(content);
    let mut extensions = Vec::with_capacity(OFFICE_MIME_ASSOCIATIONS.len());
    let mut mime_types = Vec::with_capacity(OFFICE_MIME_ASSOCIATIONS.len());
    let mut desktop_files = Vec::with_capacity(OFFICE_MIME_ASSOCIATIONS.len());

    for association in OFFICE_MIME_ASSOCIATIONS {
        let desktop_id = desktop_id(association.launcher);
        let registered = defaults
            .get(association.mime_type)
            .map(|value| {
                value
                    .split(';')
                    .map(str::trim)
                    .any(|entry| entry == desktop_id)
            })
            .unwrap_or(false);
        if !registered {
            return Err(file_association_error(
                phase,
                format!(
                    "{} precisa apontar para {}",
                    association.mime_type, desktop_id
                ),
            ));
        }
        extensions.push(association.extension.to_string());
        mime_types.push(association.mime_type.to_string());
        desktop_files.push(desktop_id.to_string());
    }

    Ok(FileAssociationRegistration {
        extensions,
        mime_types,
        desktop_files,
    })
}

fn ensure_phase_done(
    state: &super::office_state::OfficeProvisioningState,
    phase: OfficePhase,
    code: &str,
) -> std::result::Result<(), OfficeError> {
    if state.phase_status(phase) == Some(super::office_state::PhaseStatus::Done) {
        return Ok(());
    }
    Err(OfficeError::new(
        code,
        OfficePhase::FinalVerify,
        true,
        Some(serde_json::json!({
            "requiredPhase": phase,
            "status": state.phase_status(phase),
        })),
    ))
}

fn ensure_winapps_launchers(
    state: &super::office_state::OfficeProvisioningState,
) -> std::result::Result<(), OfficeError> {
    let launchers = state
        .phases
        .get(&OfficePhase::WinappsConfig)
        .and_then(|phase| phase.evidence.as_ref())
        .map(|evidence| evidence.launcher_ids.as_slice())
        .unwrap_or(&[]);
    for expected in OFFICE_LAUNCHERS {
        if !launchers.iter().any(|launcher| launcher == expected) {
            return Err(OfficeError::new(
                OfficeError::APP_NOT_REGISTERED,
                OfficePhase::FinalVerify,
                true,
                Some(serde_json::json!({
                    "launcher": expected,
                    "registeredLaunchers": launchers,
                })),
            ));
        }
    }
    Ok(())
}

fn ensure_office_install_evidence(
    state: &super::office_state::OfficeProvisioningState,
) -> std::result::Result<(), OfficeError> {
    ensure_phase_done(
        state,
        OfficePhase::OfficeInstall,
        OfficeError::OFFICE_DETECTION_FAILED,
    )?;
    let evidence = state
        .phases
        .get(&OfficePhase::OfficeInstall)
        .and_then(|phase| phase.evidence.as_ref());
    let has_registry = evidence
        .and_then(|evidence| evidence.registry.as_ref())
        .is_some();
    let files = evidence
        .map(|evidence| evidence.files.as_slice())
        .unwrap_or(&[]);
    let has_excel = files.iter().any(|file| file.ends_with("EXCEL.EXE"));
    let has_word = files.iter().any(|file| file.ends_with("WINWORD.EXE"));
    let has_powerpoint = files.iter().any(|file| file.ends_with("POWERPNT.EXE"));

    if has_registry && has_excel && has_word && has_powerpoint {
        return Ok(());
    }
    Err(OfficeError::new(
        OfficeError::OFFICE_DETECTION_FAILED,
        OfficePhase::FinalVerify,
        true,
        Some(serde_json::json!({
            "detail": "office_install não tem evidência completa de registry ClickToRun e executáveis x64.",
            "hasRegistry": has_registry,
            "hasExcel": has_excel,
            "hasWord": has_word,
            "hasPowerPoint": has_powerpoint,
        })),
    ))
}

fn desktop_has_wrapper_exec(content: &str, profile: &str, launcher: &str) -> Result<bool> {
    let expected = format!("Exec={}", desktop_exec_command(profile, launcher)?);
    Ok(content
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .any(|line| line == expected))
}

fn missing_observable_desktop_fields(content: &str) -> Vec<&'static str> {
    OFFICE_OBSERVABLE_DESKTOP_FIELDS
        .into_iter()
        .filter(|field| {
            desktop_field(content, field)
                .map(str::trim)
                .map_or(true, str::is_empty)
        })
        .collect()
}

fn desktop_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    content
        .lines()
        .find_map(|line| line.trim_end_matches('\r').strip_prefix(&prefix))
}

fn split_lines_preserving_newline(content: &str) -> Vec<(&str, &str)> {
    if content.is_empty() {
        return Vec::new();
    }
    content
        .split_inclusive('\n')
        .map(|segment| {
            if let Some(line) = segment.strip_suffix("\r\n") {
                (line, "\r\n")
            } else if let Some(line) = segment.strip_suffix('\n') {
                (line, "\n")
            } else {
                (segment, "")
            }
        })
        .collect()
}

fn desktop_file_path(applications_dir: &Path, launcher: &str) -> PathBuf {
    applications_dir.join(desktop_id(launcher))
}

fn desktop_id(launcher: &str) -> String {
    format!("{launcher}.desktop")
}

fn append_required_mime_defaults(out: &mut Vec<String>) {
    for association in OFFICE_MIME_ASSOCIATIONS {
        out.push(format!(
            "{}={}",
            association.mime_type,
            desktop_id(association.launcher)
        ));
    }
}

fn required_mime_defaults() -> BTreeMap<&'static str, String> {
    OFFICE_MIME_ASSOCIATIONS
        .into_iter()
        .map(|association| (association.mime_type, desktop_id(association.launcher)))
        .collect()
}

fn parse_mime_defaults(content: &str) -> BTreeMap<String, String> {
    let mut defaults = BTreeMap::new();
    let mut in_default_applications = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_default_applications = trimmed == "[Default Applications]";
            continue;
        }
        if !in_default_applications || trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            defaults.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    defaults
}

fn desktop_registration_error(
    phase: OfficePhase,
    launcher: &str,
    path: &Path,
    detail: String,
) -> OfficeError {
    OfficeError::new(
        OfficeError::DESKTOP_REGISTRATION_FAILED,
        phase,
        true,
        Some(serde_json::json!({
            "launcher": launcher,
            "desktopPath": path,
            "detail": detail,
        })),
    )
}

fn file_association_error(phase: OfficePhase, detail: String) -> OfficeError {
    OfficeError::new(
        OfficeError::FILE_ASSOCIATION_FAILED,
        phase,
        true,
        Some(serde_json::json!({
            "detail": detail,
            "mimeTypes": OFFICE_MIME_ASSOCIATIONS
                .iter()
                .map(|association| association.mime_type)
                .collect::<Vec<_>>(),
        })),
    )
}

fn detect_container_findings(profile: &str, docker: &dyn DockerClient) -> Vec<AdoptionFinding> {
    let candidates = ["winbox-windows".to_string(), format!("winbox-{profile}")];
    candidates
        .into_iter()
        .filter_map(|container| {
            let status = docker.container_status(&container);
            if status == "absent" {
                return None;
            }
            Some(AdoptionFinding {
                id: format!("container:{container}"),
                kind: AdoptionFindingKind::Container,
                status: adoption_status_from_container(&status),
                evidence: format!("{container} status={status}"),
                managed_by_default: false,
            })
        })
        .collect()
}

fn detect_desktop_entry_finding(applications_dir: &Path) -> Option<AdoptionFinding> {
    let found = OFFICE_LAUNCHERS
        .iter()
        .filter(|launcher| {
            applications_dir
                .join(format!("{launcher}.desktop"))
                .is_file()
        })
        .map(|launcher| (*launcher).to_string())
        .collect::<Vec<_>>();
    if found.is_empty() {
        return None;
    }
    let status = if found.len() == OFFICE_LAUNCHERS.len() {
        AdoptionFindingStatus::Compatible
    } else {
        AdoptionFindingStatus::Partial
    };
    Some(AdoptionFinding {
        id: "desktop_entries:office".to_string(),
        kind: AdoptionFindingKind::DesktopEntry,
        status,
        evidence: format!("{} em {}", found.join(","), applications_dir.display()),
        managed_by_default: false,
    })
}

fn detect_rdp_port_finding(profile_env_file: &Path) -> Option<AdoptionFinding> {
    let env = env_file::read(profile_env_file).ok()?;
    let rdp_port = env_file::get_u16(&env, "RDP_PORT");
    if rdp_port == 0 {
        return None;
    }
    Some(AdoptionFinding {
        id: format!("rdp_port:{rdp_port}"),
        kind: AdoptionFindingKind::RdpPort,
        status: AdoptionFindingStatus::Compatible,
        evidence: format!("{} publica RDP_PORT={rdp_port}", profile_env_file.display()),
        managed_by_default: true,
    })
}

fn adoption_status_from_container(status: &str) -> AdoptionFindingStatus {
    match status {
        "running" | "exited" | "paused" | "created" => AdoptionFindingStatus::Compatible,
        _ => AdoptionFindingStatus::Partial,
    }
}

fn has_finding(findings: &[AdoptionFinding], kind: AdoptionFindingKind) -> bool {
    findings.iter().any(|finding| finding.kind == kind)
}

fn write_winapps_conf(path: &Path, config: &WinAppsConfig) -> Result<()> {
    let content = render_winapps_conf(config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("criando {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("escrevendo {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("chmod 600 {}", path.display()))?;
    }
    Ok(())
}

fn run_git_checked(
    client: &dyn WinAppsClient,
    cwd: Option<&Path>,
    args: &[&str],
) -> std::result::Result<WinAppsCommandOutput, OfficeError> {
    let owned = args
        .iter()
        .map(|arg| (*arg).to_string())
        .collect::<Vec<_>>();
    let output = client.git(cwd, &owned).map_err(|err| {
        winapps_error(
            OfficeError::WINAPPS_CLONE_FAILED,
            Some(serde_json::json!({
                "detail": format!("{err:#}"),
                "args": owned,
            })),
        )
    })?;
    if output.success {
        Ok(output)
    } else {
        Err(winapps_error(
            OfficeError::WINAPPS_CLONE_FAILED,
            Some(serde_json::json!({
                "args": owned,
                "exitCode": output.status_code,
                "stderr": output.stderr,
            })),
        ))
    }
}

fn current_commit(
    client: &dyn WinAppsClient,
    source_dir: &Path,
) -> std::result::Result<String, OfficeError> {
    let output = run_git_checked(client, Some(source_dir), &["rev-parse", "HEAD"])?;
    Ok(output.stdout.trim().to_string())
}

fn map_winapps_exit_output(output: &WinAppsCommandOutput) -> OfficeError {
    let code = match output.status_code.unwrap_or(-1) {
        4 => OfficeError::WINAPPS_NO_CONFIG,
        5 => OfficeError::WINAPPS_MISSING_DEPS,
        13 => OfficeError::WINAPPS_BAD_PORT,
        14 => OfficeError::WINAPPS_RDP_FAILED,
        15 => OfficeError::WINAPPS_APP_SCAN_FAILED,
        _ => OfficeError::WINAPPS_APP_SCAN_FAILED,
    };
    winapps_error(
        code,
        Some(serde_json::json!({
            "exitCode": output.status_code,
            "stdout": output.stdout,
            "stderr": output.stderr,
        })),
    )
}

fn winapps_error(code: &str, details: Option<serde_json::Value>) -> OfficeError {
    OfficeError::new(code, OfficePhase::WinappsConfig, true, details)
}

// WinApps sources winapps.conf as bash. Single-quote every scalar and escape
// embedded single quotes with the POSIX '\'' pattern; reject line breaks so one
// env value cannot create another assignment.
fn shell_single_quote(value: &str) -> Result<String> {
    if value.contains(['\n', '\r', '\0']) {
        bail!("valor contém quebra de linha ou NUL e não pode entrar no winapps.conf");
    }
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

fn command_output(output: std::process::Output) -> WinAppsCommandOutput {
    WinAppsCommandOutput {
        success: output.status.success(),
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[cfg(test)]
pub mod mock {
    use super::{WinAppsClient, WinAppsCommandOutput};
    use anyhow::Result;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::path::Path;

    #[derive(Default)]
    pub struct MockWinAppsClient {
        git_outputs: RefCell<VecDeque<WinAppsCommandOutput>>,
        setup_outputs: RefCell<VecDeque<WinAppsCommandOutput>>,
        calls: RefCell<Vec<String>>,
    }

    impl MockWinAppsClient {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn push_git(&self, output: WinAppsCommandOutput) {
            self.git_outputs.borrow_mut().push_back(output);
        }

        pub fn push_setup(&self, output: WinAppsCommandOutput) {
            self.setup_outputs.borrow_mut().push_back(output);
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }

        fn record(&self, call: impl Into<String>) {
            self.calls.borrow_mut().push(call.into());
        }
    }

    impl WinAppsClient for MockWinAppsClient {
        fn git(&self, _cwd: Option<&Path>, args: &[String]) -> Result<WinAppsCommandOutput> {
            self.record(format!("git:{}", args.join(" ")));
            self.git_outputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("git output não configurado"))
        }

        fn setup(&self, _source_dir: &Path, args: &[String]) -> Result<WinAppsCommandOutput> {
            self.record(format!("setup:{}", args.join(" ")));
            self.setup_outputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| anyhow::anyhow!("setup output não configurado"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::docker::mock::MockDocker;
    use crate::core::office_state::{OfficeProvisioningState, PhaseEvidence};
    use mock::MockWinAppsClient;
    use std::collections::BTreeSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn winapps_conf_quotes_rdp_credentials_for_bash_source() {
        let config = sample_config("bruno", "pa'$$ word");
        let rendered = render_winapps_conf(&config).expect("config should render");

        assert!(rendered.contains("RDP_USER='bruno'\n"));
        assert!(rendered.contains("RDP_PASS='pa'\\''$$ word'\n"));
        assert!(rendered.contains("WAFLAVOR='manual'\n"));
        assert!(rendered
            .contains("FREERDP_COMMAND='flatpak run --command=xfreerdp com.freerdp.FreeRDP'\n"));
        assert!(rendered.contains("RDP_FLAGS='/cert:ignore +home-drive'\n"));
        assert!(render_winapps_conf(&sample_config("bruno", "line\nbreak")).is_err());
    }

    #[test]
    fn winapps_exit_code_maps_to_office_error() {
        let cases = [
            (4, OfficeError::WINAPPS_NO_CONFIG),
            (5, OfficeError::WINAPPS_MISSING_DEPS),
            (13, OfficeError::WINAPPS_BAD_PORT),
            (14, OfficeError::WINAPPS_RDP_FAILED),
            (15, OfficeError::WINAPPS_APP_SCAN_FAILED),
        ];

        for (exit_code, expected_code) in cases {
            let error = map_winapps_exit_code(exit_code);
            assert_eq!(error.code(), expected_code);
            assert_eq!(error.fields().phase, OfficePhase::WinappsConfig);
            assert!(error.fields().retryable);
        }
    }

    #[test]
    fn winapps_clone_pin_mismatch_is_reported() {
        let root = temp_dir("pin-mismatch");
        let source_dir = root.join("winapps");
        std::fs::create_dir_all(source_dir.join(".git")).expect("git dir should exist");
        let client = MockWinAppsClient::new();
        client.push_git(ok(""));
        client.push_git(ok(""));
        client.push_git(ok(""));
        client.push_git(ok("deadbeef\n"));

        let error =
            ensure_pinned_clone(&client, &source_dir).expect_err("wrong commit should be rejected");

        assert_eq!(error.code(), OfficeError::WINAPPS_PIN_MISMATCH);
        assert_eq!(
            error
                .fields()
                .details
                .as_ref()
                .and_then(|d| d.get("actual")),
            Some(&serde_json::json!("deadbeef"))
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn winapps_clone_failure_maps_to_office_error() {
        let root = temp_dir("clone-failed");
        let source_dir = root.join("winapps");
        let client = MockWinAppsClient::new();
        client.push_git(fail(128, "network down"));

        let error = ensure_pinned_clone(&client, &source_dir)
            .expect_err("git clone failure should be actionable");

        assert_eq!(error.code(), OfficeError::WINAPPS_CLONE_FAILED);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn desktop_patch_preserves_observable_fields() {
        let original = desktop_content("excel-o365", "Microsoft Excel");

        let patched =
            patch_desktop_exec(&original, "office", "excel-o365").expect("patch should pass");

        assert!(patched.contains("Exec=winbox office launch office excel --gui-progress -- %F\n"));
        for field in OFFICE_OBSERVABLE_DESKTOP_FIELDS {
            assert_eq!(
                desktop_field(&patched, field),
                desktop_field(&original, field),
                "{field} should be preserved"
            );
        }
        let original_non_exec = original
            .lines()
            .filter(|line| !line.starts_with("Exec="))
            .collect::<Vec<_>>();
        let patched_non_exec = patched
            .lines()
            .filter(|line| !line.starts_with("Exec="))
            .collect::<Vec<_>>();
        assert_eq!(patched_non_exec, original_non_exec);

        let malformed = "[Desktop Entry]\nName=Microsoft Excel\n";
        assert!(patch_desktop_exec(malformed, "office", "excel-o365").is_err());
    }

    #[test]
    fn mime_registration_maps_all_required_extensions() {
        let rendered = render_mimeapps_list("[Added Associations]\ntext/plain=code.desktop;\n");
        let registration =
            verify_mime_associations_content(&rendered).expect("required MIME entries should pass");

        assert_eq!(
            registration.extensions,
            vec![".xls", ".xlsx", ".doc", ".docx", ".ppt", ".pptx"]
        );
        assert!(rendered.contains("application/vnd.ms-excel=excel-o365.desktop\n"));
        assert!(rendered.contains(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document=word-o365.desktop\n"
        ));
        assert!(rendered.contains(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation=powerpoint-o365.desktop\n"
        ));
        assert_eq!(render_mimeapps_list(&rendered), rendered);

        let err = verify_mime_associations_content("[Default Applications]\n")
            .expect_err("missing Office MIME mappings should fail");
        assert_eq!(err.code(), OfficeError::FILE_ASSOCIATION_FAILED);
    }

    #[test]
    fn final_verify_requires_desktop_and_mime_entries() {
        let root = temp_dir("final-verify");
        let applications_dir = root.join("applications");
        let mimeapps = root.join("config").join("mimeapps.list");
        std::fs::create_dir_all(&applications_dir).expect("applications dir should exist");
        seed_office_desktops(&applications_dir);
        register_desktop_launchers("office", &applications_dir)
            .expect("desktop registration should patch upstream entries");
        let state = ready_for_final_verify_state();

        let err = final_verify(&state, &applications_dir, &mimeapps)
            .expect_err("missing mimeapps.list should block final verify");
        assert_eq!(err.code(), OfficeError::FILE_ASSOCIATION_FAILED);
        assert_eq!(err.fields().phase, OfficePhase::FinalVerify);

        std::fs::create_dir_all(mimeapps.parent().expect("mimeapps parent should exist"))
            .expect("mimeapps parent should be created");
        std::fs::write(&mimeapps, render_mimeapps_list("")).expect("mimeapps should be written");
        let report =
            final_verify(&state, &applications_dir, &mimeapps).expect("all evidence should pass");

        assert!(report.office_present);
        assert!(report.rdp_ready);
        assert!(report.winapps_ready);
        assert_eq!(report.launchers, OFFICE_LAUNCHERS);
        assert_eq!(report.desktop_files.len(), 3);
        assert_eq!(report.mime_types.len(), 6);

        let mut missing_launcher = state;
        missing_launcher
            .phases
            .get_mut(&OfficePhase::WinappsConfig)
            .and_then(|phase| phase.evidence.as_mut())
            .expect("winapps evidence should exist")
            .launcher_ids = vec!["excel-o365".to_string()];
        let err = final_verify(&missing_launcher, &applications_dir, &mimeapps)
            .expect_err("missing WinApps launchers should block final verify");
        assert_eq!(err.code(), OfficeError::APP_NOT_REGISTERED);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn adoption_detects_all_required_signals() {
        let root = temp_dir("adoption-signals");
        let applications_dir = root.join("applications");
        let winapps_conf = root.join("config").join("winapps.conf");
        let clone_dir = root.join("winapps-src");
        let profile_env = root.join("profile").join("config.env");
        std::fs::create_dir_all(winapps_conf.parent().expect("conf parent should exist"))
            .expect("conf parent should be created");
        std::fs::write(&winapps_conf, "RDP_PORT='3391'\n").expect("conf should be written");
        std::fs::create_dir_all(&clone_dir).expect("clone dir should exist");
        std::fs::create_dir_all(&applications_dir).expect("applications dir should exist");
        seed_office_desktops(&applications_dir);
        std::fs::create_dir_all(profile_env.parent().expect("profile parent should exist"))
            .expect("profile parent should be created");
        std::fs::write(&profile_env, "PROFILE_KIND=office\nRDP_PORT=3391\n")
            .expect("profile env should be written");
        let paths = AdoptionProbePaths {
            winapps_conf_path: winapps_conf,
            winapps_clone_dir: clone_dir,
            applications_dir,
            profile_env_file: profile_env,
        };
        let docker = MockDocker::new();
        docker.seed_status("winbox-windows", "running");

        let findings = detect_adoption_findings("office", &docker, &paths);
        let kinds = findings
            .iter()
            .map(|finding| finding.kind)
            .collect::<BTreeSet<_>>();

        assert!(kinds.contains(&AdoptionFindingKind::Container));
        assert!(kinds.contains(&AdoptionFindingKind::WinappsConf));
        assert!(kinds.contains(&AdoptionFindingKind::WinappsClone));
        assert!(kinds.contains(&AdoptionFindingKind::DesktopEntry));
        assert!(kinds.contains(&AdoptionFindingKind::RdpPort));
        assert!(findings.iter().any(|finding| finding.id == "rdp_port:3391"));

        let empty_paths = AdoptionProbePaths {
            winapps_conf_path: root.join("missing.conf"),
            winapps_clone_dir: root.join("missing-clone"),
            applications_dir: root.join("missing-applications"),
            profile_env_file: root.join("missing.env"),
        };
        let empty = detect_adoption_findings("empty", &MockDocker::new(), &empty_paths);
        assert!(empty.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn managed_scope_prevents_overwriting_user_assets() {
        let findings = vec![
            AdoptionFinding {
                id: "winapps_conf".to_string(),
                kind: AdoptionFindingKind::WinappsConf,
                status: AdoptionFindingStatus::Compatible,
                evidence: "~/.config/winapps/winapps.conf".to_string(),
                managed_by_default: false,
            },
            AdoptionFinding {
                id: "winapps_clone".to_string(),
                kind: AdoptionFindingKind::WinappsClone,
                status: AdoptionFindingStatus::Compatible,
                evidence: "~/.local/bin/winapps-src".to_string(),
                managed_by_default: false,
            },
            AdoptionFinding {
                id: "desktop_entries:office".to_string(),
                kind: AdoptionFindingKind::DesktopEntry,
                status: AdoptionFindingStatus::Compatible,
                evidence: "excel-o365,word-o365,powerpoint-o365".to_string(),
                managed_by_default: false,
            },
            AdoptionFinding {
                id: "container:winbox-windows".to_string(),
                kind: AdoptionFindingKind::Container,
                status: AdoptionFindingStatus::Compatible,
                evidence: "winbox-windows status=running".to_string(),
                managed_by_default: false,
            },
        ];

        let scope = classify_adoption_managed_scope(&findings);

        assert!(!scope.manage_winapps_conf);
        assert!(!scope.manage_desktop_entries);
        assert!(!scope.manage_file_associations);
        assert!(!scope.manage_disk_lifecycle);
        assert!(scope.preserve_existing_winapps_clone);

        let empty_scope = classify_adoption_managed_scope(&[]);
        assert!(empty_scope.manage_profile_config);
        assert!(empty_scope.manage_winapps_conf);
        assert!(empty_scope.manage_desktop_entries);
        assert!(empty_scope.manage_file_associations);
        assert!(empty_scope.manage_disk_lifecycle);
        assert!(!empty_scope.preserve_existing_winapps_clone);
    }

    fn sample_config(user: &str, pass: &str) -> WinAppsConfig {
        WinAppsConfig {
            rdp_user: user.to_string(),
            rdp_pass: pass.to_string(),
            rdp_port: 3390,
            freerdp_command: FREERDP_FLATPAK_COMMAND.to_string(),
            rdp_flags: WINAPPS_RDP_FLAGS.to_string(),
            port_timeout: WINAPPS_PORT_TIMEOUT,
            rdp_timeout: WINAPPS_RDP_TIMEOUT,
            app_scan_timeout: WINAPPS_APP_SCAN_TIMEOUT,
            boot_timeout: WINAPPS_BOOT_TIMEOUT,
        }
    }

    fn ok(stdout: &str) -> WinAppsCommandOutput {
        WinAppsCommandOutput {
            success: true,
            status_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn fail(code: i32, stderr: &str) -> WinAppsCommandOutput {
        WinAppsCommandOutput {
            success: false,
            status_code: Some(code),
            stdout: String::new(),
            stderr: stderr.to_string(),
        }
    }

    fn desktop_content(launcher: &str, name: &str) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={name}\n\
             Exec=/home/bruno/.local/bin/winapps {launcher} %F\n\
             Icon=/home/bruno/.local/share/winapps/apps/{launcher}/icon.svg\n\
             StartupWMClass={name}\n\
             Categories=WinApps;Office;\n\
             MimeType=application/x-winbox-test;\n"
        )
    }

    fn seed_office_desktops(applications_dir: &Path) {
        let names = [
            ("excel-o365", "Microsoft Excel"),
            ("word-o365", "Microsoft Word"),
            ("powerpoint-o365", "Microsoft PowerPoint"),
        ];
        for (launcher, name) in names {
            std::fs::write(
                applications_dir.join(format!("{launcher}.desktop")),
                desktop_content(launcher, name),
            )
            .expect("desktop should be written");
        }
    }

    fn ready_for_final_verify_state() -> OfficeProvisioningState {
        let mut state = OfficeProvisioningState::new("office");
        for phase in [
            OfficePhase::RemoteappPrepare,
            OfficePhase::OfficeInstall,
            OfficePhase::WinappsConfig,
        ] {
            state.mark_phase_running(phase).expect("phase should run");
            let evidence = match phase {
                OfficePhase::OfficeInstall => Some(PhaseEvidence {
                    files: OFFICE_DESKTOP_APPS
                        .iter()
                        .map(|app| app.executable.to_string())
                        .collect(),
                    registry: Some(serde_json::json!({
                        "productReleaseIds": "O365ProPlusRetail",
                        "versionToReport": "16.0.12345.67890",
                        "platform": "x64",
                    })),
                    ..PhaseEvidence::default()
                }),
                OfficePhase::WinappsConfig => Some(PhaseEvidence {
                    launcher_ids: OFFICE_LAUNCHERS
                        .iter()
                        .map(|launcher| launcher.to_string())
                        .collect(),
                    ..PhaseEvidence::default()
                }),
                _ => None,
            };
            state
                .mark_phase_done(phase, evidence)
                .expect("phase should finish");
        }
        state
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be valid")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "winbox-winapps-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }
}

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    env_file, flatpak::FREERDP_FLATPAK_COMMAND, launch_error::OfficeError,
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

pub fn managed_source_dir() -> PathBuf {
    paths::data_dir().join("winapps")
}

pub fn winapps_conf_path() -> PathBuf {
    paths::home().join(WINAPPS_CONF_RELATIVE)
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
        _ => "Falha na configuração WinApps.",
    }
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
    use mock::MockWinAppsClient;
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

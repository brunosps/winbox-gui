use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    env_file,
    guest_executor::{self, GuestScript},
    launch_error::OfficeError,
    validation,
};

pub const OFFICE_SETUP_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkID=626065";
pub const OFFICE_SETUP_EXE: &str = "setup.exe";
pub const OFFICE_CONFIGURATION_XML: &str = "configuration.xml";
pub const OFFICE_INSTALL_SCRIPT: &str = "winbox-office-install.ps1";
pub const OFFICE_SETUP_VERIFY_SCRIPT: &str = "winbox-office-verify-setup.ps1";
pub const OFFICE_INSTALL_MARKER: &str = "office_install.json";
pub const OFFICE_STAGE_MARKER: &str = "office_stage_odt.json";
pub const OFFICE_ODT_MODE_CONFIGURE_CDN: &str = "configure_cdn";

pub const EXCLUDED_APPS: &[&str] = &[
    "Access",
    "Groove",
    "Lync",
    "OneDrive",
    "OneNote",
    "Outlook",
    "OutlookForWindows",
    "Publisher",
    "Teams",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeOdtConfig {
    pub product_id: String,
    pub language: String,
    pub channel: String,
}

impl OfficeOdtConfig {
    pub fn new(product_id: &str, language: &str, channel: &str) -> Result<Self> {
        validate_allowed(
            "OFFICE_PRODUCT_ID",
            product_id,
            validation::OFFICE_PRODUCT_IDS,
        )?;
        validate_allowed("OFFICE_LANGUAGE", language, validation::OFFICE_LANGUAGES)?;
        validate_allowed("OFFICE_CHANNEL", channel, validation::OFFICE_CHANNELS)?;
        Ok(Self {
            product_id: product_id.to_string(),
            language: language.to_string(),
            channel: channel.to_string(),
        })
    }

    pub fn from_env_map(map: &env_file::EnvMap) -> Result<Self> {
        let env = validation::office_env_config_from_map(map);
        validation::validate_office_config(&env)?;
        let product_id = default_if_empty(
            &env.office_product_id,
            validation::OFFICE_DEFAULT_PRODUCT_ID,
        );
        let language = default_if_empty(&env.office_language, validation::OFFICE_DEFAULT_LANGUAGE);
        let channel = default_if_empty(&env.office_channel, validation::OFFICE_DEFAULT_CHANNEL);
        Self::new(product_id, language, channel)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfficeOdtStageOptions {
    pub expected_setup_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeOdtStage {
    pub odt_dir: PathBuf,
    pub setup_exe: PathBuf,
    pub configuration_xml: PathBuf,
    pub install_script: PathBuf,
    pub verify_script: PathBuf,
    pub integrity: OfficeSetupIntegrity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficeSetupIntegrity {
    Sha256 { hash: String },
    AuthenticodeGuest { script: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeOdtStageFailed {
    pub detail: String,
}

impl OfficeOdtStageFailed {
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        OfficeError::OFFICE_ODT_STAGE_FAILED
    }
}

impl std::fmt::Display for OfficeOdtStageFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} Próxima ação: revalide o ODT fwlink/hash da release ou rode a verificação Authenticode no guest antes de instalar.",
            self.code(),
            self.detail
        )
    }
}

impl std::error::Error for OfficeOdtStageFailed {}

pub trait OdtHost {
    fn download_file(&self, url: &str, destination: &Path) -> Result<()>;
    fn sha256_file(&self, path: &Path) -> Result<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CliOdtHost;

impl OdtHost for CliOdtHost {
    fn download_file(&self, url: &str, destination: &Path) -> Result<()> {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        let output = Command::new("curl")
            .args(["--fail", "--location", "--show-error", "--silent"])
            .arg("--output")
            .arg(destination)
            .arg(url)
            .output()
            .context("executando curl para baixar ODT")?;
        if output.status.success() {
            return Ok(());
        }
        bail!(
            "curl falhou ao baixar ODT: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }

    fn sha256_file(&self, path: &Path) -> Result<String> {
        let output = Command::new("sha256sum")
            .arg(path)
            .output()
            .with_context(|| format!("executando sha256sum em {}", path.display()))?;
        if !output.status.success() {
            bail!(
                "sha256sum falhou: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .split_whitespace()
            .next()
            .map(|hash| hash.to_ascii_lowercase())
            .ok_or_else(|| anyhow!("sha256sum não retornou hash"))
    }
}

pub fn render_configuration_xml(config: &OfficeOdtConfig) -> String {
    let mut xml = String::new();
    xml.push_str("<Configuration>\n");
    xml.push_str(&format!(
        "  <Add OfficeClientEdition=\"64\" Channel=\"{}\">\n",
        xml_attr(&config.channel)
    ));
    xml.push_str(&format!(
        "    <Product ID=\"{}\">\n",
        xml_attr(&config.product_id)
    ));
    xml.push_str(&format!(
        "      <Language ID=\"{}\" />\n",
        xml_attr(&config.language)
    ));
    for app in EXCLUDED_APPS {
        xml.push_str(&format!("      <ExcludeApp ID=\"{}\" />\n", xml_attr(app)));
    }
    xml.push_str("    </Product>\n");
    xml.push_str("  </Add>\n");
    xml.push_str("  <Display Level=\"None\" AcceptEULA=\"TRUE\" />\n");
    xml.push_str("</Configuration>\n");
    xml
}

pub fn render_office_install_script() -> String {
    let lines = vec![
        "$ErrorActionPreference = 'Stop'".to_string(),
        "$shareRoot = '\\\\host.lan\\Data\\winbox-office'".to_string(),
        "$odtDir = Join-Path $shareRoot 'odt'".to_string(),
        "$markerDir = Join-Path $shareRoot 'markers'".to_string(),
        "$logDir = Join-Path $shareRoot 'logs'".to_string(),
        "New-Item -ItemType Directory -Force -Path $markerDir, $logDir | Out-Null".to_string(),
        "$setupPath = Join-Path $odtDir 'setup.exe'".to_string(),
        "$configPath = Join-Path $odtDir 'configuration.xml'".to_string(),
        "$markerPath = Join-Path $markerDir 'office_install.json'".to_string(),
        "$logPath = Join-Path $logDir 'office_install.log'".to_string(),
        "function Write-WinboxMarker($status, $exitCode, $errorCode, $message, $office) {"
            .to_string(),
        "  $payload = [ordered]@{ phase = 'office_install'; status = $status; exitCode = $exitCode; updatedAt = (Get-Date).ToUniversalTime().ToString('o') }".to_string(),
        "  if ($office) { $payload.office = $office }".to_string(),
        "  if ($errorCode) { $payload.error = [ordered]@{ code = $errorCode; message = $message; logPath = 'logs/office_install.log' } }".to_string(),
        "  $payload | ConvertTo-Json -Depth 6 -Compress | Set-Content -Path $markerPath -Encoding UTF8".to_string(),
        "}".to_string(),
        "function Resolve-WinboxErrorCode($exitCode, $message) {".to_string(),
        "  $text = \"$exitCode $message\"".to_string(),
        format!(
            "  if ($text -match '0x80070070|not enough space|not enough disk|disk full|insufficient disk|espaço insuficiente') {{ return '{}' }}",
            OfficeError::GUEST_DISK_FULL
        ),
        format!("  return '{}'", OfficeError::OFFICE_ODT_FAILED),
        "}".to_string(),
        "function Get-WinboxOfficeEvidence {".to_string(),
        "  $reg = Get-ItemProperty -Path 'HKLM:\\SOFTWARE\\Microsoft\\Office\\ClickToRun\\Configuration' -ErrorAction SilentlyContinue".to_string(),
        "  $officeDir = 'C:\\Program Files\\Microsoft Office\\root\\Office16'".to_string(),
        "  $exePaths = [ordered]@{".to_string(),
        "    excel = (Join-Path $officeDir 'EXCEL.EXE')".to_string(),
        "    winword = (Join-Path $officeDir 'WINWORD.EXE')".to_string(),
        "    powerpnt = (Join-Path $officeDir 'POWERPNT.EXE')".to_string(),
        "  }".to_string(),
        "  [ordered]@{".to_string(),
        "    productReleaseIds = [string]$reg.ProductReleaseIds".to_string(),
        "    versionToReport = [string]$reg.VersionToReport".to_string(),
        "    platform = [string]$reg.Platform".to_string(),
        "    exePaths = $exePaths".to_string(),
        "  }".to_string(),
        "}".to_string(),
        "function Assert-WinboxOfficeReady($office) {".to_string(),
        "  if ([string]::IsNullOrWhiteSpace($office.productReleaseIds)) { throw 'ClickToRun ProductReleaseIds ausente' }".to_string(),
        "  if ([string]::IsNullOrWhiteSpace($office.versionToReport)) { throw 'ClickToRun VersionToReport ausente' }".to_string(),
        "  if ($office.platform -ne 'x64') { throw \"ClickToRun Platform esperado x64, obtido $($office.platform)\" }".to_string(),
        "  foreach ($name in @('excel', 'winword', 'powerpnt')) {".to_string(),
        "    $path = $office.exePaths[$name]".to_string(),
        "    if (-not (Test-Path $path)) { throw \"Executável Office ausente: $path\" }".to_string(),
        "  }".to_string(),
        "}".to_string(),
        "try {".to_string(),
        "  Write-WinboxMarker 'running' $null $null $null $null".to_string(),
        "  Start-Transcript -Path $logPath -Append -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "  if (-not (Test-Path $setupPath)) { throw 'setup.exe ausente no share ODT' }".to_string(),
        "  if (-not (Test-Path $configPath)) { throw 'configuration.xml ausente no share ODT' }".to_string(),
        "  & $setupPath /configure $configPath".to_string(),
        "  $exitCode = $LASTEXITCODE".to_string(),
        "  if ($exitCode -ne 0) {".to_string(),
        "    $code = Resolve-WinboxErrorCode $exitCode \"ODT retornou $exitCode\"".to_string(),
        "    Write-WinboxMarker 'failed' $exitCode $code \"ODT retornou $exitCode\" $null".to_string(),
        "    Stop-Transcript -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "    exit $exitCode".to_string(),
        "  }".to_string(),
        "  $office = Get-WinboxOfficeEvidence".to_string(),
        "  try {".to_string(),
        "    Assert-WinboxOfficeReady $office".to_string(),
        "  } catch {".to_string(),
        format!(
            "    Write-WinboxMarker 'failed' 0 '{}' $_.Exception.Message $office",
            OfficeError::OFFICE_DETECTION_FAILED
        ),
        "    Stop-Transcript -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "    exit 1".to_string(),
        "  }".to_string(),
        "  Write-WinboxMarker 'done' 0 $null $null $office".to_string(),
        "  Stop-Transcript -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "  exit 0".to_string(),
        "} catch {".to_string(),
        "  $message = $_.Exception.Message".to_string(),
        "  $code = Resolve-WinboxErrorCode 1 $message".to_string(),
        "  Write-WinboxMarker 'failed' 1 $code $message $null".to_string(),
        "  Stop-Transcript -ErrorAction SilentlyContinue | Out-Null".to_string(),
        "  exit 1".to_string(),
        "}".to_string(),
    ];
    let refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
    crlf(&refs)
}

pub fn render_setup_authenticode_script() -> String {
    crlf(&[
        "$ErrorActionPreference = 'Stop'",
        "$shareRoot = '\\\\host.lan\\Data\\winbox-office'",
        "$setupPath = Join-Path (Join-Path $shareRoot 'odt') 'setup.exe'",
        "$markerDir = Join-Path $shareRoot 'markers'",
        "New-Item -ItemType Directory -Force -Path $markerDir | Out-Null",
        "$markerPath = Join-Path $markerDir 'office_stage_odt.json'",
        "try {",
        "  $signature = Get-AuthenticodeSignature -FilePath $setupPath",
        "  if ($signature.Status -ne 'Valid') { throw \"Authenticode inválido: $($signature.Status)\" }",
        "  [ordered]@{ phase = 'office_stage_odt'; status = 'done'; exitCode = 0; updatedAt = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content -Path $markerPath -Encoding UTF8",
        "  exit 0",
        "} catch {",
        "  [ordered]@{ phase = 'office_stage_odt'; status = 'failed'; exitCode = 1; error = [ordered]@{ code = 'office_odt_stage_failed'; message = $_.Exception.Message; logPath = 'logs/office_stage_odt.log' }; updatedAt = (Get-Date).ToUniversalTime().ToString('o') } | ConvertTo-Json -Compress | Set-Content -Path $markerPath -Encoding UTF8",
        "  exit 1",
        "}",
    ])
}

pub fn office_install_guest_script() -> GuestScript {
    GuestScript {
        phase: "office_install".to_string(),
        file_name: OFFICE_INSTALL_SCRIPT.to_string(),
        contents: render_office_install_script(),
    }
}

pub fn setup_authenticode_guest_script() -> GuestScript {
    GuestScript {
        phase: "office_stage_odt".to_string(),
        file_name: OFFICE_SETUP_VERIFY_SCRIPT.to_string(),
        contents: render_setup_authenticode_script(),
    }
}

pub fn setup_configure_command() -> Vec<&'static str> {
    vec![OFFICE_SETUP_EXE, "/configure", OFFICE_CONFIGURATION_XML]
}

pub fn stage_odt_assets(
    shared_dir: &Path,
    config: &OfficeOdtConfig,
    options: &OfficeOdtStageOptions,
    host: &dyn OdtHost,
) -> Result<OfficeOdtStage> {
    let odt_dir = odt_dir(shared_dir);
    let setup_exe = odt_dir.join(OFFICE_SETUP_EXE);
    let configuration_xml = odt_dir.join(OFFICE_CONFIGURATION_XML);
    let install_script = guest_executor::guest_scripts_dir(shared_dir).join(OFFICE_INSTALL_SCRIPT);
    let verify_script =
        guest_executor::guest_scripts_dir(shared_dir).join(OFFICE_SETUP_VERIFY_SCRIPT);

    create_share_layout(shared_dir).map_err(stage_failed)?;
    ensure_setup_exe(&setup_exe, host).map_err(stage_failed)?;
    let integrity = verify_or_plan_integrity(&setup_exe, &verify_script, options, host)
        .map_err(stage_failed)?;
    write_text(&configuration_xml, &render_configuration_xml(config)).map_err(stage_failed)?;
    write_crlf_file(&install_script, &render_office_install_script()).map_err(stage_failed)?;
    write_crlf_file(&verify_script, &render_setup_authenticode_script()).map_err(stage_failed)?;

    Ok(OfficeOdtStage {
        odt_dir,
        setup_exe,
        configuration_xml,
        install_script,
        verify_script,
        integrity,
    })
}

pub fn odt_dir(shared_dir: &Path) -> PathBuf {
    guest_executor::office_share_dir(shared_dir).join("odt")
}

fn create_share_layout(shared_dir: &Path) -> Result<()> {
    for dir in [
        odt_dir(shared_dir),
        guest_executor::guest_scripts_dir(shared_dir),
        guest_executor::guest_markers_dir(shared_dir),
        guest_executor::guest_logs_dir(shared_dir),
    ] {
        std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    Ok(())
}

fn ensure_setup_exe(setup_exe: &Path, host: &dyn OdtHost) -> Result<()> {
    if setup_exe.exists() {
        return Ok(());
    }
    let tmp = setup_exe.with_extension("exe.tmp");
    let _ = std::fs::remove_file(&tmp);
    host.download_file(OFFICE_SETUP_URL, &tmp)
        .with_context(|| format!("baixando Office Deployment Tool de {OFFICE_SETUP_URL}"))?;
    std::fs::rename(&tmp, setup_exe)
        .with_context(|| format!("movendo {} para {}", tmp.display(), setup_exe.display()))?;
    Ok(())
}

fn verify_or_plan_integrity(
    setup_exe: &Path,
    verify_script: &Path,
    options: &OfficeOdtStageOptions,
    host: &dyn OdtHost,
) -> Result<OfficeSetupIntegrity> {
    if let Some(expected) = options
        .expected_setup_sha256
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let actual = host.sha256_file(setup_exe)?;
        if !actual.eq_ignore_ascii_case(expected) {
            bail!(
                "SHA256 do OfficeSetup.exe divergente: esperado {}, obtido {}. A Microsoft pode ter rotacionado o ODT.",
                expected,
                actual
            );
        }
        return Ok(OfficeSetupIntegrity::Sha256 {
            hash: actual.to_ascii_lowercase(),
        });
    }

    Ok(OfficeSetupIntegrity::AuthenticodeGuest {
        script: verify_script.to_string_lossy().to_string(),
    })
}

fn stage_failed(err: anyhow::Error) -> anyhow::Error {
    anyhow!(OfficeOdtStageFailed::new(format!("{err:#}")))
}

fn validate_allowed(label: &str, value: &str, allowed: &[&str]) -> Result<()> {
    validation::validate_env_value(label, value)?;
    if allowed.contains(&value) {
        return Ok(());
    }
    bail!(
        "{label} inválido '{value}' — valores permitidos: {}.",
        allowed.join(", ")
    )
}

fn default_if_empty<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.trim().is_empty() {
        default
    } else {
        value.trim()
    }
}

fn write_text(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(path, contents).with_context(|| format!("escrevendo {}", path.display()))
}

fn write_crlf_file(path: &Path, contents: &str) -> Result<()> {
    write_text(path, &normalize_crlf(contents))
}

fn normalize_crlf(contents: &str) -> String {
    contents.replace("\r\n", "\n").replace('\n', "\r\n")
}

fn crlf(lines: &[&str]) -> String {
    let mut out = lines.join("\r\n");
    out.push_str("\r\n");
    out
}

fn xml_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
pub mod mock {
    use super::{OdtHost, OFFICE_SETUP_URL};
    use anyhow::{bail, Result};
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    #[derive(Default)]
    pub struct MockOdtHost {
        pub fail_download: RefCell<Option<String>>,
        pub sha256: RefCell<String>,
        pub downloads: RefCell<Vec<(String, PathBuf)>>,
    }

    impl MockOdtHost {
        pub fn new() -> Self {
            Self {
                sha256: RefCell::new("0".repeat(64)),
                ..Self::default()
            }
        }

        pub fn fail_download(&self, message: &str) {
            *self.fail_download.borrow_mut() = Some(message.to_string());
        }

        pub fn seed_sha256(&self, hash: &str) {
            *self.sha256.borrow_mut() = hash.to_string();
        }
    }

    impl OdtHost for MockOdtHost {
        fn download_file(&self, url: &str, destination: &Path) -> Result<()> {
            self.downloads
                .borrow_mut()
                .push((url.to_string(), destination.to_path_buf()));
            if let Some(message) = self.fail_download.borrow_mut().take() {
                bail!("{message}");
            }
            if url != OFFICE_SETUP_URL {
                bail!("URL ODT inesperada: {url}");
            }
            std::fs::write(destination, b"fake office setup")?;
            Ok(())
        }

        fn sha256_file(&self, _path: &Path) -> Result<String> {
            Ok(self.sha256.borrow().clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockOdtHost;
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn odt_configuration_excludes_only_supported_apps() {
        let config = OfficeOdtConfig::new("O365BusinessRetail", "pt-br", "Current")
            .expect("config should be valid");

        let xml = render_configuration_xml(&config);

        assert!(xml.contains("<Add OfficeClientEdition=\"64\" Channel=\"Current\">"));
        assert!(xml.contains("<Product ID=\"O365BusinessRetail\">"));
        assert!(xml.contains("<Language ID=\"pt-br\" />"));
        assert!(xml.contains("<Display Level=\"None\" AcceptEULA=\"TRUE\" />"));
        assert_eq!(exclude_apps_from_xml(&xml), EXCLUDED_APPS);
        assert!(!xml.contains("Excel"));
        assert!(!xml.contains("Word\""));
        assert!(!xml.contains("PowerPoint"));
    }

    #[test]
    fn odt_configuration_defaults_and_rejects_invalid_values() {
        let map = env_file::parse(
            "PROFILE_KIND=office\nOFFICE_PRODUCT_ID=\nOFFICE_LANGUAGE=\nOFFICE_CHANNEL=\n",
        );
        let config = OfficeOdtConfig::from_env_map(&map).expect("defaults should be accepted");
        assert_eq!(config.product_id, validation::OFFICE_DEFAULT_PRODUCT_ID);
        assert_eq!(config.language, validation::OFFICE_DEFAULT_LANGUAGE);
        assert_eq!(config.channel, validation::OFFICE_DEFAULT_CHANNEL);

        let err = OfficeOdtConfig::new("OfficeLTSCRetail", "pt-br", "Current")
            .expect_err("invalid product should be rejected");
        assert!(format!("{err:#}").contains("OFFICE_PRODUCT_ID inválido"));
    }

    #[test]
    fn odt_reapply_replaces_exclude_apps_instead_of_merging() {
        let root = temp_dir("reapply");
        let shared = root.join("shared");
        let setup = odt_dir(&shared).join(OFFICE_SETUP_EXE);
        std::fs::create_dir_all(setup.parent().unwrap()).expect("odt dir should exist");
        std::fs::write(&setup, b"existing setup").expect("setup should exist");
        let host = MockOdtHost::new();
        let config =
            OfficeOdtConfig::new("O365ProPlusRetail", "en-us", "Current").expect("valid config");

        stage_odt_assets(&shared, &config, &OfficeOdtStageOptions::default(), &host)
            .expect("initial stage should pass");
        let xml_path = odt_dir(&shared).join(OFFICE_CONFIGURATION_XML);
        std::fs::write(
            &xml_path,
            "<Configuration><ExcludeApp ID=\"Visio\" /></Configuration>",
        )
        .expect("test should simulate stale xml");

        stage_odt_assets(&shared, &config, &OfficeOdtStageOptions::default(), &host)
            .expect("restage should rewrite xml");
        let xml = std::fs::read_to_string(xml_path).expect("xml should exist");

        assert_eq!(exclude_apps_from_xml(&xml), EXCLUDED_APPS);
        assert!(!xml.contains("Visio"));
        assert!(host.downloads.borrow().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn odt_stage_writes_setup_xml_and_crlf_scripts() {
        let root = temp_dir("stage");
        let shared = root.join("shared");
        let host = MockOdtHost::new();
        let config =
            OfficeOdtConfig::new("O365ProPlusRetail", "pt-br", "Current").expect("valid config");

        let stage = stage_odt_assets(&shared, &config, &OfficeOdtStageOptions::default(), &host)
            .expect("stage should pass");

        assert_eq!(stage.setup_exe, odt_dir(&shared).join(OFFICE_SETUP_EXE));
        assert!(stage.setup_exe.is_file());
        assert!(stage.configuration_xml.is_file());
        assert!(stage.install_script.is_file());
        assert!(stage.verify_script.is_file());
        assert!(matches!(
            stage.integrity,
            OfficeSetupIntegrity::AuthenticodeGuest { .. }
        ));
        assert_no_lf_without_cr(&std::fs::read_to_string(stage.install_script).unwrap());
        assert_no_lf_without_cr(&std::fs::read_to_string(stage.verify_script).unwrap());
        assert_eq!(
            host.downloads.borrow()[0].0,
            "https://go.microsoft.com/fwlink/p/?LinkID=626065"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn odt_stage_validates_sha256_when_release_hash_is_pinned() {
        let root = temp_dir("sha");
        let shared = root.join("shared");
        let setup = odt_dir(&shared).join(OFFICE_SETUP_EXE);
        std::fs::create_dir_all(setup.parent().unwrap()).expect("odt dir should exist");
        std::fs::write(&setup, b"existing setup").expect("setup should exist");
        let host = MockOdtHost::new();
        let expected = "a".repeat(64);
        host.seed_sha256(&expected);
        let config =
            OfficeOdtConfig::new("O365ProPlusRetail", "pt-br", "Current").expect("valid config");

        let stage = stage_odt_assets(
            &shared,
            &config,
            &OfficeOdtStageOptions {
                expected_setup_sha256: Some(expected.clone()),
            },
            &host,
        )
        .expect("matching hash should pass");

        assert_eq!(
            stage.integrity,
            OfficeSetupIntegrity::Sha256 { hash: expected }
        );

        let bad = stage_odt_assets(
            &shared,
            &config,
            &OfficeOdtStageOptions {
                expected_setup_sha256: Some("b".repeat(64)),
            },
            &host,
        )
        .expect_err("mismatched hash should fail");
        assert!(format!("{bad:#}").contains(OfficeError::OFFICE_ODT_STAGE_FAILED));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_install_script_mounts_setup_configure_command() {
        assert_eq!(
            setup_configure_command(),
            vec![OFFICE_SETUP_EXE, "/configure", OFFICE_CONFIGURATION_XML]
        );
        let script = render_office_install_script();
        assert!(script.contains("& $setupPath /configure $configPath"));
        assert!(script.contains(OFFICE_INSTALL_MARKER));
        assert_no_lf_without_cr(&script);
    }

    fn exclude_apps_from_xml(xml: &str) -> Vec<&str> {
        xml.lines()
            .filter_map(|line| line.split_once("<ExcludeApp ID=\""))
            .filter_map(|(_, rest)| rest.split_once('"').map(|(id, _)| id))
            .collect()
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
            "winbox-office-odt-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }
}

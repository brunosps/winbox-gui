use serde::Serialize;
use std::process::Command;

use super::office_preflight::{PreflightCheck, PreflightStatus};

pub const FREERDP_FLATPAK_ID: &str = "com.freerdp.FreeRDP";
pub const FREERDP_FLATPAK_COMMAND: &str = "flatpak run --command=xfreerdp com.freerdp.FreeRDP";
pub const FREERDP_FLATPAK_INSTALL_COMMAND: &str = "flatpak install flathub com.freerdp.FreeRDP";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait FlatpakClient {
    fn xfreerdp_version(&self, binary: &str) -> Option<CommandOutput>;
    fn flatpak_info(&self, app_id: &str) -> bool;
    fn flatpak_list_apps(&self) -> Option<CommandOutput>;
    fn flatpak_permissions(&self, app_id: &str) -> Option<CommandOutput>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CliFlatpakClient;

impl FlatpakClient for CliFlatpakClient {
    fn xfreerdp_version(&self, binary: &str) -> Option<CommandOutput> {
        Command::new(binary)
            .arg("/version")
            .output()
            .ok()
            .map(command_output)
    }

    fn flatpak_info(&self, app_id: &str) -> bool {
        Command::new("flatpak")
            .args(["info", app_id])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn flatpak_list_apps(&self) -> Option<CommandOutput> {
        Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version"])
            .output()
            .ok()
            .map(command_output)
    }

    fn flatpak_permissions(&self, app_id: &str) -> Option<CommandOutput> {
        Command::new("flatpak")
            .args(["info", "--show-permissions", app_id])
            .output()
            .ok()
            .map(command_output)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreerdpPreflight {
    pub check: PreflightCheck,
    pub command: Option<String>,
    pub native_version: Option<String>,
    pub flatpak_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatpakInstallConsent {
    Unknown,
    Accepted,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreerdpVersion {
    pub major: u32,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeFreerdp {
    binary: String,
    version: FreerdpVersion,
}

pub fn detect_freerdp(
    client: &dyn FlatpakClient,
    install_consent: FlatpakInstallConsent,
) -> FreerdpPreflight {
    let native = detect_native_freerdp(client);
    if let Some(native) = native.as_ref().filter(|native| native.version.major >= 3) {
        return FreerdpPreflight {
            check: PreflightCheck::new(
                "preflight_freerdp",
                PreflightStatus::Ok,
                "FreeRDP nativo major >= 3 disponível.",
                "RemoteApp pode usar o xfreerdp do host.",
                None,
                Some(serde_json::json!({
                    "command": native.binary,
                    "version": native.version.raw,
                })),
            ),
            command: Some(native.binary.clone()),
            native_version: Some(native.version.raw.clone()),
            flatpak_version: None,
        };
    }

    let native_reason = native
        .as_ref()
        .map(|native| format!("FreeRDP nativo {} é major < 3.", native.version.raw))
        .unwrap_or_else(|| "FreeRDP nativo ausente ou não executou /version.".to_string());

    if !client.flatpak_info(FREERDP_FLATPAK_ID) {
        let status = if install_consent == FlatpakInstallConsent::Refused {
            PreflightStatus::Blocker
        } else {
            PreflightStatus::Warning
        };
        let action = if install_consent == FlatpakInstallConsent::Refused {
            "Instale o Flatpak FreeRDP ou permita a instalação guiada para continuar."
        } else {
            "Autorize a instalação guiada: flatpak install flathub com.freerdp.FreeRDP."
        };
        return FreerdpPreflight {
            check: PreflightCheck::new(
                "flatpak_freerdp_missing",
                status,
                "FreeRDP 3.x via Flatpak deve estar disponível quando o nativo é inadequado.",
                "Sem FreeRDP compatível, WinApps/RemoteApp não conseguem abrir Office.",
                Some(action.to_string()),
                Some(serde_json::json!({
                    "installCommand": FREERDP_FLATPAK_INSTALL_COMMAND,
                    "nativeReason": native_reason,
                    "installConsent": format!("{install_consent:?}"),
                })),
            ),
            command: None,
            native_version: native.map(|native| native.version.raw),
            flatpak_version: None,
        };
    }

    let flatpak_version = client
        .flatpak_list_apps()
        .filter(|output| output.success)
        .and_then(|output| parse_flatpak_freerdp_version(&output.stdout));
    let permissions = client
        .flatpak_permissions(FREERDP_FLATPAK_ID)
        .filter(|output| output.success)
        .map(|output| output.stdout)
        .unwrap_or_default();
    if !flatpak_has_home_override(&permissions) {
        return FreerdpPreflight {
            check: PreflightCheck::new(
                "flatpak_home_override_missing",
                PreflightStatus::Blocker,
                "Flatpak FreeRDP deve ter override --filesystem=home.",
                "+home-drive precisa expor o HOME como \\\\tsclient\\home para abrir arquivos Linux no Office.",
                Some(
                    "Execute: flatpak override --user --filesystem=home com.freerdp.FreeRDP."
                        .to_string(),
                ),
                Some(serde_json::json!({
                    "command": "flatpak override --user --filesystem=home com.freerdp.FreeRDP",
                    "nativeReason": native_reason,
                    "flatpakVersion": flatpak_version,
                })),
            ),
            command: Some(FREERDP_FLATPAK_COMMAND.to_string()),
            native_version: native.map(|native| native.version.raw),
            flatpak_version,
        };
    }

    FreerdpPreflight {
        check: PreflightCheck::new(
            "preflight_freerdp",
            PreflightStatus::Ok,
            "FreeRDP Flatpak compatível disponível.",
            "O perfil Office forçará FREERDP_COMMAND para o Flatpak compatível.",
            None,
            Some(serde_json::json!({
                "command": FREERDP_FLATPAK_COMMAND,
                "nativeReason": native_reason,
                "flatpakVersion": flatpak_version,
            })),
        ),
        command: Some(FREERDP_FLATPAK_COMMAND.to_string()),
        native_version: native.map(|native| native.version.raw),
        flatpak_version,
    }
}

pub fn parse_freerdp_version(output: &str) -> Option<FreerdpVersion> {
    for token in output.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let trimmed = token.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.');
        if let Some(version) = parse_version_token(trimmed) {
            return Some(version);
        }
    }
    None
}

pub fn parse_flatpak_freerdp_version(output: &str) -> Option<String> {
    for line in output.lines() {
        if !line.contains(FREERDP_FLATPAK_ID) {
            continue;
        }
        let rest = line.split_once(FREERDP_FLATPAK_ID)?.1.trim();
        let version = rest
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches('.');
        if !version.is_empty() {
            return Some(version.to_string());
        }
    }
    None
}

pub fn flatpak_has_home_override(output: &str) -> bool {
    output.lines().any(|line| {
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        if key.trim() != "filesystems" {
            return false;
        }
        value
            .split([';', ',', ' '])
            .map(str::trim)
            .any(|entry| entry == "home" || entry.starts_with("home:"))
    })
}

fn detect_native_freerdp(client: &dyn FlatpakClient) -> Option<NativeFreerdp> {
    let mut first_incompatible = None;
    for binary in ["xfreerdp", "xfreerdp3"] {
        let Some(output) = client.xfreerdp_version(binary) else {
            continue;
        };
        if !output.success {
            continue;
        }
        let combined = format!("{}\n{}", output.stdout, output.stderr);
        if let Some(version) = parse_freerdp_version(&combined) {
            let native = NativeFreerdp {
                binary: binary.to_string(),
                version,
            };
            if native.version.major >= 3 {
                return Some(native);
            }
            if first_incompatible.is_none() {
                first_incompatible = Some(native);
            }
        }
    }
    first_incompatible
}

fn parse_version_token(token: &str) -> Option<FreerdpVersion> {
    let first = token.split('.').next()?;
    if first.is_empty() || !first.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut parts = token.split('.');
    let major = parts.next()?.parse().ok()?;
    parts.next()?;
    Some(FreerdpVersion {
        major,
        raw: token.trim_end_matches('.').to_string(),
    })
}

fn command_output(output: std::process::Output) -> CommandOutput {
    CommandOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}

#[cfg(test)]
pub mod mock {
    use super::{CommandOutput, FlatpakClient, FREERDP_FLATPAK_ID};
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct MockFlatpakClient {
        xfreerdp_outputs: RefCell<HashMap<String, Option<CommandOutput>>>,
        flatpak_installed: RefCell<bool>,
        flatpak_list_output: RefCell<Option<CommandOutput>>,
        permissions_output: RefCell<Option<CommandOutput>>,
        calls: RefCell<Vec<String>>,
    }

    impl MockFlatpakClient {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn seed_xfreerdp_version(&self, binary: &str, output: Option<CommandOutput>) {
            self.xfreerdp_outputs
                .borrow_mut()
                .insert(binary.to_string(), output);
        }

        pub fn seed_flatpak_installed(&self, installed: bool) {
            *self.flatpak_installed.borrow_mut() = installed;
        }

        pub fn seed_flatpak_list(&self, output: CommandOutput) {
            *self.flatpak_list_output.borrow_mut() = Some(output);
        }

        pub fn seed_permissions(&self, output: CommandOutput) {
            *self.permissions_output.borrow_mut() = Some(output);
        }

        pub fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }

        fn record(&self, call: impl Into<String>) {
            self.calls.borrow_mut().push(call.into());
        }
    }

    impl FlatpakClient for MockFlatpakClient {
        fn xfreerdp_version(&self, binary: &str) -> Option<CommandOutput> {
            self.record(format!("xfreerdp_version:{binary}"));
            self.xfreerdp_outputs
                .borrow()
                .get(binary)
                .cloned()
                .unwrap_or(None)
        }

        fn flatpak_info(&self, app_id: &str) -> bool {
            self.record(format!("flatpak_info:{app_id}"));
            app_id == FREERDP_FLATPAK_ID && *self.flatpak_installed.borrow()
        }

        fn flatpak_list_apps(&self) -> Option<CommandOutput> {
            self.record("flatpak_list");
            self.flatpak_list_output.borrow().clone()
        }

        fn flatpak_permissions(&self, app_id: &str) -> Option<CommandOutput> {
            self.record(format!("flatpak_permissions:{app_id}"));
            self.permissions_output.borrow().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockFlatpakClient;
    use super::*;

    fn ok(stdout: &str) -> CommandOutput {
        CommandOutput {
            success: true,
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    #[test]
    fn parse_freerdp_version_reads_realistic_outputs() {
        assert_eq!(
            parse_freerdp_version("This is FreeRDP version 3.28.0 (n/a)").unwrap(),
            FreerdpVersion {
                major: 3,
                raw: "3.28.0".to_string(),
            }
        );
        assert_eq!(parse_freerdp_version("xfreerdp 2.11.7").unwrap().major, 2);
        assert!(parse_freerdp_version("FreeRDP version unknown").is_none());
    }

    #[test]
    fn native_freerdp_v2_forces_flatpak() {
        let client = MockFlatpakClient::new();
        client.seed_xfreerdp_version("xfreerdp", Some(ok("This is FreeRDP version 2.11.7")));
        client.seed_flatpak_installed(true);
        client.seed_flatpak_list(ok("com.freerdp.FreeRDP\t3.28.\n"));
        client.seed_permissions(ok("[Context]\nfilesystems=xdg-download;home;\n"));

        let detected = detect_freerdp(&client, FlatpakInstallConsent::Unknown);

        assert_eq!(detected.command.as_deref(), Some(FREERDP_FLATPAK_COMMAND));
        assert_eq!(detected.flatpak_version.as_deref(), Some("3.28"));
        assert_eq!(detected.check.status, PreflightStatus::Ok);
        assert!(detected.check.impact.contains("Flatpak"));
    }

    #[test]
    fn native_xfreerdp3_wins_when_xfreerdp_is_v2() {
        let client = MockFlatpakClient::new();
        client.seed_xfreerdp_version("xfreerdp", Some(ok("xfreerdp 2.11.7")));
        client.seed_xfreerdp_version("xfreerdp3", Some(ok("This is FreeRDP version 3.28.0")));

        let detected = detect_freerdp(&client, FlatpakInstallConsent::Unknown);

        assert_eq!(detected.command.as_deref(), Some("xfreerdp3"));
        assert_eq!(detected.native_version.as_deref(), Some("3.28.0"));
        assert_eq!(detected.check.status, PreflightStatus::Ok);
    }

    #[test]
    fn flatpak_missing_blocks_only_after_refusal() {
        let client = MockFlatpakClient::new();
        client.seed_xfreerdp_version("xfreerdp", Some(ok("xfreerdp 2.11.7")));
        client.seed_flatpak_installed(false);

        let offered = detect_freerdp(&client, FlatpakInstallConsent::Unknown);
        assert_eq!(offered.check.status, PreflightStatus::Warning);
        assert!(offered
            .check
            .details
            .as_ref()
            .unwrap()
            .to_string()
            .contains(FREERDP_FLATPAK_INSTALL_COMMAND));

        let refused = detect_freerdp(&client, FlatpakInstallConsent::Refused);
        assert_eq!(refused.check.status, PreflightStatus::Blocker);
        assert_eq!(refused.check.id, "flatpak_freerdp_missing");
    }

    #[test]
    fn flatpak_home_override_required_for_home_drive() {
        let client = MockFlatpakClient::new();
        client.seed_xfreerdp_version("xfreerdp", Some(ok("xfreerdp 2.11.7")));
        client.seed_flatpak_installed(true);
        client.seed_flatpak_list(ok("Application\tVersion\ncom.freerdp.FreeRDP\t3.28.0\n"));
        client.seed_permissions(ok("[Context]\nfilesystems=xdg-download;\n"));

        let detected = detect_freerdp(&client, FlatpakInstallConsent::Unknown);

        assert_eq!(detected.check.id, "flatpak_home_override_missing");
        assert_eq!(detected.check.status, PreflightStatus::Blocker);
        assert!(detected
            .check
            .action_hint
            .as_deref()
            .unwrap()
            .contains("flatpak override --user --filesystem=home"));
    }

    #[test]
    fn flatpak_permissions_parser_accepts_home_variants() {
        assert!(flatpak_has_home_override(
            "[Context]\nfilesystems=xdg-download;home;\n"
        ));
        assert!(flatpak_has_home_override(
            "[Context]\nfilesystems=home:rw;\n"
        ));
        assert!(!flatpak_has_home_override(
            "[Context]\nfilesystems=xdg-download;\n"
        ));
    }
}

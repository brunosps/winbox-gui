use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};
use std::io::{self, Write};

use crate::commands::office::{
    self, ManagedScope, OfficeAdoptProfileArgs, OfficeLaunchAppArgs, OfficeNameArgs,
    OfficePreflightArgs, OfficeRemoveProfileArgs, OfficeRetryPhaseArgs,
    OfficeStartProvisioningArgs,
};
use crate::core::launch_error::OfficeError;
use crate::core::office_preflight::Resources;
use crate::core::office_state::OfficePhase;

#[derive(Debug, Args)]
pub struct OfficeArgs {
    #[command(subcommand)]
    pub command: OfficeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum OfficeSubcommand {
    /// Estado persistido do provisionamento Office.
    Status { profile: String },
    /// Diagnóstico do host antes de criar/adotar um perfil Office.
    Preflight {
        profile: Option<String>,
        #[command(flatten)]
        resources: ResourceArgs,
    },
    /// Inicia o provisionamento contract-first do perfil Office.
    Provision {
        profile: String,
        #[arg(long = "product-id", default_value = "O365ProPlusRetail")]
        product_id: String,
        #[arg(long, default_value = "pt-br")]
        language: String,
        #[arg(long = "accept-byol", alias = "byol-accepted")]
        byol_accepted: bool,
        #[arg(long = "telemetry-opt-in")]
        telemetry_opt_in: Option<bool>,
        #[arg(long = "adoption-id")]
        adoption_id: Option<String>,
        #[arg(long, value_enum)]
        progress: Option<ProgressFormat>,
        #[command(flatten)]
        resources: ResourceArgs,
    },
    /// Rebaixa uma fase retry-safe e suas dependentes para pending.
    Retry {
        profile: String,
        #[arg(value_parser = parse_office_phase)]
        phase: OfficePhase,
        #[arg(long, value_enum)]
        progress: Option<ProgressFormat>,
    },
    /// Adota um setup Office existente após confirmação do usuário.
    Adopt {
        profile: String,
        #[arg(long = "adoption-id", default_value = "manual")]
        adoption_id: String,
        #[arg(long)]
        confirm: bool,
    },
    /// Abre Excel, Word ou PowerPoint via launcher WinApps.
    Launch {
        profile: String,
        app: String,
        #[arg(long = "gui-progress")]
        gui_progress: bool,
        #[arg(long, value_enum)]
        progress: Option<ProgressFormat>,
        #[arg(last = true)]
        files: Vec<String>,
    },
    /// Remove o perfil Office usando token de confirmação.
    Remove {
        profile: String,
        #[arg(long = "delete-disk")]
        delete_disk: bool,
        #[arg(long)]
        confirm: Option<String>,
    },
}

#[derive(Debug, Clone, Args)]
pub struct ResourceArgs {
    #[arg(long = "ram-gb", default_value_t = 8)]
    ram_gb: u32,
    #[arg(long = "cpu-cores", default_value_t = 4)]
    cpu_cores: u32,
    #[arg(long = "disk-gb", default_value_t = 128)]
    disk_gb: u32,
    #[arg(long = "storage-path")]
    storage_path: Option<String>,
    #[arg(long = "warning-override")]
    warning_override: bool,
}

impl ResourceArgs {
    fn into_resources(self) -> Resources {
        Resources {
            ram_gb: self.ram_gb,
            cpu_cores: self.cpu_cores,
            disk_gb: self.disk_gb,
            storage_path: self.storage_path,
            warning_override: self.warning_override,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProgressFormat {
    Jsonl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Human,
    Json,
}

impl OutputMode {
    pub fn from_json(enabled: bool) -> Self {
        if enabled {
            Self::Json
        } else {
            Self::Human
        }
    }
}

pub fn run_office_cli(args: OfficeArgs, mode: OutputMode) -> i32 {
    let result = run_office_command(args, mode);
    let stdout = io::stdout();
    let mut out = stdout.lock();
    print_office_result(&result, mode, &mut out)
}

fn print_office_result<W: Write>(
    result: &std::result::Result<Value, OfficeError>,
    mode: OutputMode,
    writer: &mut W,
) -> i32 {
    let printed = match (result, mode) {
        (Ok(value), OutputMode::Json) => print_office_json_success(writer, value),
        (Err(error), OutputMode::Json) => print_office_json_error(writer, error),
        (Err(error), OutputMode::Human)
            if error.code() == OfficeError::REMOVE_REQUIRES_CONFIRMATION =>
        {
            print_remove_confirmation(writer, error)
        }
        (Err(error), OutputMode::Human) => writeln!(io::stderr(), "✗ {error}"),
        (Ok(_), OutputMode::Human) => Ok(()),
    };
    if printed.is_err() || result.is_err() {
        1
    } else {
        0
    }
}

pub fn run_office_command(
    args: OfficeArgs,
    mode: OutputMode,
) -> std::result::Result<Value, OfficeError> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    run_office_command_with_writer(args, mode, &mut out)
}

pub fn run_office_command_with_writer<W: Write>(
    args: OfficeArgs,
    mode: OutputMode,
    writer: &mut W,
) -> std::result::Result<Value, OfficeError> {
    let value = match args.command {
        OfficeSubcommand::Status { profile } => {
            json_value(office::get_state_contract(OfficeNameArgs {
                name: profile,
            })?)?
        }
        OfficeSubcommand::Preflight { profile, resources } => {
            json_value(office::preflight_contract(OfficePreflightArgs {
                name: profile,
                resources: resources.into_resources(),
            })?)?
        }
        OfficeSubcommand::Provision {
            profile,
            product_id,
            language,
            byol_accepted,
            telemetry_opt_in,
            adoption_id,
            progress,
            resources,
        } => {
            let command_profile = profile.clone();
            run_with_progress(
                writer,
                progress,
                &profile,
                "office_provision",
                OfficePhase::ByolAcceptance,
                || {
                    json_value(office::start_provisioning_contract(
                        OfficeStartProvisioningArgs {
                            name: command_profile,
                            product_id,
                            language,
                            resources: resources.into_resources(),
                            byol_accepted,
                            telemetry_opt_in,
                            adoption_id,
                        },
                    )?)
                },
            )?
        }
        OfficeSubcommand::Retry {
            profile,
            phase,
            progress,
        } => {
            let command_profile = profile.clone();
            run_with_progress(writer, progress, &profile, "office_retry", phase, || {
                json_value(office::retry_phase_contract(OfficeRetryPhaseArgs {
                    name: command_profile,
                    phase,
                })?)
            })?
        }
        OfficeSubcommand::Adopt {
            profile,
            adoption_id,
            confirm,
        } => json_value(office::adopt_profile_contract(OfficeAdoptProfileArgs {
            name: profile,
            adoption_id,
            managed_scope: ManagedScope::default(),
            confirm,
        })?)?,
        OfficeSubcommand::Launch {
            profile,
            app,
            gui_progress,
            progress,
            files,
        } => {
            let command_profile = profile.clone();
            let mut value = run_launch_with_progress(writer, progress, &profile, || {
                json_value(office::launch_app_contract(OfficeLaunchAppArgs {
                    name: command_profile,
                    app_id: app,
                    files,
                    gui_progress,
                })?)
            })?;
            value["guiProgress"] = json!(gui_progress);
            value
        }
        OfficeSubcommand::Remove {
            profile,
            delete_disk,
            confirm,
        } => json_value(office::remove_profile_contract(OfficeRemoveProfileArgs {
            name: profile,
            delete_disk,
            confirm_token: confirm,
        })?)?,
    };

    if mode == OutputMode::Human {
        print_human_value(writer, &value)?;
    }
    Ok(value)
}

fn run_with_progress<W, F>(
    writer: &mut W,
    progress: Option<ProgressFormat>,
    profile: &str,
    operation: &str,
    phase: OfficePhase,
    run: F,
) -> std::result::Result<Value, OfficeError>
where
    W: Write,
    F: FnOnce() -> std::result::Result<Value, OfficeError>,
{
    emit_progress_if_requested(
        writer,
        progress,
        profile,
        operation,
        phase,
        "running",
        "Iniciando fase Office.",
    )?;
    match run() {
        Ok(value) => {
            emit_progress_if_requested(
                writer,
                progress,
                profile,
                operation,
                phase,
                "success",
                "Fase Office concluída.",
            )?;
            Ok(value)
        }
        Err(error) => {
            emit_progress_if_requested(
                writer,
                progress,
                profile,
                operation,
                error.fields().phase,
                "error",
                &error.to_string(),
            )?;
            Err(error)
        }
    }
}

fn run_launch_with_progress<W, F>(
    writer: &mut W,
    progress: Option<ProgressFormat>,
    profile: &str,
    run: F,
) -> std::result::Result<Value, OfficeError>
where
    W: Write,
    F: FnOnce() -> std::result::Result<Value, OfficeError>,
{
    emit_progress_if_requested(
        writer,
        progress,
        profile,
        "office_launch",
        OfficePhase::FirstLaunch,
        "running",
        "office_cold_start",
    )?;
    match run() {
        Ok(value) => {
            emit_progress_if_requested(
                writer,
                progress,
                profile,
                "office_launch",
                OfficePhase::FirstLaunch,
                "success",
                "office_launch_remoteapp",
            )?;
            Ok(value)
        }
        Err(error) => {
            emit_progress_if_requested(
                writer,
                progress,
                profile,
                "office_launch",
                error.fields().phase,
                "error",
                &error.to_string(),
            )?;
            Err(error)
        }
    }
}

fn emit_progress_if_requested<W: Write>(
    writer: &mut W,
    progress: Option<ProgressFormat>,
    profile: &str,
    operation: &str,
    phase: OfficePhase,
    status: &str,
    message: &str,
) -> std::result::Result<(), OfficeError> {
    if progress != Some(ProgressFormat::Jsonl) {
        return Ok(());
    }
    writeln!(
        writer,
        "{}",
        json!({
            "type": "progress",
            "profile": profile,
            "operation": operation,
            "phase": phase.as_str(),
            "status": status,
            "message": message,
            "timestamp": chrono::Local::now().to_rfc3339(),
        })
    )
    .map_err(write_error)
}

fn print_office_json_success<W: Write>(writer: &mut W, value: &Value) -> io::Result<()> {
    writeln!(writer, "{}", json!({ "ok": true, "value": value }))
}

fn print_office_json_error<W: Write>(writer: &mut W, error: &OfficeError) -> io::Result<()> {
    let mut envelope = json!({
        "ok": false,
        "error": {
            "code": error.code(),
            "message": error.to_string(),
            "hint": office_error_hint(error),
            "phase": error.fields().phase,
            "retryable": error.fields().retryable,
        }
    });
    if let Some(details) = &error.fields().details {
        envelope["error"]["details"] = details.clone();
    }
    writeln!(writer, "{envelope}")
}

fn print_human_value<W: Write>(
    writer: &mut W,
    value: &Value,
) -> std::result::Result<(), OfficeError> {
    writeln!(writer, "{value}").map_err(write_error)
}

fn print_remove_confirmation<W: Write>(writer: &mut W, error: &OfficeError) -> io::Result<()> {
    let details = error.fields().details.as_ref();
    let token = details
        .and_then(|details| details.get("confirmToken"))
        .and_then(Value::as_str)
        .unwrap_or("<token ausente>");
    let delete_disk = details
        .and_then(|details| details.get("deleteDisk"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    writeln!(
        writer,
        "Remoção do perfil Office exige confirmação explícita."
    )?;
    writeln!(writer, "Token: {token}")?;
    if delete_disk {
        writeln!(
            writer,
            "Esta confirmação também autoriza apagar o disco do perfil."
        )?;
    } else {
        writeln!(writer, "O disco do perfil será preservado.")?;
    }
    writeln!(writer, "Execute novamente com --confirm {token}.")
}

fn office_error_hint(error: &OfficeError) -> String {
    error
        .fields()
        .details
        .as_ref()
        .and_then(|details| details.get("actionHint"))
        .and_then(Value::as_str)
        .unwrap_or("Verifique o estado do perfil Office e execute novamente após corrigir a causa.")
        .to_string()
}

fn json_value<T: serde::Serialize>(value: T) -> std::result::Result<Value, OfficeError> {
    serde_json::to_value(value).map_err(|error| {
        OfficeError::new(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            true,
            Some(json!({ "detail": error.to_string() })),
        )
    })
}

fn write_error(error: io::Error) -> OfficeError {
    OfficeError::new(
        OfficeError::PROFILE_STATE_CONFLICT,
        OfficePhase::ProfileConfig,
        true,
        Some(json!({ "detail": error.to_string() })),
    )
}

fn parse_office_phase(value: &str) -> std::result::Result<OfficePhase, String> {
    OfficePhase::ALL
        .into_iter()
        .find(|phase| phase.as_str() == value)
        .ok_or_else(|| format!("fase Office inválida: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_contract_suite_preserves_public_shapes() {
        const EXPECTED_OFFICE_CODES: &[&str] = &[
            "byol_not_accepted",
            "preflight_kvm_missing",
            "preflight_docker_missing",
            "preflight_subnet_conflict",
            "flatpak_freerdp_missing",
            "flatpak_home_override_missing",
            "native_freerdp_too_old",
            "office_product_invalid",
            "profile_not_office",
            "profile_state_conflict",
            "guest_remoteapp_not_prepared",
            "guest_rdp_unreachable",
            "guest_executor_failed",
            "guest_phase_timeout",
            "guest_disk_full",
            "office_windows_failed",
            "office_odt_stage_failed",
            "office_odt_failed",
            "office_detection_failed",
            "winapps_clone_failed",
            "winapps_no_config",
            "winapps_missing_deps",
            "winapps_bad_port",
            "winapps_rdp_failed",
            "winapps_app_scan_failed",
            "winapps_pin_mismatch",
            "desktop_registration_failed",
            "file_association_failed",
            "app_not_registered",
            "app_launch_failed",
            "file_outside_home",
            "office_apps_maybe_open",
            "remove_requires_confirmation",
        ];
        assert_eq!(OfficeError::all_codes(), EXPECTED_OFFICE_CODES);

        for code in EXPECTED_OFFICE_CODES {
            let error = OfficeError::new(
                code,
                OfficePhase::OfficeInstall,
                true,
                Some(json!({ "contract": "office_contract_suite_preserves_public_shapes" })),
            );
            let serialized = serde_json::to_value(&error).expect("OfficeError should serialize");
            assert_eq!(serialized["code"], *code);
            assert_eq!(serialized["phase"], "office_install");
            assert_eq!(serialized["retryable"], true);
        }

        let (exit, out) = run_cli(
            OfficeArgs {
                command: OfficeSubcommand::Remove {
                    profile: "office-contract".to_string(),
                    delete_disk: false,
                    confirm: None,
                },
            },
            OutputMode::Json,
        );
        let envelope = last_json_line(&out);
        assert_eq!(exit, 1);
        assert_eq!(envelope["ok"], false);
        assert_eq!(
            envelope["error"]["code"],
            OfficeError::REMOVE_REQUIRES_CONFIRMATION
        );
        assert_eq!(envelope["error"]["phase"], "first_launch");
        assert_eq!(envelope["error"]["retryable"], false);
        assert!(envelope.get("value").is_none());
        assert_ne!(envelope["error"]["code"], "operation_failed");

        let (exit, progress_out) = run_cli(
            OfficeArgs {
                command: OfficeSubcommand::Provision {
                    profile: "office-contract".to_string(),
                    product_id: "O365ProPlusRetail".to_string(),
                    language: "pt-br".to_string(),
                    byol_accepted: false,
                    telemetry_opt_in: None,
                    adoption_id: None,
                    progress: Some(ProgressFormat::Jsonl),
                    resources: ResourceArgs {
                        ram_gb: 8,
                        cpu_cores: 4,
                        disk_gb: 128,
                        storage_path: None,
                        warning_override: false,
                    },
                },
            },
            OutputMode::Json,
        );
        let lines = json_lines(&progress_out);
        let progress = lines
            .iter()
            .find(|line| line["type"] == "progress" && line["status"] == "error")
            .expect("progress jsonl should include an error progress record");
        let mut progress_keys = progress
            .as_object()
            .expect("progress record should be an object")
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        progress_keys.sort_unstable();

        assert_eq!(exit, 1);
        assert_eq!(
            progress_keys,
            [
                "message",
                "operation",
                "phase",
                "profile",
                "status",
                "timestamp",
                "type"
            ]
        );
        assert_eq!(progress["profile"], "office-contract");
        assert_eq!(progress["operation"], "office_provision");
        assert_eq!(progress["phase"], "byol_acceptance");
        assert_eq!(progress["status"], "error");
        assert!(progress["timestamp"].as_str().is_some());
        assert_eq!(
            lines.last().expect("json envelope should be last")["error"]["code"],
            OfficeError::BYOL_NOT_ACCEPTED
        );
    }

    #[test]
    fn office_json_error_envelope_preserves_office_error_code() {
        let (exit, out) = run_cli(
            OfficeArgs {
                command: OfficeSubcommand::Remove {
                    profile: "office".to_string(),
                    delete_disk: false,
                    confirm: None,
                },
            },
            OutputMode::Json,
        );

        let envelope = last_json_line(&out);
        assert_eq!(exit, 1);
        assert_eq!(envelope["ok"], false);
        assert_eq!(
            envelope["error"]["code"],
            OfficeError::REMOVE_REQUIRES_CONFIRMATION
        );
        assert_ne!(envelope["error"]["code"], "operation_failed");
    }

    #[test]
    fn office_provision_progress_jsonl_emits_error_status() {
        let (exit, out) = run_cli(
            OfficeArgs {
                command: OfficeSubcommand::Provision {
                    profile: "office".to_string(),
                    product_id: "O365ProPlusRetail".to_string(),
                    language: "pt-br".to_string(),
                    byol_accepted: false,
                    telemetry_opt_in: None,
                    adoption_id: None,
                    progress: Some(ProgressFormat::Jsonl),
                    resources: ResourceArgs {
                        ram_gb: 8,
                        cpu_cores: 4,
                        disk_gb: 128,
                        storage_path: None,
                        warning_override: false,
                    },
                },
            },
            OutputMode::Json,
        );
        let lines = json_lines(&out);

        assert_eq!(exit, 1);
        assert!(lines.iter().any(|line| {
            line["type"] == "progress"
                && line["profile"] == "office"
                && line["operation"] == "office_provision"
                && line["phase"] == "byol_acceptance"
                && line["status"] == "error"
                && line.get("timestamp").and_then(Value::as_str).is_some()
        }));
        assert_eq!(
            lines.last().expect("envelope should be last")["error"]["code"],
            OfficeError::BYOL_NOT_ACCEPTED
        );
    }

    #[test]
    fn office_launch_progress_jsonl_emits_cold_start_and_remoteapp_steps() {
        let mut out = Vec::new();
        let value =
            run_launch_with_progress(&mut out, Some(ProgressFormat::Jsonl), "office", || {
                Ok(json!({ "appId": "excel", "delegatedTo": "excel-o365" }))
            })
            .expect("launch progress wrapper should return value");
        let lines = json_lines(&out);

        assert_eq!(value["appId"], "excel");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["operation"], "office_launch");
        assert_eq!(lines[0]["phase"], "first_launch");
        assert_eq!(lines[0]["status"], "running");
        assert_eq!(lines[0]["message"], "office_cold_start");
        assert_eq!(lines[1]["status"], "success");
        assert_eq!(lines[1]["message"], "office_launch_remoteapp");
    }

    #[test]
    fn office_adopt_uses_same_error_envelope() {
        let (exit, out) = run_cli(
            OfficeArgs {
                command: OfficeSubcommand::Adopt {
                    profile: "office".to_string(),
                    adoption_id: "manual".to_string(),
                    confirm: false,
                },
            },
            OutputMode::Json,
        );

        let envelope = last_json_line(&out);
        assert_eq!(exit, 1);
        assert_eq!(
            envelope["error"]["code"],
            OfficeError::PROFILE_STATE_CONFLICT
        );
        assert_ne!(envelope["error"]["code"], "operation_failed");
    }

    #[test]
    fn office_cli_remove_uses_same_confirmation_flow() {
        let (exit, out) = run_cli(
            OfficeArgs {
                command: OfficeSubcommand::Remove {
                    profile: "office-cli-token".to_string(),
                    delete_disk: true,
                    confirm: None,
                },
            },
            OutputMode::Human,
        );
        let text = String::from_utf8(out).expect("human output should be utf8");

        assert_eq!(exit, 1);
        assert!(text.contains("Remoção do perfil Office exige confirmação explícita."));
        assert!(text.contains("Token: office-remove:"));
        assert!(text.contains("Execute novamente com --confirm office-remove:"));
        assert!(text.contains("apagar o disco do perfil"));
    }

    fn run_cli(args: OfficeArgs, mode: OutputMode) -> (i32, Vec<u8>) {
        let mut out = Vec::new();
        let result = run_office_command_with_writer(args, mode, &mut out);
        let exit = print_office_result(&result, mode, &mut out);
        (exit, out)
    }

    fn json_lines(out: &[u8]) -> Vec<Value> {
        std::str::from_utf8(out)
            .expect("output should be utf8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("line should be json"))
            .collect()
    }

    fn last_json_line(out: &[u8]) -> Value {
        json_lines(out)
            .pop()
            .expect("json output should not be empty")
    }
}

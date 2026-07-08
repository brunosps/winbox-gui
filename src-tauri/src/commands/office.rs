use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::core::{
    docker::CliDocker,
    env_file,
    flatpak::CliFlatpakClient,
    guest_executor::{
        self, CliGuestExecutor, GuestExecutor, GuestRemoteappNotPrepared,
        GUEST_REMOTEAPP_NOT_PREPARED_CODE, REMOTEAPP_ACTION_HINT, REMOTEAPP_PREPARE_MARKER,
    },
    office_preflight::{self, CliHostPreflight, OfficePreflightResult, Resources},
    office_state::{OfficeLastError, OfficePhase, OfficeProvisioningState, PhaseEvidence},
    paths,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficePreflightArgs {
    #[serde(default)]
    pub name: Option<String>,
    pub resources: Resources,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeStartProvisioningArgs {
    pub name: String,
    #[serde(rename = "productId", alias = "product_id")]
    pub product_id: String,
    pub language: String,
    pub resources: Resources,
    #[serde(rename = "byolAccepted", alias = "byol_accepted")]
    pub byol_accepted: bool,
    #[serde(default, rename = "telemetryOptIn", alias = "telemetry_opt_in")]
    pub telemetry_opt_in: Option<bool>,
    #[serde(default, rename = "adoptionId", alias = "adoption_id")]
    pub adoption_id: Option<String>,
}

pub fn preflight(args: OfficePreflightArgs) -> Result<OfficePreflightResult> {
    office_preflight::run_preflight(
        args.name.as_deref(),
        &args.resources,
        &CliDocker,
        &CliFlatpakClient,
        &CliHostPreflight,
    )
}

pub fn prepare_remoteapp(profile: &str) -> Result<OfficeProvisioningState> {
    prepare_remoteapp_with_executor(profile, &CliGuestExecutor)
}

pub fn prepare_remoteapp_with_executor(
    profile: &str,
    executor: &dyn GuestExecutor,
) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    let env_path = paths::profile_env_file(profile);
    let map = env_file::read(&env_path)?;
    let oem_dir = env_path_or_default(
        env_file::get(&map, "OEM_DIR"),
        paths::profile_oem_dir(profile),
    );
    let shared_dir = env_path_or_default(
        env_file::get(&map, "SHARED_DIR"),
        paths::profile_shared_dir(profile),
    );
    prepare_remoteapp_at(profile, &profile_dir, &oem_dir, &shared_dir, executor)
}

fn prepare_remoteapp_at(
    profile: &str,
    profile_dir: &Path,
    oem_dir: &Path,
    shared_dir: &Path,
    executor: &dyn GuestExecutor,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    guest_executor::stage_remoteapp_bootstrap(oem_dir, shared_dir)?;
    state.mark_phase_running(OfficePhase::RemoteappPrepare)?;

    if let Some(marker) = executor.read_marker(profile, REMOTEAPP_PREPARE_MARKER)? {
        match guest_executor::verify_remoteapp_marker(Some(&marker)) {
            Ok(_) => {
                mark_remoteapp_done(&mut state, "marker")?;
                state.save_to_dir(profile_dir)?;
                return Ok(state);
            }
            Err(err) => {
                return fail_remoteapp_prepare(
                    state,
                    profile_dir,
                    format!("marker inválido: {err:#}"),
                );
            }
        }
    }

    match guest_executor::probe_remoteapp_channel(profile, executor) {
        Ok(run) => {
            mark_remoteapp_done(&mut state, "noop")?;
            if run.exit_code == 0 {
                state.ensure_remoteapp_ready_for_guest_phase(OfficePhase::OfficeStageOdt)?;
            }
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_remoteapp_prepare(state, profile_dir, format!("{err:#}")),
    }
}

fn mark_remoteapp_done(state: &mut OfficeProvisioningState, source: &str) -> Result<()> {
    state.mark_phase_done(
        OfficePhase::RemoteappPrepare,
        Some(PhaseEvidence {
            marker: Some(REMOTEAPP_PREPARE_MARKER.to_string()),
            registry: Some(serde_json::json!({
                "source": source,
                "keys": [
                    "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Terminal Server\\fDenyTSConnections",
                    "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Terminal Server\\WinStations\\RDP-Tcp\\UserAuthentication",
                    "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Terminal Server\\TSAppAllowList\\fDisabledAllowList",
                    "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Terminal Server\\TSAppAllowList\\fAllowUnlistedRemotePrograms",
                    "HKLM\\SOFTWARE\\Microsoft\\Terminal Server Client\\Default\\AddIns\\RDPDR\\IgnoreRemoteKeyboardLayout"
                ]
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn fail_remoteapp_prepare(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    detail: String,
) -> Result<OfficeProvisioningState> {
    state.mark_phase_failed(OfficeLastError {
        code: GUEST_REMOTEAPP_NOT_PREPARED_CODE.to_string(),
        message: "RemoteApp não está preparado para executar scripts no guest.".to_string(),
        phase: OfficePhase::RemoteappPrepare,
        retryable: true,
        details: Some(serde_json::json!({
            "detail": detail,
            "actionHint": REMOTEAPP_ACTION_HINT,
        })),
    });
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(GuestRemoteappNotPrepared::new(
        "canal RemoteApp /app: não executou o script no-op"
    )))
}

fn env_path_or_default(value: &str, default: PathBuf) -> PathBuf {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default
    } else {
        PathBuf::from(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::guest_executor::mock::MockGuestExecutor;
    use crate::core::guest_executor::{GuestMarker, GuestMarkerStatus};
    use crate::core::office_state::PhaseStatus;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn office_preflight_args_match_start_provisioning() {
        let preflight: OfficePreflightArgs = serde_json::from_value(serde_json::json!({
            "name": "office",
            "resources": {
                "ramGb": 8,
                "cpuCores": 4,
                "diskGb": 128,
                "storagePath": "/tmp/winbox-office",
                "warningOverride": true
            }
        }))
        .expect("office_preflight args should deserialize");
        let start: OfficeStartProvisioningArgs = serde_json::from_value(serde_json::json!({
            "name": "office",
            "productId": "O365ProPlusRetail",
            "language": "pt-br",
            "resources": {
                "ramGb": 8,
                "cpuCores": 4,
                "diskGb": 128,
                "storagePath": "/tmp/winbox-office",
                "warningOverride": true
            },
            "byolAccepted": true,
            "telemetryOptIn": false,
            "adoptionId": null
        }))
        .expect("office_start_provisioning args should deserialize");

        assert_eq!(preflight.resources, start.resources);

        let snake_case_start: OfficeStartProvisioningArgs =
            serde_json::from_value(serde_json::json!({
                "name": "office",
                "product_id": "O365BusinessRetail",
                "language": "en-us",
                "resources": {
                    "ramGb": 8,
                    "cpuCores": 4,
                    "diskGb": 128
                },
                "byol_accepted": true,
                "telemetry_opt_in": true,
                "adoption_id": "existing"
            }))
            .expect("aliases should remain compatible with command contract");
        assert_eq!(
            snake_case_start.resources.ram_gb,
            preflight.resources.ram_gb
        );
    }

    #[test]
    fn adoption_noop_failure_returns_guest_remoteapp_not_prepared() {
        let root = temp_dir("noop-failure");
        let profile_dir = root.join("profile");
        let oem_dir = root.join("oem");
        let shared_dir = root.join("shared");
        let executor = MockGuestExecutor::new();
        executor.fail_next_run("RemoteApp /app não preparado");

        let err = prepare_remoteapp_at("office", &profile_dir, &oem_dir, &shared_dir, &executor)
            .expect_err("falha do no-op deve retornar erro acionável");
        let message = format!("{err:#}");
        let state = OfficeProvisioningState::load_or_default(&profile_dir, "office")
            .expect("estado de falha deve ser persistido");

        assert!(message.contains(GUEST_REMOTEAPP_NOT_PREPARED_CODE));
        assert!(message.contains("C:\\OEM\\install.bat"));
        assert_eq!(
            state.phase_status(OfficePhase::RemoteappPrepare),
            Some(PhaseStatus::Failed)
        );
        let last_error = state.last_error.expect("last_error should be stored");
        assert_eq!(last_error.code, GUEST_REMOTEAPP_NOT_PREPARED_CODE);
        assert_eq!(last_error.phase, OfficePhase::RemoteappPrepare);
        assert!(last_error.retryable);
        assert!(oem_dir.join("install.bat").is_file());
        assert!(oem_dir
            .join(guest_executor::REMOTEAPP_PREPARE_SCRIPT)
            .is_file());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn prepare_remoteapp_accepts_completed_marker_without_noop() {
        let root = temp_dir("marker-ok");
        let profile_dir = root.join("profile");
        let oem_dir = root.join("oem");
        let shared_dir = root.join("shared");
        let executor = MockGuestExecutor::new();
        executor.seed_marker(
            REMOTEAPP_PREPARE_MARKER,
            Some(GuestMarker {
                phase: "remoteapp_prepare".to_string(),
                status: GuestMarkerStatus::Done,
                exit_code: Some(0),
                error: None,
                updated_at: Some("2026-07-08T00:00:00Z".to_string()),
            }),
        );

        let state = prepare_remoteapp_at("office", &profile_dir, &oem_dir, &shared_dir, &executor)
            .expect("marker done should complete phase");

        assert_eq!(
            state.phase_status(OfficePhase::RemoteappPrepare),
            Some(PhaseStatus::Done)
        );
        assert!(executor.runs().is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "winbox-office-command-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }
}

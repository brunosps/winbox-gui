use anyhow::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::core::{
    docker::CliDocker,
    env_file,
    flatpak::CliFlatpakClient,
    guest_executor::{
        self, CliGuestExecutor, GuestExecutor, GuestPhaseError, GuestPhaseTimeout,
        GuestRemoteappNotPrepared, GUEST_REMOTEAPP_NOT_PREPARED_CODE, REMOTEAPP_ACTION_HINT,
        REMOTEAPP_PREPARE_MARKER,
    },
    office_odt::{
        self, CliOdtHost, OdtHost, OfficeOdtStage, OfficeOdtStageFailed, OfficeOdtStageOptions,
        OFFICE_INSTALL_MARKER, OFFICE_ODT_STAGE_FAILED_CODE,
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

pub fn stage_odt(profile: &str) -> Result<OfficeProvisioningState> {
    stage_odt_with_host(profile, &CliOdtHost)
}

pub fn install_office(profile: &str) -> Result<OfficeProvisioningState> {
    install_office_with_executor(profile, &CliGuestExecutor)
}

pub fn stage_odt_with_host(profile: &str, host: &dyn OdtHost) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    let env_path = paths::profile_env_file(profile);
    let map = env_file::read(&env_path)?;
    let shared_dir = env_path_or_default(
        env_file::get(&map, "SHARED_DIR"),
        paths::profile_shared_dir(profile),
    );
    let config = office_odt::OfficeOdtConfig::from_env_map(&map)?;
    stage_odt_at(
        profile,
        &profile_dir,
        &shared_dir,
        &config,
        &OfficeOdtStageOptions::default(),
        host,
    )
}

pub fn install_office_with_executor(
    profile: &str,
    executor: &dyn GuestExecutor,
) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    install_office_at(
        profile,
        &profile_dir,
        executor,
        GuestPhaseTimeout::office_install(),
    )
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

fn stage_odt_at(
    profile: &str,
    profile_dir: &Path,
    shared_dir: &Path,
    config: &office_odt::OfficeOdtConfig,
    options: &OfficeOdtStageOptions,
    host: &dyn OdtHost,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    state.ensure_remoteapp_ready_for_guest_phase(OfficePhase::OfficeStageOdt)?;
    state.mark_phase_running(OfficePhase::OfficeStageOdt)?;

    match office_odt::stage_odt_assets(shared_dir, config, options, host) {
        Ok(stage) => {
            mark_odt_stage_done(&mut state, &stage)?;
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_odt_stage(state, profile_dir, format!("{err:#}")),
    }
}

fn mark_odt_stage_done(state: &mut OfficeProvisioningState, stage: &OfficeOdtStage) -> Result<()> {
    state.mark_phase_done(
        OfficePhase::OfficeStageOdt,
        Some(PhaseEvidence {
            files: vec![
                stage.setup_exe.display().to_string(),
                stage.configuration_xml.display().to_string(),
                stage.install_script.display().to_string(),
                stage.verify_script.display().to_string(),
            ],
            registry: Some(serde_json::json!({
                "odtMode": office_odt::OFFICE_ODT_MODE_CONFIGURE_CDN,
                "setupUrl": office_odt::OFFICE_SETUP_URL,
                "integrity": match &stage.integrity {
                    office_odt::OfficeSetupIntegrity::Sha256 { hash } => serde_json::json!({
                        "method": "sha256",
                        "hash": hash,
                    }),
                    office_odt::OfficeSetupIntegrity::AuthenticodeGuest { script } => serde_json::json!({
                        "method": "authenticode_guest",
                        "script": script,
                    }),
                }
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn fail_odt_stage(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    detail: String,
) -> Result<OfficeProvisioningState> {
    state.mark_phase_failed(OfficeLastError {
        code: OFFICE_ODT_STAGE_FAILED_CODE.to_string(),
        message: "Falha ao preparar Office Deployment Tool no share do perfil.".to_string(),
        phase: OfficePhase::OfficeStageOdt,
        retryable: true,
        details: Some(serde_json::json!({
            "detail": detail,
            "actionHint": "Verifique rede/curl, permissão de escrita no SHARED_DIR e, se o hash do ODT mudou, atualize o hash pinado da release após validação.",
        })),
    });
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(OfficeOdtStageFailed::new(
        "não foi possível preparar setup.exe/configuration.xml no share"
    )))
}

fn install_office_at(
    profile: &str,
    profile_dir: &Path,
    executor: &dyn GuestExecutor,
    timeout: GuestPhaseTimeout,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    state.ensure_remoteapp_ready_for_guest_phase(OfficePhase::OfficeInstall)?;
    if state.phase_status(OfficePhase::OfficeStageOdt)
        != Some(crate::core::office_state::PhaseStatus::Done)
    {
        return fail_office_install(
            state,
            profile_dir,
            GuestPhaseError::ExecutorFailed {
                phase: OfficePhase::OfficeInstall.as_str().to_string(),
                detail: "A fase office_install exige office_stage_odt concluída.".to_string(),
            },
        );
    }
    state.mark_phase_running(OfficePhase::OfficeInstall)?;

    let marker = match guest_executor::start_detached_and_wait_for_marker(
        profile,
        executor,
        office_odt::office_install_guest_script(),
        timeout,
    ) {
        Ok(marker) => marker,
        Err(err) => return fail_office_install(state, profile_dir, err),
    };
    match guest_executor::verify_office_install_marker(&marker) {
        Ok(verification) => {
            mark_office_install_done(&mut state, verification)?;
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_office_install(state, profile_dir, err),
    }
}

fn mark_office_install_done(
    state: &mut OfficeProvisioningState,
    verification: guest_executor::OfficeInstallVerification,
) -> Result<()> {
    let exe_paths = verification
        .office
        .exe_paths
        .as_ref()
        .map(|paths| {
            [
                paths.excel.clone(),
                paths.winword.clone(),
                paths.powerpnt.clone(),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    state.mark_phase_done(
        OfficePhase::OfficeInstall,
        Some(PhaseEvidence {
            marker: Some(OFFICE_INSTALL_MARKER.to_string()),
            exit_code: Some(verification.exit_code),
            files: exe_paths,
            registry: Some(serde_json::json!({
                "productReleaseIds": verification.office.product_release_ids,
                "versionToReport": verification.office.version_to_report,
                "platform": verification.office.platform,
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn fail_office_install(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    error: GuestPhaseError,
) -> Result<OfficeProvisioningState> {
    let code = error.code();
    state.mark_phase_failed(OfficeLastError {
        code: code.to_string(),
        message: office_install_error_message(code),
        phase: OfficePhase::OfficeInstall,
        retryable: true,
        details: Some(error.details_json()),
    });
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(error))
}

fn office_install_error_message(code: &str) -> String {
    match code {
        guest_executor::GUEST_PHASE_TIMEOUT_CODE => {
            "Instalação do Office não publicou marker antes do timeout.".to_string()
        }
        guest_executor::GUEST_DISK_FULL_CODE => {
            "Instalação do Office ficou sem espaço no guest.".to_string()
        }
        guest_executor::OFFICE_ODT_FAILED_CODE => {
            "Office Deployment Tool falhou ao instalar Microsoft 365 Apps.".to_string()
        }
        guest_executor::OFFICE_DETECTION_FAILED_CODE => {
            "Office não passou na verificação ClickToRun e executáveis.".to_string()
        }
        _ => "Executor guest falhou ao iniciar ou observar a instalação Office.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::guest_executor::mock::MockGuestExecutor;
    use crate::core::guest_executor::{GuestMarker, GuestMarkerStatus};
    use crate::core::office_odt::mock::MockOdtHost;
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
                office: None,
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

    #[test]
    fn odt_stage_host_failure_returns_office_odt_stage_failed() {
        let root = temp_dir("odt-fail");
        let profile_dir = root.join("profile");
        let shared_dir = root.join("shared");
        let mut state = OfficeProvisioningState::new("office");
        state
            .mark_phase_running(OfficePhase::RemoteappPrepare)
            .expect("remoteapp should run");
        state
            .mark_phase_done(OfficePhase::RemoteappPrepare, None)
            .expect("remoteapp should be done");
        state.save_to_dir(&profile_dir).expect("state should save");
        let config = office_odt::OfficeOdtConfig::new("O365ProPlusRetail", "pt-br", "Current")
            .expect("valid odt config");
        let host = MockOdtHost::new();
        host.fail_download("rede indisponível");

        let err = stage_odt_at(
            "office",
            &profile_dir,
            &shared_dir,
            &config,
            &OfficeOdtStageOptions::default(),
            &host,
        )
        .expect_err("host staging failure should return code");
        let message = format!("{err:#}");
        let state = OfficeProvisioningState::load_or_default(&profile_dir, "office")
            .expect("failed state should persist");

        assert!(message.contains(OFFICE_ODT_STAGE_FAILED_CODE));
        assert_eq!(
            state.phase_status(OfficePhase::OfficeStageOdt),
            Some(PhaseStatus::Failed)
        );
        let last_error = state.last_error.expect("last_error should be stored");
        assert_eq!(last_error.code, OFFICE_ODT_STAGE_FAILED_CODE);
        assert_eq!(last_error.phase, OfficePhase::OfficeStageOdt);
        assert!(last_error.retryable);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn odt_stage_reads_office_options_from_config_env() {
        let root = temp_dir("odt-env");
        let profile_name = "office";
        let profile_dir = root.join("profile");
        let shared_dir = root.join("shared");
        let env = env_file::parse(
            "PROFILE_KIND=office\n\
             OFFICE_PRODUCT_ID=O365BusinessRetail\n\
             OFFICE_LANGUAGE=en-us\n\
             OFFICE_CHANNEL=Current\n",
        );
        let config = office_odt::OfficeOdtConfig::from_env_map(&env)
            .expect("Office config should come from config.env values");
        let mut state = OfficeProvisioningState::new(profile_name);
        state
            .mark_phase_running(OfficePhase::RemoteappPrepare)
            .expect("remoteapp should run");
        state
            .mark_phase_done(OfficePhase::RemoteappPrepare, None)
            .expect("remoteapp should be done");
        state.save_to_dir(&profile_dir).expect("state should save");
        let host = MockOdtHost::new();

        let state = stage_odt_at(
            profile_name,
            &profile_dir,
            &shared_dir,
            &config,
            &OfficeOdtStageOptions::default(),
            &host,
        )
        .expect("odt stage should pass");
        let xml = std::fs::read_to_string(
            office_odt::odt_dir(&shared_dir).join(office_odt::OFFICE_CONFIGURATION_XML),
        )
        .expect("xml should be written");

        assert_eq!(
            state.phase_status(OfficePhase::OfficeStageOdt),
            Some(PhaseStatus::Done)
        );
        assert!(xml.contains("<Product ID=\"O365BusinessRetail\">"));
        assert!(xml.contains("<Language ID=\"en-us\" />"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_install_marker_done_updates_state() {
        let root = temp_dir("install-ok");
        let profile_dir = root.join("profile");
        let mut state = OfficeProvisioningState::new("office");
        for phase in [OfficePhase::RemoteappPrepare, OfficePhase::OfficeStageOdt] {
            state.mark_phase_running(phase).expect("phase should run");
            state
                .mark_phase_done(phase, None)
                .expect("phase should finish");
        }
        state.save_to_dir(&profile_dir).expect("state should save");
        let executor = MockGuestExecutor::new();
        executor.seed_marker(OFFICE_INSTALL_MARKER, Some(office_done_marker()));

        let state = install_office_at(
            "office",
            &profile_dir,
            &executor,
            GuestPhaseTimeout::immediate("office_install", OFFICE_INSTALL_MARKER),
        )
        .expect("office install marker should complete phase");

        assert_eq!(
            state.phase_status(OfficePhase::OfficeInstall),
            Some(PhaseStatus::Done)
        );
        let evidence = state
            .phases
            .get(&OfficePhase::OfficeInstall)
            .and_then(|phase| phase.evidence.as_ref())
            .expect("office install evidence should be stored");
        assert_eq!(evidence.marker.as_deref(), Some(OFFICE_INSTALL_MARKER));
        assert_eq!(evidence.exit_code, Some(0));
        assert_eq!(evidence.files.len(), 3);
        assert_eq!(executor.detached_runs().len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_install_timeout_persists_guest_phase_timeout() {
        let root = temp_dir("install-timeout");
        let profile_dir = root.join("profile");
        let mut state = OfficeProvisioningState::new("office");
        for phase in [OfficePhase::RemoteappPrepare, OfficePhase::OfficeStageOdt] {
            state.mark_phase_running(phase).expect("phase should run");
            state
                .mark_phase_done(phase, None)
                .expect("phase should finish");
        }
        state.save_to_dir(&profile_dir).expect("state should save");
        let executor = MockGuestExecutor::new();

        let err = install_office_at(
            "office",
            &profile_dir,
            &executor,
            GuestPhaseTimeout::immediate("office_install", OFFICE_INSTALL_MARKER),
        )
        .expect_err("missing marker should persist timeout");
        let message = format!("{err:#}");
        let state = OfficeProvisioningState::load_or_default(&profile_dir, "office")
            .expect("failed state should load");

        assert!(message.contains(guest_executor::GUEST_PHASE_TIMEOUT_CODE));
        assert_eq!(
            state.phase_status(OfficePhase::OfficeInstall),
            Some(PhaseStatus::Failed)
        );
        let last_error = state.last_error.expect("last_error should be stored");
        assert_eq!(last_error.code, guest_executor::GUEST_PHASE_TIMEOUT_CODE);
        assert_eq!(last_error.phase, OfficePhase::OfficeInstall);
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

    fn office_done_marker() -> GuestMarker {
        GuestMarker {
            phase: "office_install".to_string(),
            status: GuestMarkerStatus::Done,
            exit_code: Some(0),
            error: None,
            office: Some(guest_executor::OfficeInstallEvidence {
                product_release_ids: Some("O365ProPlusRetail".to_string()),
                version_to_report: Some("16.0.12345.67890".to_string()),
                platform: Some("x64".to_string()),
                exe_paths: Some(guest_executor::OfficeExePaths {
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
}

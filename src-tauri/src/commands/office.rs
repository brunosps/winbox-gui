use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use crate::commands::launch as vm_launch;
use crate::core::{
    docker::{CliDocker, DockerClient},
    env_file,
    flatpak::CliFlatpakClient,
    guest_executor::{
        self, CliGuestExecutor, GuestExecutor, GuestPhaseError, GuestPhaseTimeout,
        GuestRemoteappNotPrepared, REMOTEAPP_ACTION_HINT, REMOTEAPP_PREPARE_MARKER,
    },
    launch_error::OfficeError,
    office_odt::{
        self, CliOdtHost, OdtHost, OfficeOdtStage, OfficeOdtStageFailed, OfficeOdtStageOptions,
        OFFICE_INSTALL_MARKER,
    },
    office_preflight::{self, CliHostPreflight, OfficePreflightResult, Resources},
    office_state::{
        AdoptionState, ManagedPaths, OfficeLastError, OfficePhase, OfficeProfileStatus,
        OfficeProvisioningState, PhaseEvidence, PhaseState, PhaseStatus,
    },
    paths, telemetry,
    winapps::{self, CliRdpSessionProbe, CliWinAppsClient, RdpSessionProbe, WinAppsClient},
};

pub const OFFICE_PROGRESS_STEPS: &[&str] = &[
    "office_preflight",
    "office_byol",
    "office_windows_prepare",
    "office_windows_install",
    "office_rdp_wait",
    "office_odt_stage",
    "office_odt_install",
    "office_verify_install",
    "office_winapps_clone",
    "office_winapps_setup",
    "office_desktop_register",
    "office_file_association",
    "office_final_verify",
    "office_cold_start",
    "office_launch_remoteapp",
    "office_remove_winapps",
    "office_remove_desktop",
];

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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeNameArgs {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeRetryPhaseArgs {
    pub name: String,
    pub phase: OfficePhase,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedScope {
    #[serde(default)]
    pub manage_profile_config: bool,
    #[serde(default, rename = "manageWinAppsConf", alias = "manageWinappsConf")]
    pub manage_winapps_conf: bool,
    #[serde(default)]
    pub manage_desktop_entries: bool,
    #[serde(default)]
    pub manage_file_associations: bool,
    #[serde(default)]
    pub manage_disk_lifecycle: bool,
    #[serde(
        default,
        rename = "preserveExistingWinAppsClone",
        alias = "preserveExistingWinappsClone"
    )]
    pub preserve_existing_winapps_clone: bool,
}

impl From<winapps::AdoptionManagedScope> for ManagedScope {
    fn from(scope: winapps::AdoptionManagedScope) -> Self {
        Self {
            manage_profile_config: scope.manage_profile_config,
            manage_winapps_conf: scope.manage_winapps_conf,
            manage_desktop_entries: scope.manage_desktop_entries,
            manage_file_associations: scope.manage_file_associations,
            manage_disk_lifecycle: scope.manage_disk_lifecycle,
            preserve_existing_winapps_clone: scope.preserve_existing_winapps_clone,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeAdoptProfileArgs {
    pub name: String,
    #[serde(rename = "adoptionId", alias = "adoption_id")]
    pub adoption_id: String,
    #[serde(rename = "managedScope", alias = "managed_scope")]
    pub managed_scope: ManagedScope,
    pub confirm: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeLaunchAppArgs {
    pub name: String,
    #[serde(rename = "appId", alias = "app_id")]
    pub app_id: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default, rename = "guiProgress", alias = "gui_progress")]
    pub gui_progress: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeRemoveProfileArgs {
    pub name: String,
    #[serde(rename = "deleteDisk", alias = "delete_disk")]
    pub delete_disk: bool,
    #[serde(default, rename = "confirmToken", alias = "confirm_token")]
    pub confirm_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeTelemetrySetOptInArgs {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeStateResponse {
    pub status: OfficeProfileStatus,
    pub phases: BTreeMap<OfficePhase, PhaseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<OfficeLastError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_sessions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adoption: Option<AdoptionState>,
    pub managed_paths: ManagedPaths,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeProvisioningResponse {
    pub status: OfficeProfileStatus,
    pub phases: BTreeMap<OfficePhase, PhaseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<OfficeLastError>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeLaunchAppResponse {
    pub app_id: String,
    pub delegated_to: String,
    pub files_accepted: Vec<String>,
    pub active_sessions: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeRemoveProfileResponse {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserved_disk: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeTelemetryOptInResponse {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OfficeOperationProgress {
    pub profile: String,
    pub op: String,
    pub step: String,
    pub status: String,
    pub message: String,
}

pub fn preflight(args: OfficePreflightArgs) -> Result<OfficePreflightResult> {
    let result = office_preflight::run_preflight(
        args.name.as_deref(),
        &args.resources,
        &CliDocker,
        &CliFlatpakClient,
        &CliHostPreflight,
    )?;
    let profile = args.name.as_deref().unwrap_or("office");
    let probe_paths = winapps::AdoptionProbePaths::for_profile(profile);
    Ok(with_adoption_candidates(
        result,
        profile,
        &CliDocker,
        &probe_paths,
    ))
}

fn with_adoption_candidates(
    mut result: OfficePreflightResult,
    profile: &str,
    docker: &dyn DockerClient,
    probe_paths: &winapps::AdoptionProbePaths,
) -> OfficePreflightResult {
    result.adoption_candidates = winapps::detect_adoption_findings(profile, docker, probe_paths)
        .into_iter()
        .filter_map(|finding| serde_json::to_value(finding).ok())
        .collect();
    result
}

pub fn preflight_contract(
    args: OfficePreflightArgs,
) -> std::result::Result<OfficePreflightResult, OfficeError> {
    preflight(args).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::Preflight,
            false,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })
}

pub fn get_state_contract(
    args: OfficeNameArgs,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    get_state_with_probe(args, &CliRdpSessionProbe)
}

fn get_state_with_probe(
    args: OfficeNameArgs,
    probe: &dyn RdpSessionProbe,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    let profile_dir = paths::profile_cfg_dir(&args.name);
    let env_path = paths::profile_env_file(&args.name);
    get_state_at(args, &profile_dir, &env_path, probe)
}

fn get_state_at(
    args: OfficeNameArgs,
    profile_dir: &Path,
    env_path: &Path,
    probe: &dyn RdpSessionProbe,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    let mut state = load_office_state_at(profile_dir, &args.name)?;
    state.active_sessions = active_sessions_from_env(env_path, probe);
    Ok(state_response(&state))
}

pub fn start_provisioning_contract(
    args: OfficeStartProvisioningArgs,
) -> std::result::Result<OfficeProvisioningResponse, OfficeError> {
    if !args.byol_accepted {
        return Err(office_error(
            OfficeError::BYOL_NOT_ACCEPTED,
            OfficePhase::ByolAcceptance,
            false,
            serde_json::json!({ "detail": "Aceite BYOL é obrigatório antes do provisionamento." }),
        ));
    }
    args.resources.validate().map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            false,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    office_odt::OfficeOdtConfig::new(
        &args.product_id,
        &args.language,
        crate::core::validation::OFFICE_DEFAULT_CHANNEL,
    )
    .map_err(|err| {
        office_error(
            OfficeError::OFFICE_PRODUCT_INVALID,
            OfficePhase::ProfileConfig,
            false,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;

    if let Some(enabled) = args.telemetry_opt_in {
        let _ = telemetry::set_opt_in(enabled);
    }

    let profile_dir = paths::profile_cfg_dir(&args.name);
    let mut state =
        OfficeProvisioningState::load_or_default(&profile_dir, &args.name).map_err(|err| {
            office_error(
                OfficeError::PROFILE_STATE_CONFLICT,
                OfficePhase::ProfileConfig,
                true,
                serde_json::json!({ "detail": format!("{err:#}") }),
            )
        })?;
    state.byol.accepted = true;
    state.byol.accepted_at = Some(chrono::Utc::now().to_rfc3339());
    state.options.product_id = args.product_id;
    state.options.language = args.language;
    state.options.office_channel = crate::core::validation::OFFICE_DEFAULT_CHANNEL.to_string();
    mark_phase_done_idempotent(
        &mut state,
        OfficePhase::Preflight,
        Some(PhaseEvidence {
            registry: Some(serde_json::json!({ "contract": "office_start_provisioning" })),
            ..PhaseEvidence::default()
        }),
    )?;
    mark_phase_done_idempotent(&mut state, OfficePhase::ByolAcceptance, None)?;
    mark_phase_done_idempotent(&mut state, OfficePhase::ProfileConfig, None)?;
    state.save_to_dir(&profile_dir).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    let _ = telemetry::send_if_enabled_at(
        &telemetry::telemetry_preferences_path(),
        telemetry::TelemetryEventInput {
            event: "wizard_started".to_string(),
            phase: "office_byol".to_string(),
            duration_ms: None,
            error_code: None,
            language: state.options.language.clone(),
            product_id: state.options.product_id.clone(),
        },
        &telemetry::CurlTelemetrySink::default(),
    );
    Ok(provisioning_response(&state))
}

pub fn retry_phase_contract(
    args: OfficeRetryPhaseArgs,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    let profile_dir = paths::profile_cfg_dir(&args.name);
    let mut state = load_office_state_at(&profile_dir, &args.name)?;
    state.retry_from_phase(args.phase).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            args.phase,
            false,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    state.save_to_dir(&profile_dir).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            args.phase,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    Ok(state_response(&state))
}

pub fn adopt_profile_contract(
    args: OfficeAdoptProfileArgs,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    adopt_profile_with_executor(args, &CliGuestExecutor)
}

fn adopt_profile_with_executor(
    args: OfficeAdoptProfileArgs,
    executor: &dyn GuestExecutor,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    if !args.confirm {
        return Err(office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            false,
            serde_json::json!({ "detail": "Adoção exige confirmação explícita." }),
        ));
    }
    let profile_dir = paths::profile_cfg_dir(&args.name);
    adopt_profile_at(args, &profile_dir, executor)
}

fn adopt_profile_at(
    args: OfficeAdoptProfileArgs,
    profile_dir: &Path,
    executor: &dyn GuestExecutor,
) -> std::result::Result<OfficeStateResponse, OfficeError> {
    guest_executor::probe_remoteapp_channel(&args.name, executor).map_err(|err| {
        office_error(
            OfficeError::GUEST_REMOTEAPP_NOT_PREPARED,
            OfficePhase::RemoteappPrepare,
            true,
            serde_json::json!({
                "detail": format!("{err:#}"),
                "actionHint": REMOTEAPP_ACTION_HINT,
            }),
        )
    })?;
    let mut state =
        OfficeProvisioningState::load_or_default(profile_dir, &args.name).map_err(|err| {
            office_error(
                OfficeError::PROFILE_STATE_CONFLICT,
                OfficePhase::ProfileConfig,
                true,
                serde_json::json!({ "detail": format!("{err:#}") }),
            )
        })?;
    mark_adoption_skipped_phases(&mut state, &args.managed_scope).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    state.status = OfficeProfileStatus::Adopted;
    state.managed_paths.winapps_conf_owned = args.managed_scope.manage_winapps_conf;
    state.adoption = Some(AdoptionState {
        found: vec![serde_json::json!({
            "id": args.adoption_id,
            "kind": "profile",
            "status": "compatible",
            "evidence": "Adoção confirmada pelo usuário após revisão e no-op RemoteApp bem-sucedido.",
            "managedByDefault": args.managed_scope.manage_profile_config,
            "managedScope": args.managed_scope,
        })],
        user_confirmed_at: Some(chrono::Utc::now().to_rfc3339()),
    });
    state.save_to_dir(profile_dir).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    Ok(state_response(&state))
}

fn mark_adoption_skipped_phases(
    state: &mut OfficeProvisioningState,
    scope: &ManagedScope,
) -> Result<()> {
    for phase in adoption_skipped_phases(scope) {
        state.mark_phase_skipped(
            phase,
            Some(PhaseEvidence {
                registry: Some(serde_json::json!({
                    "adoption": true,
                    "managedScope": scope,
                })),
                ..PhaseEvidence::default()
            }),
        )?;
    }
    Ok(())
}

fn adoption_skipped_phases(scope: &ManagedScope) -> Vec<OfficePhase> {
    let mut phases = vec![
        OfficePhase::Preflight,
        OfficePhase::ByolAcceptance,
        OfficePhase::ProfileConfig,
        OfficePhase::WindowsPrepare,
        OfficePhase::WindowsInstall,
        OfficePhase::RemoteappPrepare,
    ];
    if !scope.manage_winapps_conf || scope.preserve_existing_winapps_clone {
        phases.push(OfficePhase::WinappsConfig);
    }
    if !scope.manage_desktop_entries {
        phases.push(OfficePhase::DesktopRegistration);
    }
    if !scope.manage_file_associations {
        phases.push(OfficePhase::FileAssociation);
    }
    phases
}

pub fn launch_app_contract(
    args: OfficeLaunchAppArgs,
) -> std::result::Result<OfficeLaunchAppResponse, OfficeError> {
    launch_app_with_clients(args, &CliDocker, &CliWinAppsClient, &CliOfficeLaunchHost)
}

fn launch_app_with_clients<D: DockerClient>(
    args: OfficeLaunchAppArgs,
    docker: &D,
    winapps_client: &dyn WinAppsClient,
    host: &dyn OfficeLaunchHost,
) -> std::result::Result<OfficeLaunchAppResponse, OfficeError> {
    let profile_dir = paths::profile_cfg_dir(&args.name);
    launch_app_at(args, &profile_dir, docker, winapps_client, host)
}

fn launch_app_at<D: DockerClient>(
    args: OfficeLaunchAppArgs,
    profile_dir: &Path,
    docker: &D,
    winapps_client: &dyn WinAppsClient,
    host: &dyn OfficeLaunchHost,
) -> std::result::Result<OfficeLaunchAppResponse, OfficeError> {
    let accepted = validate_launch_files(&args.files)?;
    let mut state = load_office_state_at(profile_dir, &args.name)?;
    let launcher = winapps::launcher_for_app_id(&args.app_id).ok_or_else(|| {
        office_error(
            OfficeError::APP_NOT_REGISTERED,
            OfficePhase::FirstLaunch,
            false,
            serde_json::json!({ "appId": args.app_id }),
        )
    })?;
    ensure_launch_prerequisites(&state, launcher)?;

    if args.gui_progress {
        let _ = host.spawn_progress_window(&args.name, &args.app_id);
    }

    host.notify(
        "winbox Office",
        &format!("Preparando {} para abrir {}.", args.name, args.app_id),
    );
    let transition = vm_launch::ensure_started(&args.name, docker)
        .map_err(|err| map_launch_error(err, "vm_start"))?;
    let cold_started = transition != vm_launch::ContainerTransition::AlreadyRunning;
    if cold_started {
        host.notify(
            "winbox Office",
            "VM Office iniciando; isso pode levar alguns minutos.",
        );
    }
    if transition == vm_launch::ContainerTransition::Recreated {
        let container = paths::profile_container(&args.name);
        vm_launch::wait_for_windows(&args.name, &container, docker)
            .map_err(|err| map_launch_error(err, "rdp_wait"))?;
    }

    host.notify("winbox Office", "Delegando app Office ao WinApps.");
    let launch = winapps::launch_office_app(winapps_client, &args.app_id, &accepted)?;
    mark_first_launch_done(&mut state, &args.app_id, &launch, cold_started).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    state.save_to_dir(profile_dir).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;

    Ok(OfficeLaunchAppResponse {
        app_id: args.app_id,
        delegated_to: launch.launcher,
        files_accepted: accepted,
        active_sessions: state.active_sessions,
    })
}

pub fn remove_profile_contract(
    args: OfficeRemoveProfileArgs,
) -> std::result::Result<OfficeRemoveProfileResponse, OfficeError> {
    remove_profile_with_client(args, &CliWinAppsClient)
}

fn remove_profile_with_client(
    args: OfficeRemoveProfileArgs,
    client: &dyn WinAppsClient,
) -> std::result::Result<OfficeRemoveProfileResponse, OfficeError> {
    let paths = OfficeRemovalPaths::for_profile(&args.name);
    remove_profile_at(args, &paths, client)
}

fn remove_profile_at(
    args: OfficeRemoveProfileArgs,
    removal_paths: &OfficeRemovalPaths,
    client: &dyn WinAppsClient,
) -> std::result::Result<OfficeRemoveProfileResponse, OfficeError> {
    let token = args.confirm_token.as_deref().unwrap_or("").trim();
    if token.is_empty() || !validate_remove_confirm_token(token, &args.name, args.delete_disk) {
        let reason = if token.is_empty() {
            "missing"
        } else {
            "invalid_or_used"
        };
        return Err(remove_requires_confirmation(
            &args.name,
            args.delete_disk,
            reason,
        ));
    }

    let mut state =
        OfficeProvisioningState::load_or_default(&removal_paths.profile_dir, &args.name).map_err(
            |err| {
                office_error(
                    OfficeError::PROFILE_STATE_CONFLICT,
                    OfficePhase::FirstLaunch,
                    true,
                    serde_json::json!({ "detail": format!("{err:#}") }),
                )
            },
        )?;
    let mut removed_paths = Vec::new();

    if should_uninstall_winapps(&state) {
        winapps::uninstall(client, &removal_paths.winapps_source_dir)?;
        if state.managed_paths.winapps_conf_owned {
            remove_file_if_exists(&removal_paths.winapps_conf_path, &mut removed_paths)?;
        }
    }

    let desktop_paths = state
        .managed_paths
        .desktop_files
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if manages_desktop_entries(&state) && !desktop_paths.is_empty() {
        let removal = winapps::remove_desktop_files(&desktop_paths)?;
        removed_paths.extend(removal.removed_files);
    }

    if manages_file_associations(&state) && !state.managed_paths.mime_types.is_empty() {
        let removal = winapps::remove_mime_associations(&removal_paths.mimeapps_path)?;
        removed_paths.push(removal.path);
    }

    let disk_removed = args.delete_disk && manages_disk_lifecycle(&state);
    if disk_removed {
        remove_dir_if_exists(&removal_paths.profile_data_dir, &mut removed_paths)?;
    }

    state.status = OfficeProfileStatus::Removed;
    state
        .save_to_dir(&removal_paths.profile_dir)
        .map_err(|err| {
            office_error(
                OfficeError::PROFILE_STATE_CONFLICT,
                OfficePhase::FirstLaunch,
                true,
                serde_json::json!({ "detail": format!("{err:#}") }),
            )
        })?;

    Ok(OfficeRemoveProfileResponse {
        state: "removed".to_string(),
        confirm_token: None,
        preserved_disk: Some(!disk_removed),
        removed_paths,
    })
}

pub fn telemetry_set_opt_in_contract(
    args: OfficeTelemetrySetOptInArgs,
) -> OfficeTelemetryOptInResponse {
    let enabled = telemetry::set_opt_in(args.enabled)
        .map(|prefs| prefs.enabled)
        .unwrap_or(false);
    OfficeTelemetryOptInResponse { enabled }
}

pub fn office_progress_payload(
    profile: &str,
    op: &str,
    step: &str,
    status: &str,
    message: impl Into<String>,
) -> OfficeOperationProgress {
    OfficeOperationProgress {
        profile: profile.to_string(),
        op: op.to_string(),
        step: step.to_string(),
        status: status.to_string(),
        message: message.into(),
    }
}

pub fn office_phase_progress_step(phase: OfficePhase) -> &'static str {
    match phase {
        OfficePhase::Preflight => "office_preflight",
        OfficePhase::ByolAcceptance => "office_byol",
        OfficePhase::ProfileConfig => "office_byol",
        OfficePhase::WindowsPrepare => "office_windows_prepare",
        OfficePhase::WindowsInstall => "office_windows_install",
        OfficePhase::RemoteappPrepare => "office_rdp_wait",
        OfficePhase::OfficeStageOdt => "office_odt_stage",
        OfficePhase::OfficeInstall => "office_odt_install",
        OfficePhase::WinappsConfig => "office_winapps_setup",
        OfficePhase::DesktopRegistration => "office_desktop_register",
        OfficePhase::FileAssociation => "office_file_association",
        OfficePhase::FinalVerify => "office_final_verify",
        OfficePhase::FirstLaunch => "office_launch_remoteapp",
    }
}

fn office_error(
    code: &str,
    phase: OfficePhase,
    retryable: bool,
    details: serde_json::Value,
) -> OfficeError {
    OfficeError::new(code, phase, retryable, Some(details))
}

fn state_conflict(err: anyhow::Error) -> OfficeError {
    office_error(
        OfficeError::PROFILE_STATE_CONFLICT,
        OfficePhase::ProfileConfig,
        true,
        serde_json::json!({ "detail": format!("{err:#}") }),
    )
}

fn mark_phase_done_idempotent(
    state: &mut OfficeProvisioningState,
    phase: OfficePhase,
    evidence: Option<PhaseEvidence>,
) -> std::result::Result<(), OfficeError> {
    if state.phase_status(phase) == Some(PhaseStatus::Done) {
        return Ok(());
    }
    state.mark_phase_running(phase).map_err(state_conflict)?;
    state
        .mark_phase_done(phase, evidence)
        .map_err(state_conflict)
}

fn load_office_state_at(
    profile_dir: &Path,
    profile: &str,
) -> std::result::Result<OfficeProvisioningState, OfficeError> {
    let state = OfficeProvisioningState::load_or_default(profile_dir, profile).map_err(|err| {
        office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::ProfileConfig,
            true,
            serde_json::json!({ "detail": format!("{err:#}") }),
        )
    })?;
    if state.profile_kind != "office" {
        return Err(office_error(
            OfficeError::PROFILE_NOT_OFFICE,
            OfficePhase::ProfileConfig,
            false,
            serde_json::json!({ "profileKind": state.profile_kind }),
        ));
    }
    Ok(state)
}

fn active_sessions_from_env(env_path: &Path, probe: &dyn RdpSessionProbe) -> Option<bool> {
    let env = env_file::read(env_path).ok()?;
    winapps::detect_active_rdp_sessions(probe, env_file::get_u16(&env, "RDP_PORT"))
}

fn state_response(state: &OfficeProvisioningState) -> OfficeStateResponse {
    OfficeStateResponse {
        status: state.summarize_status(),
        phases: state.phases.clone(),
        last_error: state.last_error.clone(),
        active_sessions: state.active_sessions,
        adoption: state.adoption.clone(),
        managed_paths: state.managed_paths.clone(),
    }
}

fn provisioning_response(state: &OfficeProvisioningState) -> OfficeProvisioningResponse {
    OfficeProvisioningResponse {
        status: state.summarize_status(),
        phases: state.phases.clone(),
        last_error: state.last_error.clone(),
    }
}

trait OfficeLaunchHost {
    fn notify(&self, title: &str, message: &str);
    fn spawn_progress_window(&self, profile: &str, app_id: &str) -> Result<()>;
}

#[derive(Debug, Clone, Copy)]
struct CliOfficeLaunchHost;

impl OfficeLaunchHost for CliOfficeLaunchHost {
    fn notify(&self, title: &str, message: &str) {
        let _ = Command::new("notify-send").arg(title).arg(message).status();
    }

    fn spawn_progress_window(&self, profile: &str, app_id: &str) -> Result<()> {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("winbox"));
        Command::new(exe)
            .arg("--window=office-progress")
            .arg(profile)
            .arg(app_id)
            .spawn()
            .map(|_| ())
            .map_err(|err| anyhow::anyhow!("não foi possível abrir janela office-progress: {err}"))
    }
}

fn ensure_launch_prerequisites(
    state: &OfficeProvisioningState,
    launcher: &str,
) -> std::result::Result<(), OfficeError> {
    if state.phase_status(OfficePhase::RemoteappPrepare) != Some(PhaseStatus::Done) {
        return Err(office_error(
            OfficeError::GUEST_REMOTEAPP_NOT_PREPARED,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({
                "requiredPhase": OfficePhase::RemoteappPrepare,
                "status": state.phase_status(OfficePhase::RemoteappPrepare),
                "actionHint": "Prepare RemoteApp pelo wizard ou abra o desktop via noVNC e execute C:\\OEM\\install.bat.",
            }),
        ));
    }
    if state.phase_status(OfficePhase::OfficeInstall) != Some(PhaseStatus::Done) {
        return Err(office_error(
            OfficeError::OFFICE_DETECTION_FAILED,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({
                "requiredPhase": OfficePhase::OfficeInstall,
                "status": state.phase_status(OfficePhase::OfficeInstall),
                "actionHint": "Conclua a instalação/verificação do Office antes de abrir apps.",
            }),
        ));
    }
    if state.phase_status(OfficePhase::WinappsConfig) != Some(PhaseStatus::Done) {
        return Err(office_error(
            OfficeError::APP_NOT_REGISTERED,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({
                "requiredPhase": OfficePhase::WinappsConfig,
                "status": state.phase_status(OfficePhase::WinappsConfig),
                "launcher": launcher,
                "actionHint": "Rode a configuração WinApps do perfil Office novamente.",
            }),
        ));
    }
    let launchers = state
        .phases
        .get(&OfficePhase::WinappsConfig)
        .and_then(|phase| phase.evidence.as_ref())
        .map(|evidence| evidence.launcher_ids.as_slice())
        .unwrap_or(&[]);
    if !launchers.iter().any(|registered| registered == launcher) {
        return Err(office_error(
            OfficeError::APP_NOT_REGISTERED,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({
                "launcher": launcher,
                "registeredLaunchers": launchers,
                "actionHint": "Reaplique winapps_config/desktop_registration para recriar o launcher ausente.",
            }),
        ));
    }
    Ok(())
}

fn map_launch_error(error: crate::core::launch_error::LaunchError, context: &str) -> OfficeError {
    let code = error.code();
    let office_code = if code == "timeout_windows" {
        OfficeError::GUEST_RDP_UNREACHABLE
    } else {
        OfficeError::OFFICE_WINDOWS_FAILED
    };
    office_error(
        office_code,
        OfficePhase::FirstLaunch,
        true,
        serde_json::json!({
            "context": context,
            "launchCode": code,
            "launchDetails": serde_json::to_value(&error).unwrap_or_else(|_| serde_json::json!({ "code": code })),
            "actionHint": launch_error_hint(code),
        }),
    )
}

fn launch_error_hint(code: &str) -> &'static str {
    match code {
        "timeout_windows" => "A VM iniciou, mas o Windows/RDP não ficou pronto; abra o viewer noVNC e verifique o boot.",
        "docker_missing" | "docker_daemon_down" => {
            "Corrija Docker/daemon e execute novamente o launcher Office."
        }
        "port_conflict" => "Libere a porta do perfil ou ajuste RDP_PORT antes de abrir o app.",
        _ => "Verifique o estado da VM Office e tente abrir o app novamente.",
    }
}

fn mark_first_launch_done(
    state: &mut OfficeProvisioningState,
    app_id: &str,
    launch: &winapps::WinAppsLaunch,
    cold_started: bool,
) -> Result<()> {
    state.mark_phase_done(
        OfficePhase::FirstLaunch,
        Some(PhaseEvidence {
            exit_code: Some(launch.exit_code),
            files: launch.files.clone(),
            launcher_ids: vec![launch.launcher.clone()],
            registry: Some(serde_json::json!({
                "appId": app_id,
                "delegatedTo": "winapps",
                "coldStarted": cold_started,
                "activationDetection": "not_available_static_guidance",
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn validate_launch_files(files: &[String]) -> std::result::Result<Vec<String>, OfficeError> {
    let home = std::env::var("HOME").unwrap_or_default();
    let home_path = Path::new(&home);
    let mut accepted = Vec::with_capacity(files.len());
    for file in files {
        let path = Path::new(file);
        let has_parent_dir = path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir));
        if home_path.as_os_str().is_empty()
            || !path.is_absolute()
            || has_parent_dir
            || !path.starts_with(home_path)
        {
            return Err(office_error(
                OfficeError::FILE_OUTSIDE_HOME,
                OfficePhase::FirstLaunch,
                false,
                serde_json::json!({
                    "path": file,
                    "actionHint": "+home-drive só expõe $HOME ao guest; mova o arquivo para dentro do seu home.",
                }),
            ));
        }
        accepted.push(file.clone());
    }
    Ok(accepted)
}

fn remove_requires_confirmation(profile: &str, delete_disk: bool, reason: &str) -> OfficeError {
    office_error(
        OfficeError::REMOVE_REQUIRES_CONFIRMATION,
        OfficePhase::FirstLaunch,
        false,
        serde_json::json!({
            "confirmToken": mint_remove_confirm_token(profile, delete_disk),
            "deleteDisk": delete_disk,
            "reason": reason,
            "actionHint": "Execute novamente informando --confirm <token>. Use --delete-disk somente se quiser apagar o disco do perfil.",
        }),
    )
}

fn mint_remove_confirm_token(profile: &str, delete_disk: bool) -> String {
    let nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let token = format!(
        "office-remove:{}:{}:{}:{}",
        std::process::id(),
        profile,
        delete_disk,
        nanos
    );
    let expires_at = chrono::Utc::now().timestamp() + REMOVE_CONFIRM_TOKEN_TTL_SECONDS;
    remove_confirm_tokens()
        .lock()
        .expect("remove token store poisoned")
        .insert(
            token.clone(),
            RemoveConfirmToken {
                profile: profile.to_string(),
                delete_disk,
                expires_at,
            },
        );
    token
}

fn validate_remove_confirm_token(token: &str, profile: &str, delete_disk: bool) -> bool {
    let Some(stored) = remove_confirm_tokens()
        .lock()
        .expect("remove token store poisoned")
        .remove(token)
    else {
        return false;
    };
    stored.profile == profile
        && stored.delete_disk == delete_disk
        && stored.expires_at >= chrono::Utc::now().timestamp()
}

fn remove_confirm_tokens() -> &'static Mutex<BTreeMap<String, RemoveConfirmToken>> {
    static TOKENS: OnceLock<Mutex<BTreeMap<String, RemoveConfirmToken>>> = OnceLock::new();
    TOKENS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

const REMOVE_CONFIRM_TOKEN_TTL_SECONDS: i64 = 15 * 60;

#[derive(Debug, Clone)]
struct RemoveConfirmToken {
    profile: String,
    delete_disk: bool,
    expires_at: i64,
}

#[derive(Debug, Clone)]
struct OfficeRemovalPaths {
    profile_dir: PathBuf,
    profile_data_dir: PathBuf,
    winapps_source_dir: PathBuf,
    winapps_conf_path: PathBuf,
    mimeapps_path: PathBuf,
}

impl OfficeRemovalPaths {
    fn for_profile(profile: &str) -> Self {
        Self {
            profile_dir: paths::profile_cfg_dir(profile),
            profile_data_dir: paths::profile_data_dir(profile),
            winapps_source_dir: winapps::managed_source_dir(),
            winapps_conf_path: winapps::winapps_conf_path(),
            mimeapps_path: winapps::mimeapps_path(),
        }
    }
}

fn should_uninstall_winapps(state: &OfficeProvisioningState) -> bool {
    state.managed_paths.winapps_conf_owned
        && state.phase_status(OfficePhase::WinappsConfig) == Some(PhaseStatus::Done)
        && manages_winapps_conf(state)
}

fn manages_winapps_conf(state: &OfficeProvisioningState) -> bool {
    managed_scope_bool(state, "manageWinAppsConf", true)
}

fn manages_desktop_entries(state: &OfficeProvisioningState) -> bool {
    managed_scope_bool(state, "manageDesktopEntries", true)
}

fn manages_file_associations(state: &OfficeProvisioningState) -> bool {
    managed_scope_bool(state, "manageFileAssociations", true)
}

fn manages_disk_lifecycle(state: &OfficeProvisioningState) -> bool {
    managed_scope_bool(state, "manageDiskLifecycle", true)
}

fn managed_scope_bool(state: &OfficeProvisioningState, key: &str, default: bool) -> bool {
    state
        .adoption
        .as_ref()
        .and_then(|adoption| adoption.found.first())
        .and_then(|finding| finding.get("managedScope"))
        .and_then(|scope| scope.get(key))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(default)
}

fn remove_file_if_exists(
    path: &Path,
    removed_paths: &mut Vec<String>,
) -> std::result::Result<(), OfficeError> {
    match std::fs::remove_file(path) {
        Ok(()) => {
            removed_paths.push(path.display().to_string());
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({
                "path": path,
                "detail": format!("não foi possível remover arquivo gerenciado: {err}"),
            }),
        )),
    }
}

fn remove_dir_if_exists(
    path: &Path,
    removed_paths: &mut Vec<String>,
) -> std::result::Result<(), OfficeError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {
            removed_paths.push(path.display().to_string());
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(office_error(
            OfficeError::PROFILE_STATE_CONFLICT,
            OfficePhase::FirstLaunch,
            true,
            serde_json::json!({
                "path": path,
                "detail": format!("não foi possível remover diretório gerenciado: {err}"),
            }),
        )),
    }
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

pub fn configure_winapps(profile: &str) -> Result<OfficeProvisioningState> {
    configure_winapps_with_client(profile, &CliWinAppsClient)
}

pub fn register_desktop(profile: &str) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    register_desktop_at(profile, &profile_dir, &winapps::desktop_applications_dir())
}

pub fn register_file_associations(profile: &str) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    register_file_associations_at(profile, &profile_dir, &winapps::mimeapps_path())
}

pub fn final_verify(profile: &str) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    final_verify_at(
        profile,
        &profile_dir,
        &winapps::desktop_applications_dir(),
        &winapps::mimeapps_path(),
    )
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

pub fn configure_winapps_with_client(
    profile: &str,
    client: &dyn WinAppsClient,
) -> Result<OfficeProvisioningState> {
    let profile_dir = paths::profile_cfg_dir(profile);
    let env_path = paths::profile_env_file(profile);
    configure_winapps_at(profile, &profile_dir, &env_path, client)
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
        code: OfficeError::GUEST_REMOTEAPP_NOT_PREPARED.to_string(),
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
        code: OfficeError::OFFICE_ODT_STAGE_FAILED.to_string(),
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
        OfficeError::GUEST_PHASE_TIMEOUT => {
            "Instalação do Office não publicou marker antes do timeout.".to_string()
        }
        OfficeError::GUEST_DISK_FULL => {
            "Instalação do Office ficou sem espaço no guest.".to_string()
        }
        OfficeError::OFFICE_ODT_FAILED => {
            "Office Deployment Tool falhou ao instalar Microsoft 365 Apps.".to_string()
        }
        OfficeError::OFFICE_DETECTION_FAILED => {
            "Office não passou na verificação ClickToRun e executáveis.".to_string()
        }
        _ => "Executor guest falhou ao iniciar ou observar a instalação Office.".to_string(),
    }
}

fn configure_winapps_at(
    profile: &str,
    profile_dir: &Path,
    env_path: &Path,
    client: &dyn WinAppsClient,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    state.ensure_remoteapp_ready_for_guest_phase(OfficePhase::WinappsConfig)?;
    if state.phase_status(OfficePhase::OfficeInstall) != Some(PhaseStatus::Done) {
        return fail_winapps_config(
            state,
            profile_dir,
            OfficeError::new(
                OfficeError::WINAPPS_NO_CONFIG,
                OfficePhase::WinappsConfig,
                true,
                Some(serde_json::json!({
                    "detail": "A fase winapps_config exige office_install concluída.",
                })),
            ),
        );
    }
    state.mark_phase_running(OfficePhase::WinappsConfig)?;

    let map = env_file::read(env_path)?;
    let config = match winapps::WinAppsConfig::from_env_map(&map) {
        Ok(config) => config,
        Err(err) => {
            return fail_winapps_config(
                state,
                profile_dir,
                OfficeError::new(
                    OfficeError::WINAPPS_NO_CONFIG,
                    OfficePhase::WinappsConfig,
                    true,
                    Some(serde_json::json!({ "detail": format!("{err:#}") })),
                ),
            );
        }
    };
    let paths = winapps::WinAppsPaths::managed();
    match winapps::configure_winapps(&config, &paths, client) {
        Ok(setup) => {
            mark_winapps_done(&mut state, setup)?;
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_winapps_config(state, profile_dir, err),
    }
}

fn register_desktop_at(
    profile: &str,
    profile_dir: &Path,
    applications_dir: &Path,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    if state.phase_status(OfficePhase::DesktopRegistration) == Some(PhaseStatus::Done) {
        return Ok(state);
    }
    if state.phase_status(OfficePhase::WinappsConfig) != Some(PhaseStatus::Done) {
        return fail_desktop_registration(
            state,
            profile_dir,
            OfficeError::new(
                OfficeError::DESKTOP_REGISTRATION_FAILED,
                OfficePhase::DesktopRegistration,
                true,
                Some(serde_json::json!({
                    "detail": "A fase desktop_registration exige winapps_config concluída.",
                })),
            ),
        );
    }
    state.mark_phase_running(OfficePhase::DesktopRegistration)?;

    match winapps::register_desktop_launchers(profile, applications_dir) {
        Ok(registration) => {
            mark_desktop_registration_done(&mut state, registration)?;
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_desktop_registration(state, profile_dir, err),
    }
}

fn register_file_associations_at(
    profile: &str,
    profile_dir: &Path,
    mimeapps_path: &Path,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    if state.phase_status(OfficePhase::FileAssociation) == Some(PhaseStatus::Done) {
        return Ok(state);
    }
    if state.phase_status(OfficePhase::DesktopRegistration) != Some(PhaseStatus::Done) {
        return fail_file_association(
            state,
            profile_dir,
            OfficeError::new(
                OfficeError::FILE_ASSOCIATION_FAILED,
                OfficePhase::FileAssociation,
                true,
                Some(serde_json::json!({
                    "detail": "A fase file_association exige desktop_registration concluída.",
                })),
            ),
        );
    }
    state.mark_phase_running(OfficePhase::FileAssociation)?;

    match winapps::register_mime_associations(mimeapps_path) {
        Ok(registration) => {
            mark_file_association_done(&mut state, registration)?;
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_file_association(state, profile_dir, err),
    }
}

fn final_verify_at(
    profile: &str,
    profile_dir: &Path,
    applications_dir: &Path,
    mimeapps_path: &Path,
) -> Result<OfficeProvisioningState> {
    let mut state = OfficeProvisioningState::load_or_default(profile_dir, profile)?;
    if state.phase_status(OfficePhase::FinalVerify) == Some(PhaseStatus::Done) {
        return Ok(state);
    }
    if state.phase_status(OfficePhase::FileAssociation) != Some(PhaseStatus::Done) {
        return fail_final_verify(
            state,
            profile_dir,
            OfficeError::new(
                OfficeError::FILE_ASSOCIATION_FAILED,
                OfficePhase::FinalVerify,
                true,
                Some(serde_json::json!({
                    "detail": "A fase final_verify exige file_association concluída.",
                })),
            ),
        );
    }
    state.mark_phase_running(OfficePhase::FinalVerify)?;

    match winapps::final_verify(&state, applications_dir, mimeapps_path) {
        Ok(report) => {
            mark_final_verify_done(&mut state, report)?;
            state.save_to_dir(profile_dir)?;
            Ok(state)
        }
        Err(err) => fail_final_verify(state, profile_dir, err),
    }
}

fn mark_winapps_done(
    state: &mut OfficeProvisioningState,
    setup: winapps::WinAppsSetup,
) -> Result<()> {
    state.managed_paths.winapps_conf_owned = true;
    state.mark_phase_done(
        OfficePhase::WinappsConfig,
        Some(PhaseEvidence {
            files: vec![
                setup.source_dir.display().to_string(),
                setup.config_path.display().to_string(),
            ],
            launcher_ids: setup.launchers,
            registry: Some(serde_json::json!({
                "commit": setup.commit,
                "exitCode": setup.exit_code,
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn mark_desktop_registration_done(
    state: &mut OfficeProvisioningState,
    registration: winapps::DesktopRegistration,
) -> Result<()> {
    state.managed_paths.desktop_files = registration.desktop_files.clone();
    state.mark_phase_done(
        OfficePhase::DesktopRegistration,
        Some(PhaseEvidence {
            files: registration.desktop_files,
            launcher_ids: registration.launchers,
            ..PhaseEvidence::default()
        }),
    )
}

fn mark_file_association_done(
    state: &mut OfficeProvisioningState,
    registration: winapps::FileAssociationRegistration,
) -> Result<()> {
    state.managed_paths.mime_types = registration.mime_types.clone();
    state.mark_phase_done(
        OfficePhase::FileAssociation,
        Some(PhaseEvidence {
            files: registration.desktop_files,
            registry: Some(serde_json::json!({
                "extensions": registration.extensions,
                "mimeTypes": registration.mime_types,
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn mark_final_verify_done(
    state: &mut OfficeProvisioningState,
    report: winapps::FinalVerifyReport,
) -> Result<()> {
    state.managed_paths.desktop_files = report.desktop_files.clone();
    state.managed_paths.mime_types = report.mime_types.clone();
    state.mark_phase_done(
        OfficePhase::FinalVerify,
        Some(PhaseEvidence {
            files: report.desktop_files,
            launcher_ids: report.launchers,
            registry: Some(serde_json::json!({
                "officePresent": report.office_present,
                "rdpReady": report.rdp_ready,
                "winappsReady": report.winapps_ready,
                "mimeTypes": report.mime_types,
            })),
            ..PhaseEvidence::default()
        }),
    )
}

fn fail_winapps_config(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    error: OfficeError,
) -> Result<OfficeProvisioningState> {
    let code = error.code();
    state.mark_phase_failed(OfficeLastError {
        code: code.to_string(),
        message: winapps::winapps_error_message(code).to_string(),
        phase: OfficePhase::WinappsConfig,
        retryable: true,
        details: error.fields().details.clone(),
    });
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(error))
}

fn fail_desktop_registration(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    error: OfficeError,
) -> Result<OfficeProvisioningState> {
    fail_office_host_phase(
        &mut state,
        OfficePhase::DesktopRegistration,
        &error,
        winapps::winapps_error_message(error.code()),
    );
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(error))
}

fn fail_file_association(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    error: OfficeError,
) -> Result<OfficeProvisioningState> {
    fail_office_host_phase(
        &mut state,
        OfficePhase::FileAssociation,
        &error,
        winapps::winapps_error_message(error.code()),
    );
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(error))
}

fn fail_final_verify(
    mut state: OfficeProvisioningState,
    profile_dir: &Path,
    error: OfficeError,
) -> Result<OfficeProvisioningState> {
    fail_office_host_phase(
        &mut state,
        OfficePhase::FinalVerify,
        &error,
        winapps::winapps_error_message(error.code()),
    );
    state.save_to_dir(profile_dir)?;
    Err(anyhow::anyhow!(error))
}

fn fail_office_host_phase(
    state: &mut OfficeProvisioningState,
    phase: OfficePhase,
    error: &OfficeError,
    message: &str,
) {
    state.mark_phase_failed(OfficeLastError {
        code: error.code().to_string(),
        message: message.to_string(),
        phase,
        retryable: error.fields().retryable,
        details: error.fields().details.clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::docker::mock::MockDocker;
    use crate::core::guest_executor::mock::MockGuestExecutor;
    use crate::core::guest_executor::{GuestMarker, GuestMarkerStatus};
    use crate::core::launch_error::LaunchError;
    use crate::core::office_odt::mock::MockOdtHost;
    use crate::core::office_state::PhaseStatus;
    use crate::core::winapps::mock::MockWinAppsClient;
    use std::cell::RefCell;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MockRdpSessionProbe {
        pgrep: anyhow::Result<winapps::RdpSessionCommandOutput>,
        ss: anyhow::Result<winapps::RdpSessionCommandOutput>,
    }

    impl RdpSessionProbe for MockRdpSessionProbe {
        fn pgrep_xfreerdp(&self) -> anyhow::Result<winapps::RdpSessionCommandOutput> {
            self.pgrep
                .as_ref()
                .map(Clone::clone)
                .map_err(|err| anyhow::anyhow!("{err:#}"))
        }

        fn ss_tcp_processes(&self) -> anyhow::Result<winapps::RdpSessionCommandOutput> {
            self.ss
                .as_ref()
                .map(Clone::clone)
                .map_err(|err| anyhow::anyhow!("{err:#}"))
        }
    }

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
    fn byol_backend_rejects_missing_acceptance() {
        let err = start_provisioning_contract(OfficeStartProvisioningArgs {
            name: "office".to_string(),
            product_id: "O365ProPlusRetail".to_string(),
            language: "pt-br".to_string(),
            resources: Resources {
                ram_gb: 8,
                cpu_cores: 4,
                disk_gb: 128,
                storage_path: None,
                warning_override: true,
            },
            byol_accepted: false,
            telemetry_opt_in: None,
            adoption_id: None,
        })
        .expect_err("backend must reject provisioning without BYOL acceptance");

        assert_eq!(err.code(), OfficeError::BYOL_NOT_ACCEPTED);
        assert_eq!(err.fields().phase, OfficePhase::ByolAcceptance);
        assert!(!err.fields().retryable);
    }

    #[test]
    fn office_get_state_populates_active_sessions() {
        let root = temp_dir("state-active-sessions");
        let profile_dir = root.join("profile");
        std::fs::create_dir_all(&profile_dir).expect("profile dir should exist");
        let env_path = profile_dir.join("config.env");
        std::fs::write(&env_path, "PROFILE_KIND=office\nRDP_PORT=3391\n")
            .expect("env should be written");
        let mut state = OfficeProvisioningState::new("office");
        state
            .save_to_dir(&profile_dir)
            .expect("state should be saved");
        let probe = MockRdpSessionProbe {
            pgrep: Ok(rdp_ok("1234 xfreerdp /v:127.0.0.1:3391\n")),
            ss: Ok(rdp_ok(
                "ESTAB 0 0 127.0.0.1:52122 127.0.0.1:3391 users:((\"xfreerdp\",pid=1234,fd=7))\n",
            )),
        };

        let response = get_state_at(
            OfficeNameArgs {
                name: "office".into(),
            },
            &profile_dir,
            &env_path,
            &probe,
        )
        .expect("state should load");

        assert_eq!(response.active_sessions, Some(true));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_preflight_returns_adoption_candidates() {
        let root = temp_dir("preflight-adoption");
        let profile_env = root.join("profile").join("config.env");
        std::fs::create_dir_all(profile_env.parent().expect("profile parent should exist"))
            .expect("profile parent should be created");
        std::fs::write(&profile_env, "PROFILE_KIND=office\nRDP_PORT=3391\n")
            .expect("profile env should be written");
        let probe_paths = winapps::AdoptionProbePaths {
            winapps_conf_path: root.join("missing-winapps.conf"),
            winapps_clone_dir: root.join("missing-winapps-src"),
            applications_dir: root.join("missing-applications"),
            profile_env_file: profile_env,
        };
        let docker = MockDocker::new();
        docker.seed_status("winbox-windows", "running");
        let result = OfficePreflightResult {
            checks: Vec::new(),
            warnings: 0,
            blockers: 0,
            adoption_candidates: Vec::new(),
        };

        let result = with_adoption_candidates(result, "office", &docker, &probe_paths);

        assert_eq!(result.adoption_candidates.len(), 2);
        assert!(result
            .adoption_candidates
            .iter()
            .any(|candidate| candidate["kind"] == "container"
                && candidate["id"] == "container:winbox-windows"
                && candidate["managedByDefault"] == false));
        assert!(
            result
                .adoption_candidates
                .iter()
                .any(|candidate| candidate["kind"] == "rdp_port"
                    && candidate["id"] == "rdp_port:3391")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn operation_progress_office_error_status_shape() {
        let payload = office_progress_payload(
            "office",
            "office_provision",
            "office_odt_install",
            "error",
            "ODT falhou",
        );
        let json = serde_json::to_value(&payload).expect("payload should serialize");

        assert_eq!(json["profile"], "office");
        assert_eq!(json["op"], "office_provision");
        assert_eq!(json["step"], "office_odt_install");
        assert_eq!(json["status"], "error");
        assert_eq!(json["message"], "ODT falhou");
        assert_eq!(
            office_phase_progress_step(OfficePhase::FinalVerify),
            "office_final_verify"
        );
        assert!(OFFICE_PROGRESS_STEPS.contains(&"office_final_verify"));
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

        assert!(message.contains(OfficeError::GUEST_REMOTEAPP_NOT_PREPARED));
        assert!(message.contains("C:\\OEM\\install.bat"));
        assert_eq!(
            state.phase_status(OfficePhase::RemoteappPrepare),
            Some(PhaseStatus::Failed)
        );
        let last_error = state.last_error.expect("last_error should be stored");
        assert_eq!(last_error.code, OfficeError::GUEST_REMOTEAPP_NOT_PREPARED);
        assert_eq!(last_error.phase, OfficePhase::RemoteappPrepare);
        assert!(last_error.retryable);
        assert!(oem_dir.join("install.bat").is_file());
        assert!(oem_dir
            .join(guest_executor::REMOTEAPP_PREPARE_SCRIPT)
            .is_file());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn adoption_requires_remoteapp_noop_success() {
        let root = temp_dir("adoption-review");
        let profile_dir = root.join("profile");
        let args = OfficeAdoptProfileArgs {
            name: "office".to_string(),
            adoption_id: "manual-existing".to_string(),
            managed_scope: manual_user_assets_scope(),
            confirm: true,
        };
        let failing_executor = MockGuestExecutor::new();
        failing_executor.fail_next_run("RemoteApp /app não preparado");

        let err = adopt_profile_at(args.clone(), &profile_dir, &failing_executor)
            .expect_err("adoption must require a RemoteApp no-op");

        assert_eq!(err.code(), OfficeError::GUEST_REMOTEAPP_NOT_PREPARED);
        assert_eq!(err.fields().phase, OfficePhase::RemoteappPrepare);
        assert_eq!(failing_executor.runs().len(), 1);
        assert!(!crate::core::office_state::state_path(&profile_dir).exists());

        let executor = MockGuestExecutor::new();
        let response =
            adopt_profile_at(args, &profile_dir, &executor).expect("adoption should be persisted");
        let state = OfficeProvisioningState::load_or_default(&profile_dir, "office")
            .expect("adopted state should load");

        assert_eq!(executor.runs().len(), 1);
        assert!(executor.detached_runs().is_empty());
        assert_eq!(response.status, OfficeProfileStatus::Adopted);
        assert_eq!(
            response.phases[&OfficePhase::RemoteappPrepare].status,
            PhaseStatus::Skipped
        );
        assert_eq!(
            response.phases[&OfficePhase::WinappsConfig].status,
            PhaseStatus::Skipped
        );
        assert_eq!(
            response.phases[&OfficePhase::DesktopRegistration].status,
            PhaseStatus::Skipped
        );
        assert_eq!(
            response.phases[&OfficePhase::FileAssociation].status,
            PhaseStatus::Skipped
        );
        assert!(!state.managed_paths.winapps_conf_owned);
        let adoption = state.adoption.expect("adoption review should be stored");
        assert_eq!(adoption.found.len(), 1);
        assert_eq!(adoption.found[0]["id"], "manual-existing");
        assert_eq!(
            adoption.found[0]["managedScope"]["manageWinAppsConf"],
            false
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_launch_file_outside_home_returns_hint() {
        let err = launch_app_contract(OfficeLaunchAppArgs {
            name: "office".to_string(),
            app_id: "excel".to_string(),
            files: vec!["/__winbox_outside_home/planilha.xlsx".to_string()],
            gui_progress: false,
        })
        .expect_err("files outside HOME must be rejected before launch");

        assert_eq!(err.code(), OfficeError::FILE_OUTSIDE_HOME);
        assert_eq!(err.fields().phase, OfficePhase::FirstLaunch);
        let details = err.fields().details.as_ref().expect("details required");
        assert!(details["actionHint"]
            .as_str()
            .expect("hint should be string")
            .contains("+home-drive só expõe $HOME"));
    }

    #[test]
    fn launch_emits_cold_start_and_remoteapp_steps() {
        let root = temp_dir("launch-cold-start");
        let profile_dir = root.join("profile");
        let profile = "office-task16-cold";
        seed_launch_ready_state(profile, &profile_dir);
        let docker = MockDocker::new();
        let container = paths::profile_container(profile);
        docker.seed_status(&container, "absent");
        docker.seed_logs_contains(&container, "windows started successfully", true);
        let winapps = MockWinAppsClient::new();
        winapps.push_launch(winapps_command_ok(""));
        let host = MockLaunchHost::default();

        let response = launch_app_at(
            OfficeLaunchAppArgs {
                name: profile.to_string(),
                app_id: "excel".to_string(),
                files: vec![home_file("office-launch.xlsx")],
                gui_progress: true,
            },
            &profile_dir,
            &docker,
            &winapps,
            &host,
        )
        .expect("cold-start launch should delegate to WinApps");

        assert_eq!(response.app_id, "excel");
        assert_eq!(response.delegated_to, "excel-o365");
        assert_eq!(response.files_accepted.len(), 1);
        assert!(docker
            .calls()
            .iter()
            .any(|call| call == &format!("compose:{profile}:up -d")));
        assert!(
            docker
                .calls()
                .iter()
                .any(|call| call
                    == &format!("logs_contains:{container}:windows started successfully"))
        );
        assert!(winapps
            .calls()
            .contains(&format!("launch:excel-o365:{}", response.files_accepted[0])));
        assert_eq!(
            host.spawns(),
            vec![format!("--window=office-progress {profile} excel")]
        );
        let notifications = host.notifications();
        assert!(notifications
            .iter()
            .any(|message| message.contains("VM Office iniciando")));
        assert!(notifications
            .iter()
            .any(|message| message.contains("Delegando app Office ao WinApps")));
        let state = OfficeProvisioningState::load_or_default(&profile_dir, profile)
            .expect("state should persist first launch");
        assert_eq!(
            state.phase_status(OfficePhase::FirstLaunch),
            Some(PhaseStatus::Done)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn launch_direct_when_vm_already_running_skips_cold_start_wait() {
        let root = temp_dir("launch-running");
        let profile_dir = root.join("profile");
        let profile = "office-task16-running";
        seed_launch_ready_state(profile, &profile_dir);
        let docker = MockDocker::new();
        let container = paths::profile_container(profile);
        docker.seed_status(&container, "running");
        let winapps = MockWinAppsClient::new();
        winapps.push_launch(winapps_command_ok(""));
        let host = MockLaunchHost::default();

        let response = launch_app_at(
            OfficeLaunchAppArgs {
                name: profile.to_string(),
                app_id: "word".to_string(),
                files: Vec::new(),
                gui_progress: false,
            },
            &profile_dir,
            &docker,
            &winapps,
            &host,
        )
        .expect("running VM should launch directly");

        assert_eq!(response.delegated_to, "word-o365");
        assert!(!docker
            .calls()
            .iter()
            .any(|call| call.starts_with("logs_contains:")));
        assert!(host.spawns().is_empty());
        let notifications = host.notifications();
        assert!(!notifications
            .iter()
            .any(|message| message.contains("VM Office iniciando")));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn launch_failure_reports_vm_rdp_winapps_or_app_context() {
        let root = temp_dir("launch-failures");
        let profile_dir = root.join("profile");
        let profile = "office-task16-failures";
        seed_launch_ready_state(profile, &profile_dir);
        let docker = MockDocker::new();
        docker.set_compose_failure(LaunchError::PortConflict { port: 3390 });
        let err = launch_app_at(
            OfficeLaunchAppArgs {
                name: profile.to_string(),
                app_id: "excel".to_string(),
                files: Vec::new(),
                gui_progress: false,
            },
            &profile_dir,
            &docker,
            &MockWinAppsClient::new(),
            &MockLaunchHost::default(),
        )
        .expect_err("compose failure should preserve launch context");
        assert_eq!(err.code(), OfficeError::OFFICE_WINDOWS_FAILED);
        assert_eq!(
            err.fields()
                .details
                .as_ref()
                .and_then(|details| details.get("launchCode"))
                .and_then(serde_json::Value::as_str),
            Some("port_conflict")
        );

        let docker = MockDocker::new();
        docker.seed_status(&paths::profile_container(profile), "running");
        let winapps = MockWinAppsClient::new();
        winapps.push_launch(winapps_command_fail(7, "RDP caiu"));
        let err = launch_app_at(
            OfficeLaunchAppArgs {
                name: profile.to_string(),
                app_id: "powerpoint".to_string(),
                files: Vec::new(),
                gui_progress: false,
            },
            &profile_dir,
            &docker,
            &winapps,
            &MockLaunchHost::default(),
        )
        .expect_err("WinApps launch failure should be actionable");
        assert_eq!(err.code(), OfficeError::APP_LAUNCH_FAILED);
        assert_eq!(
            err.fields()
                .details
                .as_ref()
                .and_then(|details| details.get("launcher"))
                .and_then(serde_json::Value::as_str),
            Some("powerpoint-o365")
        );

        let err = launch_app_at(
            OfficeLaunchAppArgs {
                name: profile.to_string(),
                app_id: "access".to_string(),
                files: Vec::new(),
                gui_progress: false,
            },
            &profile_dir,
            &MockDocker::new(),
            &MockWinAppsClient::new(),
            &MockLaunchHost::default(),
        )
        .expect_err("unsupported app should not reach Docker or WinApps");
        assert_eq!(err.code(), OfficeError::APP_NOT_REGISTERED);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn office_remove_requires_single_use_token() {
        let root = temp_dir("remove-token");
        let removal_paths = removal_paths_under(&root);
        seed_managed_removal_state("office", &removal_paths);
        let client = MockWinAppsClient::new();

        let missing = remove_profile_at(
            OfficeRemoveProfileArgs {
                name: "office".to_string(),
                delete_disk: true,
                confirm_token: None,
            },
            &removal_paths,
            &client,
        )
        .expect_err("first call should require confirmation");
        assert_eq!(missing.code(), OfficeError::REMOVE_REQUIRES_CONFIRMATION);
        let scoped_token = confirm_token_from(&missing);

        let wrong_scope = remove_profile_at(
            OfficeRemoveProfileArgs {
                name: "office".to_string(),
                delete_disk: false,
                confirm_token: Some(scoped_token),
            },
            &removal_paths,
            &client,
        )
        .expect_err("token minted for deleteDisk=true must not authorize deleteDisk=false");
        assert_eq!(
            wrong_scope
                .fields()
                .details
                .as_ref()
                .and_then(|details| details.get("reason"))
                .and_then(serde_json::Value::as_str),
            Some("invalid_or_used")
        );

        let fresh = remove_profile_at(
            OfficeRemoveProfileArgs {
                name: "office".to_string(),
                delete_disk: true,
                confirm_token: None,
            },
            &removal_paths,
            &client,
        )
        .expect_err("second confirmation should mint a fresh token");
        let token = confirm_token_from(&fresh);
        client.push_setup(winapps_ok(""));

        let response = remove_profile_at(
            OfficeRemoveProfileArgs {
                name: "office".to_string(),
                delete_disk: true,
                confirm_token: Some(token.clone()),
            },
            &removal_paths,
            &client,
        )
        .expect("valid token should remove managed Office assets");

        assert_eq!(response.state, "removed");
        assert_eq!(response.preserved_disk, Some(false));
        assert!(client
            .calls()
            .contains(&"setup:--user --uninstall".to_string()));
        assert!(!removal_paths.profile_data_dir.exists());
        assert!(!removal_paths.winapps_conf_path.exists());
        for launcher in winapps::OFFICE_LAUNCHERS {
            assert!(!root
                .join("applications")
                .join(format!("{launcher}.desktop"))
                .exists());
        }
        let mimeapps = std::fs::read_to_string(&removal_paths.mimeapps_path)
            .expect("mimeapps should remain readable");
        assert!(!mimeapps.contains("excel-o365.desktop"));
        let state = OfficeProvisioningState::load_or_default(&removal_paths.profile_dir, "office")
            .expect("removed state should load");
        assert_eq!(state.summarize_status(), OfficeProfileStatus::Removed);

        let reused = remove_profile_at(
            OfficeRemoveProfileArgs {
                name: "office".to_string(),
                delete_disk: true,
                confirm_token: Some(token),
            },
            &removal_paths,
            &client,
        )
        .expect_err("confirmToken must be single-use");
        assert_eq!(reused.code(), OfficeError::REMOVE_REQUIRES_CONFIRMATION);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn remove_preserves_unmanaged_adoption_assets() {
        let root = temp_dir("remove-adopted");
        let removal_paths = removal_paths_under(&root);
        seed_unmanaged_adoption_state("office", &removal_paths);
        let client = MockWinAppsClient::new();
        let token = confirm_token_from(
            &remove_profile_at(
                OfficeRemoveProfileArgs {
                    name: "office".to_string(),
                    delete_disk: true,
                    confirm_token: None,
                },
                &removal_paths,
                &client,
            )
            .expect_err("confirmation should be required"),
        );

        let response = remove_profile_at(
            OfficeRemoveProfileArgs {
                name: "office".to_string(),
                delete_disk: true,
                confirm_token: Some(token),
            },
            &removal_paths,
            &client,
        )
        .expect("adopted profile removal should preserve unmanaged assets");

        assert_eq!(response.state, "removed");
        assert_eq!(response.preserved_disk, Some(true));
        assert!(client.calls().is_empty());
        assert!(removal_paths.profile_data_dir.exists());
        assert!(removal_paths.winapps_conf_path.exists());
        assert!(root
            .join("applications")
            .join("excel-o365.desktop")
            .exists());
        let mimeapps = std::fs::read_to_string(&removal_paths.mimeapps_path)
            .expect("mimeapps should remain readable");
        assert!(mimeapps.contains("excel-o365.desktop"));
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

        assert!(message.contains(OfficeError::OFFICE_ODT_STAGE_FAILED));
        assert_eq!(
            state.phase_status(OfficePhase::OfficeStageOdt),
            Some(PhaseStatus::Failed)
        );
        let last_error = state.last_error.expect("last_error should be stored");
        assert_eq!(last_error.code, OfficeError::OFFICE_ODT_STAGE_FAILED);
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

        assert!(message.contains(OfficeError::GUEST_PHASE_TIMEOUT));
        assert_eq!(
            state.phase_status(OfficePhase::OfficeInstall),
            Some(PhaseStatus::Failed)
        );
        let last_error = state.last_error.expect("last_error should be stored");
        assert_eq!(last_error.code, OfficeError::GUEST_PHASE_TIMEOUT);
        assert_eq!(last_error.phase, OfficePhase::OfficeInstall);
        let _ = std::fs::remove_dir_all(root);
    }

    fn rdp_ok(stdout: &str) -> winapps::RdpSessionCommandOutput {
        winapps::RdpSessionCommandOutput {
            success: true,
            status_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
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

    #[derive(Default)]
    struct MockLaunchHost {
        notifications: RefCell<Vec<String>>,
        spawns: RefCell<Vec<String>>,
    }

    impl MockLaunchHost {
        fn notifications(&self) -> Vec<String> {
            self.notifications.borrow().clone()
        }

        fn spawns(&self) -> Vec<String> {
            self.spawns.borrow().clone()
        }
    }

    impl OfficeLaunchHost for MockLaunchHost {
        fn notify(&self, title: &str, message: &str) {
            self.notifications
                .borrow_mut()
                .push(format!("{title}: {message}"));
        }

        fn spawn_progress_window(&self, profile: &str, app_id: &str) -> Result<()> {
            self.spawns
                .borrow_mut()
                .push(format!("--window=office-progress {profile} {app_id}"));
            Ok(())
        }
    }

    fn seed_launch_ready_state(profile: &str, profile_dir: &Path) {
        let mut state = OfficeProvisioningState::new(profile);
        for phase in [
            OfficePhase::RemoteappPrepare,
            OfficePhase::OfficeInstall,
            OfficePhase::WinappsConfig,
            OfficePhase::FinalVerify,
        ] {
            state.mark_phase_running(phase).expect("phase should run");
            let evidence = match phase {
                OfficePhase::OfficeInstall => Some(PhaseEvidence {
                    files: winapps::OFFICE_DESKTOP_APPS
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
                OfficePhase::WinappsConfig | OfficePhase::FinalVerify => Some(PhaseEvidence {
                    launcher_ids: winapps::OFFICE_LAUNCHERS
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
            .save_to_dir(profile_dir)
            .expect("launch-ready state should save");
    }

    fn home_file(name: &str) -> String {
        paths::home().join(name).display().to_string()
    }

    fn winapps_command_ok(stdout: &str) -> winapps::WinAppsCommandOutput {
        winapps::WinAppsCommandOutput {
            success: true,
            status_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn winapps_command_fail(code: i32, stderr: &str) -> winapps::WinAppsCommandOutput {
        winapps::WinAppsCommandOutput {
            success: false,
            status_code: Some(code),
            stdout: String::new(),
            stderr: stderr.to_string(),
        }
    }

    fn removal_paths_under(root: &Path) -> OfficeRemovalPaths {
        OfficeRemovalPaths {
            profile_dir: root.join("profile-cfg"),
            profile_data_dir: root.join("profile-data"),
            winapps_source_dir: root.join("winapps-src"),
            winapps_conf_path: root.join("config").join("winapps.conf"),
            mimeapps_path: root.join("config").join("mimeapps.list"),
        }
    }

    fn seed_managed_removal_state(profile: &str, paths: &OfficeRemovalPaths) {
        seed_removal_files(paths);
        let applications_dir = paths
            .winapps_source_dir
            .parent()
            .expect("root should exist")
            .join("applications");
        let desktop_files = winapps::OFFICE_LAUNCHERS
            .iter()
            .map(|launcher| {
                applications_dir
                    .join(format!("{launcher}.desktop"))
                    .display()
                    .to_string()
            })
            .collect::<Vec<_>>();
        let mut state = OfficeProvisioningState::new(profile);
        state.managed_paths.winapps_conf_owned = true;
        state.managed_paths.desktop_files = desktop_files;
        state.managed_paths.mime_types = winapps::OFFICE_MIME_ASSOCIATIONS
            .iter()
            .map(|association| association.mime_type.to_string())
            .collect();
        mark_ready_winapps_phase(&mut state);
        state
            .save_to_dir(&paths.profile_dir)
            .expect("managed state should save");
    }

    fn seed_unmanaged_adoption_state(profile: &str, paths: &OfficeRemovalPaths) {
        seed_managed_removal_state(profile, paths);
        let mut state = OfficeProvisioningState::load_or_default(&paths.profile_dir, profile)
            .expect("state should load");
        state.status = OfficeProfileStatus::Adopted;
        state.adoption = Some(AdoptionState {
            found: vec![serde_json::json!({
                "id": "manual-existing",
                "kind": "profile",
                "status": "compatible",
                "evidence": "ativos existentes do usuário",
                "managedByDefault": false,
                "managedScope": manual_user_assets_scope(),
            })],
            user_confirmed_at: Some("2026-07-08T00:00:00Z".to_string()),
        });
        state
            .save_to_dir(&paths.profile_dir)
            .expect("adopted state should save");
    }

    fn seed_removal_files(paths: &OfficeRemovalPaths) {
        std::fs::create_dir_all(&paths.profile_data_dir).expect("profile data should exist");
        std::fs::write(paths.profile_data_dir.join("boot.qcow2"), "disk")
            .expect("disk marker should be written");
        std::fs::create_dir_all(&paths.winapps_source_dir).expect("winapps source should exist");
        std::fs::create_dir_all(
            paths
                .winapps_conf_path
                .parent()
                .expect("config parent should exist"),
        )
        .expect("config dir should exist");
        std::fs::write(&paths.winapps_conf_path, "RDP_USER='winbox'\n")
            .expect("winapps conf should be written");
        std::fs::write(&paths.mimeapps_path, winapps::render_mimeapps_list(""))
            .expect("mimeapps should be written");
        let applications_dir = paths
            .winapps_source_dir
            .parent()
            .expect("root should exist")
            .join("applications");
        std::fs::create_dir_all(&applications_dir).expect("applications dir should exist");
        for launcher in winapps::OFFICE_LAUNCHERS {
            std::fs::write(
                applications_dir.join(format!("{launcher}.desktop")),
                format!("[Desktop Entry]\nName={launcher}\nExec=winapps {launcher}\n"),
            )
            .expect("desktop should be written");
        }
    }

    fn mark_ready_winapps_phase(state: &mut OfficeProvisioningState) {
        state
            .mark_phase_running(OfficePhase::WinappsConfig)
            .expect("winapps should run");
        state
            .mark_phase_done(
                OfficePhase::WinappsConfig,
                Some(PhaseEvidence {
                    launcher_ids: winapps::OFFICE_LAUNCHERS
                        .iter()
                        .map(|launcher| launcher.to_string())
                        .collect(),
                    ..PhaseEvidence::default()
                }),
            )
            .expect("winapps should be done");
    }

    fn winapps_ok(stdout: &str) -> winapps::WinAppsCommandOutput {
        winapps::WinAppsCommandOutput {
            success: true,
            status_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
        }
    }

    fn confirm_token_from(error: &OfficeError) -> String {
        error
            .fields()
            .details
            .as_ref()
            .and_then(|details| details.get("confirmToken"))
            .and_then(serde_json::Value::as_str)
            .expect("error should carry confirmToken")
            .to_string()
    }

    fn manual_user_assets_scope() -> ManagedScope {
        ManagedScope {
            manage_profile_config: false,
            manage_winapps_conf: false,
            manage_desktop_entries: false,
            manage_file_associations: false,
            manage_disk_lifecycle: false,
            preserve_existing_winapps_clone: true,
        }
    }
}

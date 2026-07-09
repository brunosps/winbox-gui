use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    paths,
    winapps::{self, CliWinAppsClient, WinAppsClient},
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
    let state = load_office_state_for_contract(&args.name)?;
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
    if !matches!(args.app_id.as_str(), "excel" | "word" | "powerpoint") {
        return Err(office_error(
            OfficeError::APP_NOT_REGISTERED,
            OfficePhase::FirstLaunch,
            false,
            serde_json::json!({ "appId": args.app_id }),
        ));
    }
    let accepted = validate_launch_files(&args.files)?;
    let state = load_office_state_for_contract(&args.name).ok();
    Ok(OfficeLaunchAppResponse {
        app_id: args.app_id,
        delegated_to: "winapps".to_string(),
        files_accepted: accepted,
        active_sessions: state.and_then(|state| state.active_sessions),
    })
}

pub fn remove_profile_contract(
    args: OfficeRemoveProfileArgs,
) -> std::result::Result<OfficeRemoveProfileResponse, OfficeError> {
    if args
        .confirm_token
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        return Err(office_error(
            OfficeError::REMOVE_REQUIRES_CONFIRMATION,
            OfficePhase::FirstLaunch,
            false,
            serde_json::json!({
                "confirmToken": mint_remove_confirm_token(&args.name, args.delete_disk),
                "deleteDisk": args.delete_disk,
            }),
        ));
    }
    let profile_dir = paths::profile_cfg_dir(&args.name);
    let mut state =
        OfficeProvisioningState::load_or_default(&profile_dir, &args.name).map_err(|err| {
            office_error(
                OfficeError::PROFILE_STATE_CONFLICT,
                OfficePhase::FirstLaunch,
                true,
                serde_json::json!({ "detail": format!("{err:#}") }),
            )
        })?;
    state.status = OfficeProfileStatus::Removed;
    state.save_to_dir(&profile_dir).map_err(|err| {
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
        preserved_disk: Some(!args.delete_disk),
        removed_paths: Vec::new(),
    })
}

pub fn telemetry_set_opt_in_contract(
    args: OfficeTelemetrySetOptInArgs,
) -> OfficeTelemetryOptInResponse {
    OfficeTelemetryOptInResponse {
        enabled: args.enabled,
    }
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

fn load_office_state_for_contract(
    profile: &str,
) -> std::result::Result<OfficeProvisioningState, OfficeError> {
    let profile_dir = paths::profile_cfg_dir(profile);
    load_office_state_at(&profile_dir, profile)
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

fn mint_remove_confirm_token(profile: &str, delete_disk: bool) -> String {
    format!(
        "office-remove:{}:{}:{}",
        profile,
        delete_disk,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
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

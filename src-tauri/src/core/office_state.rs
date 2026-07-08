use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const OFFICE_STATE_FILE: &str = "office-provisioning.json";
pub const OFFICE_STATE_SCHEMA_VERSION: &str = "1.0";
pub const OFFICE_BYOL_TEXT_VERSION: &str = "2026-07-08";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficePhase {
    Preflight,
    ByolAcceptance,
    ProfileConfig,
    WindowsPrepare,
    WindowsInstall,
    RemoteappPrepare,
    OfficeStageOdt,
    OfficeInstall,
    WinappsConfig,
    DesktopRegistration,
    FileAssociation,
    FinalVerify,
    FirstLaunch,
}

impl OfficePhase {
    pub const ALL: [OfficePhase; 13] = [
        OfficePhase::Preflight,
        OfficePhase::ByolAcceptance,
        OfficePhase::ProfileConfig,
        OfficePhase::WindowsPrepare,
        OfficePhase::WindowsInstall,
        OfficePhase::RemoteappPrepare,
        OfficePhase::OfficeStageOdt,
        OfficePhase::OfficeInstall,
        OfficePhase::WinappsConfig,
        OfficePhase::DesktopRegistration,
        OfficePhase::FileAssociation,
        OfficePhase::FinalVerify,
        OfficePhase::FirstLaunch,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            OfficePhase::Preflight => "preflight",
            OfficePhase::ByolAcceptance => "byol_acceptance",
            OfficePhase::ProfileConfig => "profile_config",
            OfficePhase::WindowsPrepare => "windows_prepare",
            OfficePhase::WindowsInstall => "windows_install",
            OfficePhase::RemoteappPrepare => "remoteapp_prepare",
            OfficePhase::OfficeStageOdt => "office_stage_odt",
            OfficePhase::OfficeInstall => "office_install",
            OfficePhase::WinappsConfig => "winapps_config",
            OfficePhase::DesktopRegistration => "desktop_registration",
            OfficePhase::FileAssociation => "file_association",
            OfficePhase::FinalVerify => "final_verify",
            OfficePhase::FirstLaunch => "first_launch",
        }
    }

    pub fn is_retry_safe(self) -> bool {
        matches!(
            self,
            OfficePhase::Preflight
                | OfficePhase::RemoteappPrepare
                | OfficePhase::OfficeStageOdt
                | OfficePhase::OfficeInstall
                | OfficePhase::WinappsConfig
                | OfficePhase::DesktopRegistration
                | OfficePhase::FileAssociation
                | OfficePhase::FinalVerify
                | OfficePhase::FirstLaunch
        )
    }
}

impl std::fmt::Display for OfficePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficeProfileStatus {
    Draft,
    Running,
    Blocked,
    Ready,
    Failed,
    Adopted,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Running,
    Done,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByolState {
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    pub text_version: String,
}

impl Default for ByolState {
    fn default() -> Self {
        Self {
            accepted: false,
            accepted_at: None,
            text_version: OFFICE_BYOL_TEXT_VERSION.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeOptions {
    pub product_id: String,
    pub language: String,
    pub office_channel: String,
    pub windows_version: String,
    pub winapps_commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ManagedPaths {
    pub desktop_files: Vec<String>,
    pub mime_types: Vec<String>,
    pub winapps_conf_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionState {
    #[serde(default)]
    pub found: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_confirmed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeLastError {
    pub code: String,
    pub message: String,
    pub phase: OfficePhase,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PhaseEvidence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launcher_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseHistoryEntry {
    pub status: PhaseStatus,
    pub attempt: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<PhaseEvidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseState {
    pub status: PhaseStatus,
    pub attempt: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<PhaseEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<PhaseHistoryEntry>,
}

impl PhaseState {
    pub fn pending() -> Self {
        Self {
            status: PhaseStatus::Pending,
            attempt: 0,
            started_at: None,
            finished_at: None,
            evidence: None,
            history: Vec::new(),
        }
    }

    fn history_entry(&self) -> PhaseHistoryEntry {
        PhaseHistoryEntry {
            status: self.status,
            attempt: self.attempt,
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            evidence: self.evidence.clone(),
        }
    }

    fn reset_for_retry(&mut self) {
        if self.status != PhaseStatus::Pending
            || self.started_at.is_some()
            || self.finished_at.is_some()
            || self.evidence.is_some()
        {
            self.history.push(self.history_entry());
        }
        self.status = PhaseStatus::Pending;
        self.started_at = None;
        self.finished_at = None;
        self.evidence = None;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeProvisioningState {
    pub schema_version: String,
    pub profile: String,
    pub profile_kind: String,
    pub status: OfficeProfileStatus,
    pub created_at: String,
    pub updated_at: String,
    pub byol: ByolState,
    pub options: OfficeOptions,
    pub phases: BTreeMap<OfficePhase, PhaseState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<OfficeLastError>,
    pub managed_paths: ManagedPaths,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adoption: Option<AdoptionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_sessions: Option<bool>,
}

impl OfficeProvisioningState {
    pub fn new(profile: impl Into<String>) -> Self {
        let now = now_rfc3339();
        Self {
            schema_version: OFFICE_STATE_SCHEMA_VERSION.to_string(),
            profile: profile.into(),
            profile_kind: "office".to_string(),
            status: OfficeProfileStatus::Draft,
            created_at: now.clone(),
            updated_at: now,
            byol: ByolState::default(),
            options: OfficeOptions::default(),
            phases: initial_phases(),
            last_error: None,
            managed_paths: ManagedPaths::default(),
            adoption: None,
            active_sessions: None,
        }
    }

    pub fn load_or_default(profile_dir: &Path, profile: &str) -> Result<Self> {
        let path = state_path(profile_dir);
        if !path.exists() {
            return Ok(Self::new(profile));
        }
        let json = std::fs::read_to_string(&path)
            .with_context(|| format!("lendo estado Office em {}", path.display()))?;
        let mut state: Self = serde_json::from_str(&json)
            .with_context(|| format!("estado Office inválido em {}", path.display()))?;
        state.ensure_supported_schema()?;
        state.ensure_all_phases();
        Ok(state)
    }

    pub fn save_to_dir(&mut self, profile_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(profile_dir)
            .with_context(|| format!("criando diretório {}", profile_dir.display()))?;
        self.ensure_supported_schema()?;
        self.ensure_all_phases();
        self.updated_at = now_rfc3339();
        let path = state_path(profile_dir);
        let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
        let json = serde_json::to_string_pretty(self).context("serializando estado Office")?;
        std::fs::write(&tmp, json).with_context(|| format!("escrevendo {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("movendo {} para {}", tmp.display(), path.display()))?;
        Ok(())
    }

    pub fn summarize_status(&self) -> OfficeProfileStatus {
        if self.status == OfficeProfileStatus::Removed {
            return OfficeProfileStatus::Removed;
        }
        if self.last_error.is_some()
            || self
                .phases
                .values()
                .any(|phase| phase.status == PhaseStatus::Failed)
        {
            return OfficeProfileStatus::Failed;
        }
        if self.phase_status(OfficePhase::FinalVerify) == Some(PhaseStatus::Done) {
            return OfficeProfileStatus::Ready;
        }
        if self
            .phases
            .values()
            .any(|phase| phase.status == PhaseStatus::Running)
        {
            return OfficeProfileStatus::Running;
        }
        self.status
    }

    pub fn retry_from_phase(&mut self, phase: OfficePhase) -> Result<Vec<OfficePhase>> {
        if !phase.is_retry_safe() {
            bail!("A fase '{}' não é segura para retry automático.", phase);
        }
        self.ensure_all_phases();
        let mut affected = vec![phase];
        affected.extend(descendants_of(phase));
        for affected_phase in &affected {
            self.phases
                .entry(*affected_phase)
                .or_insert_with(PhaseState::pending)
                .reset_for_retry();
        }
        self.last_error = None;
        self.status = OfficeProfileStatus::Running;
        self.updated_at = now_rfc3339();
        Ok(affected)
    }

    pub fn mark_phase_running(&mut self, phase: OfficePhase) -> Result<()> {
        let now = now_rfc3339();
        let state = self.phase_mut(phase);
        if state.status == PhaseStatus::Done {
            bail!(
                "A fase '{}' já foi concluída; use retry explícito quando for seguro.",
                phase
            );
        }
        if state.status != PhaseStatus::Running {
            state.attempt = state.attempt.saturating_add(1);
            state.started_at = Some(now.clone());
            state.finished_at = None;
            state.evidence = None;
        }
        state.status = PhaseStatus::Running;
        self.status = OfficeProfileStatus::Running;
        self.updated_at = now;
        Ok(())
    }

    pub fn mark_phase_done(
        &mut self,
        phase: OfficePhase,
        evidence: Option<PhaseEvidence>,
    ) -> Result<()> {
        self.complete_phase(phase, PhaseStatus::Done, evidence)
    }

    pub fn mark_phase_skipped(
        &mut self,
        phase: OfficePhase,
        evidence: Option<PhaseEvidence>,
    ) -> Result<()> {
        self.complete_phase(phase, PhaseStatus::Skipped, evidence)
    }

    pub fn mark_phase_failed(&mut self, error: OfficeLastError) {
        let now = now_rfc3339();
        let phase = error.phase;
        let state = self.phase_mut(phase);
        state.status = PhaseStatus::Failed;
        state.finished_at = Some(now.clone());
        self.last_error = Some(error);
        self.status = OfficeProfileStatus::Failed;
        self.updated_at = now;
    }

    pub fn phase_status(&self, phase: OfficePhase) -> Option<PhaseStatus> {
        self.phases.get(&phase).map(|state| state.status)
    }

    pub fn ensure_remoteapp_ready_for_guest_phase(&self, phase: OfficePhase) -> Result<()> {
        if !phase_requires_remoteapp(phase) {
            return Ok(());
        }
        if self.phase_status(OfficePhase::RemoteappPrepare) == Some(PhaseStatus::Done) {
            return Ok(());
        }
        bail!(
            "A fase '{}' exige remoteapp_prepare concluída antes de usar o executor guest.",
            phase
        )
    }

    fn complete_phase(
        &mut self,
        phase: OfficePhase,
        status: PhaseStatus,
        evidence: Option<PhaseEvidence>,
    ) -> Result<()> {
        let now = now_rfc3339();
        let state = self.phase_mut(phase);
        if state.status == status && state.evidence == evidence {
            return Ok(());
        }
        state.status = status;
        state.finished_at = Some(now.clone());
        if evidence.is_some() {
            state.evidence = evidence;
        }
        self.last_error = None;
        self.status = self.summarize_status();
        self.updated_at = now;
        Ok(())
    }

    fn phase_mut(&mut self, phase: OfficePhase) -> &mut PhaseState {
        self.phases.entry(phase).or_insert_with(PhaseState::pending)
    }

    fn ensure_supported_schema(&self) -> Result<()> {
        if self.schema_version != OFFICE_STATE_SCHEMA_VERSION {
            bail!(
                "Schema de estado Office '{}' não suportado; esperado '{}'.",
                self.schema_version,
                OFFICE_STATE_SCHEMA_VERSION
            );
        }
        Ok(())
    }

    fn ensure_all_phases(&mut self) {
        for phase in OfficePhase::ALL {
            self.phases.entry(phase).or_insert_with(PhaseState::pending);
        }
    }
}

pub fn state_path(profile_dir: &Path) -> PathBuf {
    profile_dir.join(OFFICE_STATE_FILE)
}

pub fn canonical_phases() -> &'static [OfficePhase; 13] {
    &OfficePhase::ALL
}

pub fn direct_dependencies() -> &'static [(OfficePhase, OfficePhase)] {
    &[
        (OfficePhase::Preflight, OfficePhase::ByolAcceptance),
        (OfficePhase::ByolAcceptance, OfficePhase::ProfileConfig),
        (OfficePhase::ProfileConfig, OfficePhase::WindowsPrepare),
        (OfficePhase::WindowsPrepare, OfficePhase::WindowsInstall),
        (OfficePhase::WindowsInstall, OfficePhase::RemoteappPrepare),
        (OfficePhase::RemoteappPrepare, OfficePhase::OfficeStageOdt),
        (OfficePhase::OfficeStageOdt, OfficePhase::OfficeInstall),
        (OfficePhase::OfficeInstall, OfficePhase::WinappsConfig),
        (OfficePhase::WinappsConfig, OfficePhase::DesktopRegistration),
        (
            OfficePhase::DesktopRegistration,
            OfficePhase::FileAssociation,
        ),
        (OfficePhase::FileAssociation, OfficePhase::FinalVerify),
        (OfficePhase::FinalVerify, OfficePhase::FirstLaunch),
    ]
}

pub fn descendants_of(phase: OfficePhase) -> Vec<OfficePhase> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    collect_descendants(phase, &mut seen, &mut out);
    out
}

pub fn phase_requires_remoteapp(phase: OfficePhase) -> bool {
    matches!(
        phase,
        OfficePhase::OfficeStageOdt
            | OfficePhase::OfficeInstall
            | OfficePhase::WinappsConfig
            | OfficePhase::FinalVerify
    )
}

fn collect_descendants(
    phase: OfficePhase,
    seen: &mut BTreeSet<OfficePhase>,
    out: &mut Vec<OfficePhase>,
) {
    for (parent, child) in direct_dependencies() {
        if *parent == phase && seen.insert(*child) {
            out.push(*child);
            collect_descendants(*child, seen, out);
        }
    }
}

fn initial_phases() -> BTreeMap<OfficePhase, PhaseState> {
    OfficePhase::ALL
        .into_iter()
        .map(|phase| (phase, PhaseState::pending()))
        .collect()
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn retry_phase_invalidates_descendants() {
        let mut state = OfficeProvisioningState::new("office");
        for phase in OfficePhase::ALL {
            state
                .mark_phase_running(phase)
                .expect("phase should enter running");
            state
                .mark_phase_done(phase, Some(sample_evidence(phase)))
                .expect("phase should be done");
        }

        let affected = state
            .retry_from_phase(OfficePhase::WinappsConfig)
            .expect("winapps_config is retry-safe");

        assert_eq!(
            affected,
            vec![
                OfficePhase::WinappsConfig,
                OfficePhase::DesktopRegistration,
                OfficePhase::FileAssociation,
                OfficePhase::FinalVerify,
                OfficePhase::FirstLaunch,
            ]
        );
        assert_eq!(
            state.phase_status(OfficePhase::OfficeInstall),
            Some(PhaseStatus::Done)
        );
        for phase in affected {
            let phase_state = state.phases.get(&phase).expect("phase exists");
            assert_eq!(phase_state.status, PhaseStatus::Pending);
            assert!(phase_state.evidence.is_none());
            assert_eq!(phase_state.history.len(), 1);
        }
    }

    #[test]
    fn retry_refused_for_unsafe_phase() {
        let mut state = OfficeProvisioningState::new("office");
        state
            .mark_phase_running(OfficePhase::WindowsInstall)
            .expect("phase should run");
        state
            .mark_phase_done(OfficePhase::WindowsInstall, None)
            .expect("phase should finish");

        let err = state
            .retry_from_phase(OfficePhase::WindowsInstall)
            .expect_err("windows_install is not retry-safe");

        assert!(format!("{err:#}").contains("não é segura"));
        assert_eq!(
            state.phase_status(OfficePhase::WindowsInstall),
            Some(PhaseStatus::Done)
        );
    }

    #[test]
    fn ready_ignores_first_launch_phase() {
        let mut state = OfficeProvisioningState::new("office");
        state
            .mark_phase_running(OfficePhase::FinalVerify)
            .expect("phase should run");
        state
            .mark_phase_done(OfficePhase::FinalVerify, None)
            .expect("final verify should finish");

        assert_eq!(
            state.phase_status(OfficePhase::FirstLaunch),
            Some(PhaseStatus::Pending)
        );
        assert_eq!(state.summarize_status(), OfficeProfileStatus::Ready);
    }

    #[test]
    fn office_state_roundtrip_preserves_schema_version() {
        let dir = temp_profile_dir("roundtrip");
        let mut state = OfficeProvisioningState::new("office");
        state.options.product_id = "O365ProPlusRetail".to_string();
        state
            .mark_phase_running(OfficePhase::Preflight)
            .expect("phase should run");
        state
            .mark_phase_done(
                OfficePhase::Preflight,
                Some(sample_evidence(OfficePhase::Preflight)),
            )
            .expect("phase should finish");

        state.save_to_dir(&dir).expect("state should save");
        let loaded =
            OfficeProvisioningState::load_or_default(&dir, "office").expect("state should load");

        assert_eq!(loaded.schema_version, OFFICE_STATE_SCHEMA_VERSION);
        assert_eq!(loaded.profile, "office");
        assert_eq!(loaded.options.product_id, "O365ProPlusRetail");
        assert_eq!(
            loaded.phase_status(OfficePhase::Preflight),
            Some(PhaseStatus::Done)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_state_file_loads_default() {
        let dir = temp_profile_dir("missing");

        let state = OfficeProvisioningState::load_or_default(&dir, "office")
            .expect("missing state should return default");

        assert_eq!(state.schema_version, OFFICE_STATE_SCHEMA_VERSION);
        assert_eq!(state.profile, "office");
        assert_eq!(state.profile_kind, "office");
        assert_eq!(state.phases.len(), OfficePhase::ALL.len());
        assert_eq!(
            state.phase_status(OfficePhase::Preflight),
            Some(PhaseStatus::Pending)
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn mark_phase_running_refuses_done_without_retry() {
        let mut state = OfficeProvisioningState::new("office");
        state
            .mark_phase_running(OfficePhase::OfficeInstall)
            .expect("phase should run");
        state
            .mark_phase_done(OfficePhase::OfficeInstall, None)
            .expect("phase should finish");

        let err = state
            .mark_phase_running(OfficePhase::OfficeInstall)
            .expect_err("done phase needs explicit retry");

        assert!(format!("{err:#}").contains("use retry explícito"));
    }

    #[test]
    fn remoteapp_prepare_precedes_guest_executor_phases() {
        let mut state = OfficeProvisioningState::new("office");

        let err = state
            .ensure_remoteapp_ready_for_guest_phase(OfficePhase::OfficeInstall)
            .expect_err("guest executor phase should require RemoteApp first");

        assert!(format!("{err:#}").contains("remoteapp_prepare"));
        assert!(!phase_requires_remoteapp(OfficePhase::WindowsInstall));
        assert!(phase_requires_remoteapp(OfficePhase::OfficeInstall));
        assert!(direct_dependencies()
            .contains(&(OfficePhase::RemoteappPrepare, OfficePhase::OfficeStageOdt)));

        state
            .mark_phase_running(OfficePhase::RemoteappPrepare)
            .expect("remoteapp_prepare should run");
        state
            .mark_phase_done(OfficePhase::RemoteappPrepare, None)
            .expect("remoteapp_prepare should finish");

        state
            .ensure_remoteapp_ready_for_guest_phase(OfficePhase::OfficeInstall)
            .expect("done remoteapp_prepare should unlock guest executor phases");
    }

    fn sample_evidence(phase: OfficePhase) -> PhaseEvidence {
        PhaseEvidence {
            marker: Some(format!("{}.json", phase.as_str())),
            ..PhaseEvidence::default()
        }
    }

    fn temp_profile_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "winbox-office-state-{name}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }
}

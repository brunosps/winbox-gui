use serde::{Deserialize, Serialize};
use std::fmt;

use super::office_state::OfficePhase;

/// Errors that may bubble out of the VM launch path.
///
/// Variants are serialized as `{ "code": "<snake_case>", ...fields }` so the
/// frontend can match on `code` and render localized hints. The string
/// returned by [`LaunchError::code`] matches the serde tag.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum LaunchError {
    /// `/dev/kvm` absent or unreadable by the current user.
    KvmDenied,
    /// `docker` binary not found on PATH.
    DockerMissing,
    /// Docker CLI present but daemon socket unreachable.
    DockerDaemonDown,
    /// A host port is already bound to another process.
    PortConflict { port: u16 },
    /// `docker pull` failed (network, manifest, or auth).
    ImagePullFailed { image: String, stderr: String },
    /// Container started but exited before becoming ready.
    ContainerCrash { container: String, log_tail: String },
    /// Windows guest never logged the "started successfully" marker
    /// within the configured polling window.
    TimeoutWindows { profile: String },
    /// Linux guest's web-VNC port never accepted a TCP connection
    /// within the configured polling window.
    TimeoutLinux { profile: String, port: u16 },
    /// The WSL2 distribution required by Docker Desktop / the project
    /// is not running. Common on Windows hosts after `wsl --shutdown`
    /// or a Windows boot before any process invoked WSL.
    WslDistroDown { distro: String },
    /// Container was killed by the kernel OOM killer. The user almost
    /// always needs to raise `RAM_SIZE` in the profile.
    ContainerOomKilled {
        container: String,
        mem_limit: String,
    },
    /// A required ISO (Windows installer, Linux distro, etc.) couldn't
    /// be downloaded. Carries the upstream URL and the HTTP status we
    /// observed (0 for connection failure).
    IsoDownloadFailed { url: String, http_status: u16 },
    /// `/dev/kvm` exists but QEMU refuses to use it — typical when the
    /// host kernel exposes KVM but nested virtualization is disabled
    /// at the firmware or hypervisor level.
    KvmNestedNotEnabled,
    /// Docker is installed on the Windows host but the WSL integration
    /// toggle is off for the target distro, so `docker` from the WSL
    /// shell can't reach the daemon.
    DockerDesktopWslIntegrationOff { distro: String },
    /// A volume / image / cache target ran out of space mid-operation.
    DiskFull { path: String, needed_bytes: u64 },
    /// User-supplied custom storage path failed validation (not
    /// absolute, not writable, not enough free space, etc.). `reason`
    /// is a short human-readable explanation in Portuguese.
    StoragePathInvalid { path: String, reason: String },
    /// Wrapped error that doesn't fit a named variant.
    Other { message: String },
}

impl LaunchError {
    /// Stable, machine-readable identifier (matches the serde tag).
    pub fn code(&self) -> &'static str {
        match self {
            LaunchError::KvmDenied => "kvm_denied",
            LaunchError::DockerMissing => "docker_missing",
            LaunchError::DockerDaemonDown => "docker_daemon_down",
            LaunchError::PortConflict { .. } => "port_conflict",
            LaunchError::ImagePullFailed { .. } => "image_pull_failed",
            LaunchError::ContainerCrash { .. } => "container_crash",
            LaunchError::TimeoutWindows { .. } => "timeout_windows",
            LaunchError::TimeoutLinux { .. } => "timeout_linux",
            LaunchError::WslDistroDown { .. } => "wsl_distro_down",
            LaunchError::ContainerOomKilled { .. } => "container_oom_killed",
            LaunchError::IsoDownloadFailed { .. } => "iso_download_failed",
            LaunchError::KvmNestedNotEnabled => "kvm_nested_not_enabled",
            LaunchError::DockerDesktopWslIntegrationOff { .. } => {
                "docker_desktop_wsl_integration_off"
            }
            LaunchError::DiskFull { .. } => "disk_full",
            LaunchError::StoragePathInvalid { .. } => "storage_path_invalid",
            LaunchError::Other { .. } => "other",
        }
    }
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchError::KvmDenied => write!(f, "/dev/kvm não disponível"),
            LaunchError::DockerMissing => write!(f, "docker binário não encontrado no PATH"),
            LaunchError::DockerDaemonDown => write!(f, "daemon docker inacessível"),
            LaunchError::PortConflict { port } => write!(f, "porta {port} já em uso no host"),
            LaunchError::ImagePullFailed { image, stderr } => {
                write!(f, "falha ao baixar imagem '{image}': {stderr}")
            }
            LaunchError::ContainerCrash {
                container,
                log_tail,
            } => write!(f, "container '{container}' caiu durante o boot: {log_tail}"),
            LaunchError::TimeoutWindows { profile } => {
                write!(f, "Windows '{profile}' não terminou de iniciar a tempo")
            }
            LaunchError::TimeoutLinux { profile, port } => write!(
                f,
                "perfil Linux '{profile}' não respondeu em 127.0.0.1:{port}"
            ),
            LaunchError::WslDistroDown { distro } => write!(
                f,
                "distro WSL '{distro}' não está rodando — execute 'wsl -d {distro} -- true' ou reabra o app"
            ),
            LaunchError::ContainerOomKilled { container, mem_limit } => write!(
                f,
                "container '{container}' foi terminado por falta de memória (limite: {mem_limit}); aumente RAM_SIZE no perfil"
            ),
            LaunchError::IsoDownloadFailed { url, http_status } => write!(
                f,
                "download da ISO falhou ({http_status}) em {url}"
            ),
            LaunchError::KvmNestedNotEnabled => write!(
                f,
                "/dev/kvm presente mas aceleração indisponível — habilite virtualização aninhada no .wslconfig ou no firmware"
            ),
            LaunchError::DockerDesktopWslIntegrationOff { distro } => write!(
                f,
                "Docker Desktop encontrado mas integração WSL desabilitada para '{distro}' — ative em Settings → Resources → WSL Integration"
            ),
            LaunchError::DiskFull { path, needed_bytes } => write!(
                f,
                "espaço insuficiente em {path} — faltam ~{} bytes",
                needed_bytes
            ),
            LaunchError::StoragePathInvalid { path, reason } => write!(
                f,
                "Local de armazenamento '{path}' inválido: {reason}"
            ),
            LaunchError::Other { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for LaunchError {}

impl From<anyhow::Error> for LaunchError {
    fn from(e: anyhow::Error) -> Self {
        LaunchError::Other {
            message: format!("{e:#}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficeErrorFields {
    pub phase: OfficePhase,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl OfficeErrorFields {
    pub fn new(phase: OfficePhase, retryable: bool, details: Option<serde_json::Value>) -> Self {
        Self {
            phase,
            retryable,
            details,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum OfficeError {
    ByolNotAccepted {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    PreflightKvmMissing {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    PreflightDockerMissing {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    PreflightSubnetConflict {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    FlatpakFreerdpMissing {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    FlatpakHomeOverrideMissing {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    NativeFreerdpTooOld {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    OfficeProductInvalid {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    ProfileNotOffice {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    ProfileStateConflict {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    GuestRemoteappNotPrepared {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    GuestRdpUnreachable {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    GuestExecutorFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    GuestPhaseTimeout {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    GuestDiskFull {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    OfficeWindowsFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    OfficeOdtStageFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    OfficeOdtFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    OfficeDetectionFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsCloneFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsNoConfig {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsMissingDeps {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsBadPort {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsRdpFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsAppScanFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    WinappsPinMismatch {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    DesktopRegistrationFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    FileAssociationFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    AppNotRegistered {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    AppLaunchFailed {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    FileOutsideHome {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    OfficeAppsMaybeOpen {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
    RemoveRequiresConfirmation {
        #[serde(flatten)]
        fields: OfficeErrorFields,
    },
}

impl OfficeError {
    pub const BYOL_NOT_ACCEPTED: &'static str = "byol_not_accepted";
    pub const PREFLIGHT_KVM_MISSING: &'static str = "preflight_kvm_missing";
    pub const PREFLIGHT_DOCKER_MISSING: &'static str = "preflight_docker_missing";
    pub const PREFLIGHT_SUBNET_CONFLICT: &'static str = "preflight_subnet_conflict";
    pub const FLATPAK_FREERDP_MISSING: &'static str = "flatpak_freerdp_missing";
    pub const FLATPAK_HOME_OVERRIDE_MISSING: &'static str = "flatpak_home_override_missing";
    pub const NATIVE_FREERDP_TOO_OLD: &'static str = "native_freerdp_too_old";
    pub const OFFICE_PRODUCT_INVALID: &'static str = "office_product_invalid";
    pub const PROFILE_NOT_OFFICE: &'static str = "profile_not_office";
    pub const PROFILE_STATE_CONFLICT: &'static str = "profile_state_conflict";
    pub const GUEST_REMOTEAPP_NOT_PREPARED: &'static str = "guest_remoteapp_not_prepared";
    pub const GUEST_RDP_UNREACHABLE: &'static str = "guest_rdp_unreachable";
    pub const GUEST_EXECUTOR_FAILED: &'static str = "guest_executor_failed";
    pub const GUEST_PHASE_TIMEOUT: &'static str = "guest_phase_timeout";
    pub const GUEST_DISK_FULL: &'static str = "guest_disk_full";
    pub const OFFICE_WINDOWS_FAILED: &'static str = "office_windows_failed";
    pub const OFFICE_ODT_STAGE_FAILED: &'static str = "office_odt_stage_failed";
    pub const OFFICE_ODT_FAILED: &'static str = "office_odt_failed";
    pub const OFFICE_DETECTION_FAILED: &'static str = "office_detection_failed";
    pub const WINAPPS_CLONE_FAILED: &'static str = "winapps_clone_failed";
    pub const WINAPPS_NO_CONFIG: &'static str = "winapps_no_config";
    pub const WINAPPS_MISSING_DEPS: &'static str = "winapps_missing_deps";
    pub const WINAPPS_BAD_PORT: &'static str = "winapps_bad_port";
    pub const WINAPPS_RDP_FAILED: &'static str = "winapps_rdp_failed";
    pub const WINAPPS_APP_SCAN_FAILED: &'static str = "winapps_app_scan_failed";
    pub const WINAPPS_PIN_MISMATCH: &'static str = "winapps_pin_mismatch";
    pub const DESKTOP_REGISTRATION_FAILED: &'static str = "desktop_registration_failed";
    pub const FILE_ASSOCIATION_FAILED: &'static str = "file_association_failed";
    pub const APP_NOT_REGISTERED: &'static str = "app_not_registered";
    pub const APP_LAUNCH_FAILED: &'static str = "app_launch_failed";
    pub const FILE_OUTSIDE_HOME: &'static str = "file_outside_home";
    pub const OFFICE_APPS_MAYBE_OPEN: &'static str = "office_apps_maybe_open";
    pub const REMOVE_REQUIRES_CONFIRMATION: &'static str = "remove_requires_confirmation";

    pub fn new(
        code: &str,
        phase: OfficePhase,
        retryable: bool,
        details: Option<serde_json::Value>,
    ) -> Self {
        let fields = OfficeErrorFields::new(phase, retryable, details);
        match code {
            Self::BYOL_NOT_ACCEPTED => Self::ByolNotAccepted { fields },
            Self::PREFLIGHT_KVM_MISSING => Self::PreflightKvmMissing { fields },
            Self::PREFLIGHT_DOCKER_MISSING => Self::PreflightDockerMissing { fields },
            Self::PREFLIGHT_SUBNET_CONFLICT => Self::PreflightSubnetConflict { fields },
            Self::FLATPAK_FREERDP_MISSING => Self::FlatpakFreerdpMissing { fields },
            Self::FLATPAK_HOME_OVERRIDE_MISSING => Self::FlatpakHomeOverrideMissing { fields },
            Self::NATIVE_FREERDP_TOO_OLD => Self::NativeFreerdpTooOld { fields },
            Self::OFFICE_PRODUCT_INVALID => Self::OfficeProductInvalid { fields },
            Self::PROFILE_NOT_OFFICE => Self::ProfileNotOffice { fields },
            Self::PROFILE_STATE_CONFLICT => Self::ProfileStateConflict { fields },
            Self::GUEST_REMOTEAPP_NOT_PREPARED => Self::GuestRemoteappNotPrepared { fields },
            Self::GUEST_RDP_UNREACHABLE => Self::GuestRdpUnreachable { fields },
            Self::GUEST_EXECUTOR_FAILED => Self::GuestExecutorFailed { fields },
            Self::GUEST_PHASE_TIMEOUT => Self::GuestPhaseTimeout { fields },
            Self::GUEST_DISK_FULL => Self::GuestDiskFull { fields },
            Self::OFFICE_WINDOWS_FAILED => Self::OfficeWindowsFailed { fields },
            Self::OFFICE_ODT_STAGE_FAILED => Self::OfficeOdtStageFailed { fields },
            Self::OFFICE_ODT_FAILED => Self::OfficeOdtFailed { fields },
            Self::OFFICE_DETECTION_FAILED => Self::OfficeDetectionFailed { fields },
            Self::WINAPPS_CLONE_FAILED => Self::WinappsCloneFailed { fields },
            Self::WINAPPS_NO_CONFIG => Self::WinappsNoConfig { fields },
            Self::WINAPPS_MISSING_DEPS => Self::WinappsMissingDeps { fields },
            Self::WINAPPS_BAD_PORT => Self::WinappsBadPort { fields },
            Self::WINAPPS_RDP_FAILED => Self::WinappsRdpFailed { fields },
            Self::WINAPPS_APP_SCAN_FAILED => Self::WinappsAppScanFailed { fields },
            Self::WINAPPS_PIN_MISMATCH => Self::WinappsPinMismatch { fields },
            Self::DESKTOP_REGISTRATION_FAILED => Self::DesktopRegistrationFailed { fields },
            Self::FILE_ASSOCIATION_FAILED => Self::FileAssociationFailed { fields },
            Self::APP_NOT_REGISTERED => Self::AppNotRegistered { fields },
            Self::APP_LAUNCH_FAILED => Self::AppLaunchFailed { fields },
            Self::FILE_OUTSIDE_HOME => Self::FileOutsideHome { fields },
            Self::OFFICE_APPS_MAYBE_OPEN => Self::OfficeAppsMaybeOpen { fields },
            Self::REMOVE_REQUIRES_CONFIRMATION => Self::RemoveRequiresConfirmation { fields },
            _ => Self::ProfileStateConflict {
                fields: OfficeErrorFields::new(
                    phase,
                    retryable,
                    Some(serde_json::json!({
                        "unknownCode": code,
                        "details": fields.details,
                    })),
                ),
            },
        }
    }

    pub fn from_launch_error(phase: OfficePhase, err: LaunchError) -> Self {
        let launch_code = err.code();
        let launch_details = serde_json::to_value(&err).unwrap_or_else(|_| {
            serde_json::json!({
                "code": launch_code,
                "message": err.to_string(),
            })
        });
        Self::new(
            Self::OFFICE_WINDOWS_FAILED,
            phase,
            true,
            Some(serde_json::json!({
                "launchCode": launch_code,
                "launchDetails": launch_details,
            })),
        )
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::ByolNotAccepted { .. } => Self::BYOL_NOT_ACCEPTED,
            Self::PreflightKvmMissing { .. } => Self::PREFLIGHT_KVM_MISSING,
            Self::PreflightDockerMissing { .. } => Self::PREFLIGHT_DOCKER_MISSING,
            Self::PreflightSubnetConflict { .. } => Self::PREFLIGHT_SUBNET_CONFLICT,
            Self::FlatpakFreerdpMissing { .. } => Self::FLATPAK_FREERDP_MISSING,
            Self::FlatpakHomeOverrideMissing { .. } => Self::FLATPAK_HOME_OVERRIDE_MISSING,
            Self::NativeFreerdpTooOld { .. } => Self::NATIVE_FREERDP_TOO_OLD,
            Self::OfficeProductInvalid { .. } => Self::OFFICE_PRODUCT_INVALID,
            Self::ProfileNotOffice { .. } => Self::PROFILE_NOT_OFFICE,
            Self::ProfileStateConflict { .. } => Self::PROFILE_STATE_CONFLICT,
            Self::GuestRemoteappNotPrepared { .. } => Self::GUEST_REMOTEAPP_NOT_PREPARED,
            Self::GuestRdpUnreachable { .. } => Self::GUEST_RDP_UNREACHABLE,
            Self::GuestExecutorFailed { .. } => Self::GUEST_EXECUTOR_FAILED,
            Self::GuestPhaseTimeout { .. } => Self::GUEST_PHASE_TIMEOUT,
            Self::GuestDiskFull { .. } => Self::GUEST_DISK_FULL,
            Self::OfficeWindowsFailed { .. } => Self::OFFICE_WINDOWS_FAILED,
            Self::OfficeOdtStageFailed { .. } => Self::OFFICE_ODT_STAGE_FAILED,
            Self::OfficeOdtFailed { .. } => Self::OFFICE_ODT_FAILED,
            Self::OfficeDetectionFailed { .. } => Self::OFFICE_DETECTION_FAILED,
            Self::WinappsCloneFailed { .. } => Self::WINAPPS_CLONE_FAILED,
            Self::WinappsNoConfig { .. } => Self::WINAPPS_NO_CONFIG,
            Self::WinappsMissingDeps { .. } => Self::WINAPPS_MISSING_DEPS,
            Self::WinappsBadPort { .. } => Self::WINAPPS_BAD_PORT,
            Self::WinappsRdpFailed { .. } => Self::WINAPPS_RDP_FAILED,
            Self::WinappsAppScanFailed { .. } => Self::WINAPPS_APP_SCAN_FAILED,
            Self::WinappsPinMismatch { .. } => Self::WINAPPS_PIN_MISMATCH,
            Self::DesktopRegistrationFailed { .. } => Self::DESKTOP_REGISTRATION_FAILED,
            Self::FileAssociationFailed { .. } => Self::FILE_ASSOCIATION_FAILED,
            Self::AppNotRegistered { .. } => Self::APP_NOT_REGISTERED,
            Self::AppLaunchFailed { .. } => Self::APP_LAUNCH_FAILED,
            Self::FileOutsideHome { .. } => Self::FILE_OUTSIDE_HOME,
            Self::OfficeAppsMaybeOpen { .. } => Self::OFFICE_APPS_MAYBE_OPEN,
            Self::RemoveRequiresConfirmation { .. } => Self::REMOVE_REQUIRES_CONFIRMATION,
        }
    }

    pub fn fields(&self) -> &OfficeErrorFields {
        match self {
            Self::ByolNotAccepted { fields }
            | Self::PreflightKvmMissing { fields }
            | Self::PreflightDockerMissing { fields }
            | Self::PreflightSubnetConflict { fields }
            | Self::FlatpakFreerdpMissing { fields }
            | Self::FlatpakHomeOverrideMissing { fields }
            | Self::NativeFreerdpTooOld { fields }
            | Self::OfficeProductInvalid { fields }
            | Self::ProfileNotOffice { fields }
            | Self::ProfileStateConflict { fields }
            | Self::GuestRemoteappNotPrepared { fields }
            | Self::GuestRdpUnreachable { fields }
            | Self::GuestExecutorFailed { fields }
            | Self::GuestPhaseTimeout { fields }
            | Self::GuestDiskFull { fields }
            | Self::OfficeWindowsFailed { fields }
            | Self::OfficeOdtStageFailed { fields }
            | Self::OfficeOdtFailed { fields }
            | Self::OfficeDetectionFailed { fields }
            | Self::WinappsCloneFailed { fields }
            | Self::WinappsNoConfig { fields }
            | Self::WinappsMissingDeps { fields }
            | Self::WinappsBadPort { fields }
            | Self::WinappsRdpFailed { fields }
            | Self::WinappsAppScanFailed { fields }
            | Self::WinappsPinMismatch { fields }
            | Self::DesktopRegistrationFailed { fields }
            | Self::FileAssociationFailed { fields }
            | Self::AppNotRegistered { fields }
            | Self::AppLaunchFailed { fields }
            | Self::FileOutsideHome { fields }
            | Self::OfficeAppsMaybeOpen { fields }
            | Self::RemoveRequiresConfirmation { fields } => fields,
        }
    }

    pub fn all_codes() -> &'static [&'static str] {
        &[
            Self::BYOL_NOT_ACCEPTED,
            Self::PREFLIGHT_KVM_MISSING,
            Self::PREFLIGHT_DOCKER_MISSING,
            Self::PREFLIGHT_SUBNET_CONFLICT,
            Self::FLATPAK_FREERDP_MISSING,
            Self::FLATPAK_HOME_OVERRIDE_MISSING,
            Self::NATIVE_FREERDP_TOO_OLD,
            Self::OFFICE_PRODUCT_INVALID,
            Self::PROFILE_NOT_OFFICE,
            Self::PROFILE_STATE_CONFLICT,
            Self::GUEST_REMOTEAPP_NOT_PREPARED,
            Self::GUEST_RDP_UNREACHABLE,
            Self::GUEST_EXECUTOR_FAILED,
            Self::GUEST_PHASE_TIMEOUT,
            Self::GUEST_DISK_FULL,
            Self::OFFICE_WINDOWS_FAILED,
            Self::OFFICE_ODT_STAGE_FAILED,
            Self::OFFICE_ODT_FAILED,
            Self::OFFICE_DETECTION_FAILED,
            Self::WINAPPS_CLONE_FAILED,
            Self::WINAPPS_NO_CONFIG,
            Self::WINAPPS_MISSING_DEPS,
            Self::WINAPPS_BAD_PORT,
            Self::WINAPPS_RDP_FAILED,
            Self::WINAPPS_APP_SCAN_FAILED,
            Self::WINAPPS_PIN_MISMATCH,
            Self::DESKTOP_REGISTRATION_FAILED,
            Self::FILE_ASSOCIATION_FAILED,
            Self::APP_NOT_REGISTERED,
            Self::APP_LAUNCH_FAILED,
            Self::FILE_OUTSIDE_HOME,
            Self::OFFICE_APPS_MAYBE_OPEN,
            Self::REMOVE_REQUIRES_CONFIRMATION,
        ]
    }
}

impl fmt::Display for OfficeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} na fase {}",
            self.code(),
            self.fields().phase.as_str()
        )
    }
}

impl std::error::Error for OfficeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_returns_stable_snake_case_for_every_variant() {
        let cases: &[(LaunchError, &str)] = &[
            (LaunchError::KvmDenied, "kvm_denied"),
            (LaunchError::DockerMissing, "docker_missing"),
            (LaunchError::DockerDaemonDown, "docker_daemon_down"),
            (LaunchError::PortConflict { port: 3389 }, "port_conflict"),
            (
                LaunchError::ImagePullFailed {
                    image: "x".into(),
                    stderr: "".into(),
                },
                "image_pull_failed",
            ),
            (
                LaunchError::ContainerCrash {
                    container: "x".into(),
                    log_tail: "".into(),
                },
                "container_crash",
            ),
            (
                LaunchError::TimeoutWindows {
                    profile: "x".into(),
                },
                "timeout_windows",
            ),
            (
                LaunchError::TimeoutLinux {
                    profile: "x".into(),
                    port: 8006,
                },
                "timeout_linux",
            ),
            (
                LaunchError::WslDistroDown {
                    distro: "Ubuntu-24.04".into(),
                },
                "wsl_distro_down",
            ),
            (
                LaunchError::ContainerOomKilled {
                    container: "winbox-foo".into(),
                    mem_limit: "4G".into(),
                },
                "container_oom_killed",
            ),
            (
                LaunchError::IsoDownloadFailed {
                    url: "https://example.com/x.iso".into(),
                    http_status: 0,
                },
                "iso_download_failed",
            ),
            (LaunchError::KvmNestedNotEnabled, "kvm_nested_not_enabled"),
            (
                LaunchError::DockerDesktopWslIntegrationOff {
                    distro: "Ubuntu".into(),
                },
                "docker_desktop_wsl_integration_off",
            ),
            (
                LaunchError::DiskFull {
                    path: "/var/lib/docker".into(),
                    needed_bytes: 1_073_741_824,
                },
                "disk_full",
            ),
            (
                LaunchError::StoragePathInvalid {
                    path: "/x".into(),
                    reason: "not writable".into(),
                },
                "storage_path_invalid",
            ),
            (LaunchError::Other { message: "".into() }, "other"),
        ];
        for (err, code) in cases {
            assert_eq!(
                err.code(),
                *code,
                "code() vs serde tag mismatch for {err:?}"
            );
        }
    }

    #[test]
    fn serializes_unit_variant_with_only_code() {
        let json = serde_json::to_value(LaunchError::KvmDenied).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "kvm_denied" }));
    }

    #[test]
    fn serializes_struct_variant_with_fields() {
        let json = serde_json::to_value(LaunchError::PortConflict { port: 3389 }).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "port_conflict", "port": 3389 })
        );
    }

    #[test]
    fn serializes_wsl_distro_down_with_field() {
        let json = serde_json::to_value(LaunchError::WslDistroDown {
            distro: "Ubuntu-24.04".into(),
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "code": "wsl_distro_down", "distro": "Ubuntu-24.04" })
        );
    }

    #[test]
    fn serializes_disk_full_with_path_and_bytes() {
        let json = serde_json::to_value(LaunchError::DiskFull {
            path: "/var/lib/docker".into(),
            needed_bytes: 1_073_741_824,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "code": "disk_full",
                "path": "/var/lib/docker",
                "needed_bytes": 1_073_741_824u64
            })
        );
    }

    #[test]
    fn from_anyhow_preserves_full_context_chain() {
        let root = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "boom");
        let chained = anyhow::Error::new(root).context("while launching");
        let err: LaunchError = chained.into();
        let msg = match err {
            LaunchError::Other { message } => message,
            _ => panic!("expected Other variant"),
        };
        assert!(msg.contains("while launching"));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn office_error_serializes_with_code_tag() {
        let err = OfficeError::new(
            OfficeError::GUEST_PHASE_TIMEOUT,
            OfficePhase::OfficeInstall,
            true,
            Some(serde_json::json!({
                "timeoutSeconds": 3600,
                "marker": "office_install.json"
            })),
        );

        let json = serde_json::to_value(&err).expect("OfficeError should serialize");

        assert_eq!(json["code"], "guest_phase_timeout");
        assert_eq!(json["phase"], "office_install");
        assert_eq!(json["retryable"], true);
        assert_eq!(json["details"]["marker"], "office_install.json");
        assert_eq!(err.code(), "guest_phase_timeout");
        assert_eq!(OfficeError::all_codes().len(), 33);
    }

    #[test]
    fn office_error_roundtrip_preserves_details() {
        let original = OfficeError::new(
            OfficeError::FILE_OUTSIDE_HOME,
            OfficePhase::FirstLaunch,
            false,
            Some(serde_json::json!({
                "path": "/tmp/report.xlsx",
                "actionHint": "+home-drive só expõe $HOME ao guest"
            })),
        );
        let json = serde_json::to_string(&original).expect("OfficeError should serialize");
        let decoded: OfficeError =
            serde_json::from_str(&json).expect("OfficeError should deserialize");

        assert_eq!(decoded, original);
        assert_eq!(decoded.code(), OfficeError::FILE_OUTSIDE_HOME);
    }

    #[test]
    fn launch_error_bridge_preserves_original_code() {
        let bridged = OfficeError::from_launch_error(
            OfficePhase::WindowsInstall,
            LaunchError::TimeoutWindows {
                profile: "office".to_string(),
            },
        );
        let json = serde_json::to_value(&bridged).expect("OfficeError should serialize");

        assert_eq!(json["code"], "office_windows_failed");
        assert_eq!(json["phase"], "windows_install");
        assert_eq!(json["details"]["launchCode"], "timeout_windows");
        assert_eq!(json["details"]["launchDetails"]["code"], "timeout_windows");
    }
}

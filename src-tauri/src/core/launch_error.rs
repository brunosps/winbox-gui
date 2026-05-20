use serde::Serialize;
use std::fmt;

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
    ContainerOomKilled { container: String, mem_limit: String },
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
            LaunchError::DockerDesktopWslIntegrationOff { .. } => "docker_desktop_wsl_integration_off",
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
            (
                LaunchError::KvmNestedNotEnabled,
                "kvm_nested_not_enabled",
            ),
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
}

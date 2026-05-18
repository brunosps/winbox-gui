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
    /// `xfreerdp3` binary not found on PATH.
    FreeRdpMissing,
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
            LaunchError::FreeRdpMissing => "freerdp_missing",
            LaunchError::PortConflict { .. } => "port_conflict",
            LaunchError::ImagePullFailed { .. } => "image_pull_failed",
            LaunchError::ContainerCrash { .. } => "container_crash",
            LaunchError::TimeoutWindows { .. } => "timeout_windows",
            LaunchError::TimeoutLinux { .. } => "timeout_linux",
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
            LaunchError::FreeRdpMissing => {
                write!(f, "xfreerdp3 não encontrado — instale o pacote freerdp3")
            }
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
            (LaunchError::FreeRdpMissing, "freerdp_missing"),
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

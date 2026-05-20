//! First-run backend bootstrap detection (Phase 4 of the Windows port).
//!
//! The native Windows app still needs a Linux VM engine underneath: WSL2
//! + a distro running Docker + the dockurr image. This module figures out
//! **what's missing** so the UI can guide the user (or, eventually, run
//! the fix steps). Everything here is read-only — the actual install
//! actions (wsl --install, docker pull) live behind explicit user-driven
//! commands and are NEVER triggered by detection.
//!
//! Parsers are pure (no I/O) so they're unit-tested on any platform; the
//! `check_status` orchestrator shells out to read-only probes.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Ok,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootstrapCheck {
    pub id: String,
    pub label: String,
    pub state: StepState,
    pub detail: String,
    pub fix_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootstrapStatus {
    /// True only when every required piece is present.
    pub ready: bool,
    pub checks: Vec<BootstrapCheck>,
}

/// Inputs gathered by the (platform-specific) probes, kept separate so
/// `evaluate` stays pure and testable.
#[derive(Debug, Clone, Default)]
pub struct Probes {
    /// `true` if `wsl --status` / `wsl --version` indicates WSL2 present.
    pub wsl_installed: bool,
    /// Name of a running distro, if any (from `wsl --list --running`).
    pub running_distro: Option<String>,
    /// `true` if `docker version` (server) responds.
    pub docker_available: bool,
    /// `true` if the dockurr image is already pulled.
    pub image_present: bool,
}

// ── Pure parsers ────────────────────────────────────────────────────────

/// Detect WSL2 from `wsl --status` / `wsl --version` output. Tolerant of
/// the UTF-16 noise these commands emit (caller should decode first).
pub fn parse_wsl_installed(status_output: &str) -> bool {
    let lower = status_output.to_lowercase();
    // `wsl --version` prints "WSL version: 2.x"; `wsl --status` prints
    // "Default Version: 2". Either confirms WSL2 is usable.
    lower.contains("wsl version")
        || lower.contains("versão do wsl")
        || (lower.contains("default version") && lower.contains('2'))
        || (lower.contains("versão padrão") && lower.contains('2'))
}

/// First running distro name from `wsl --list --running --quiet` output
/// (one distro per line). Returns None if the list is empty.
pub fn parse_running_distro(list_output: &str) -> Option<String> {
    list_output
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with("Windows"))
        .map(|s| s.to_string())
}

/// `docker version` succeeded with a server section?
pub fn parse_docker_available(version_output: &str) -> bool {
    let lower = version_output.to_lowercase();
    lower.contains("server:") || lower.contains("server version")
}

/// Is `image` listed in `docker images` output?
pub fn parse_image_present(images_output: &str, image_repo: &str) -> bool {
    images_output
        .lines()
        .any(|l| l.contains(image_repo))
}

/// Decide the overall bootstrap status from gathered probes. Pure.
pub fn evaluate(p: &Probes) -> BootstrapStatus {
    let mut checks = Vec::new();

    checks.push(BootstrapCheck {
        id: "wsl2".into(),
        label: "WSL2".into(),
        state: if p.wsl_installed { StepState::Ok } else { StepState::Missing },
        detail: if p.wsl_installed {
            "WSL2 disponível.".into()
        } else {
            "WSL2 não detectado.".into()
        },
        fix_hint: "Instalar com: wsl --install".into(),
    });

    let distro_ok = p.running_distro.is_some();
    checks.push(BootstrapCheck {
        id: "distro".into(),
        label: "Distro Linux".into(),
        state: if distro_ok { StepState::Ok } else { StepState::Missing },
        detail: match &p.running_distro {
            Some(d) => format!("Distro '{d}' rodando."),
            None => "Nenhuma distro WSL rodando.".into(),
        },
        fix_hint: "Instalar/iniciar a distro winbox (Docker + deps).".into(),
    });

    checks.push(BootstrapCheck {
        id: "docker".into(),
        label: "Docker".into(),
        state: if p.docker_available { StepState::Ok } else { StepState::Missing },
        detail: if p.docker_available {
            "Docker daemon acessível.".into()
        } else {
            "Docker não acessível.".into()
        },
        fix_hint: "Iniciar Docker na distro (sudo service docker start).".into(),
    });

    checks.push(BootstrapCheck {
        id: "image".into(),
        label: "Imagem dockurr".into(),
        state: if p.image_present { StepState::Ok } else { StepState::Missing },
        detail: if p.image_present {
            "Imagem dockurr/windows presente.".into()
        } else {
            "Imagem dockurr/windows ainda não baixada.".into()
        },
        fix_hint: "Baixar com: docker pull dockurr/windows".into(),
    });

    let ready = checks.iter().all(|c| c.state == StepState::Ok);
    BootstrapStatus { ready, checks }
}

// ── Platform probes ─────────────────────────────────────────────────────

/// Gather the read-only probes and evaluate. On Windows this shells out
/// to `wsl`/`docker`; elsewhere we assume a local engine and report ready
/// if `docker` answers (the dev/Linux case).
#[cfg(target_os = "windows")]
pub fn check_status() -> BootstrapStatus {
    use crate::core::health_wsl::decode_wsl_output;
    use std::process::Command;

    let run = |cmd: &str, args: &[&str]| -> String {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .map(|o| {
                let mut s = decode_wsl_output(&o.stdout);
                s.push_str(&decode_wsl_output(&o.stderr));
                s
            })
            .unwrap_or_default()
    };

    let wsl_status = run("wsl.exe", &["--status"]);
    let wsl_running = run("wsl.exe", &["--list", "--running", "--quiet"]);
    let docker_ver = run("docker", &["version"]);
    let docker_imgs = run("docker", &["images", "--format", "{{.Repository}}"]);

    let probes = Probes {
        wsl_installed: parse_wsl_installed(&wsl_status),
        running_distro: parse_running_distro(&wsl_running),
        docker_available: parse_docker_available(&docker_ver),
        image_present: parse_image_present(&docker_imgs, "dockurr/windows"),
    };
    evaluate(&probes)
}

#[cfg(not(target_os = "windows"))]
pub fn check_status() -> BootstrapStatus {
    use std::process::Command;
    let docker_ver = Command::new("docker")
        .arg("version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let docker_imgs = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    // On Linux the app runs alongside the engine; WSL/distro checks are N/A.
    let probes = Probes {
        wsl_installed: true,
        running_distro: Some("(local)".into()),
        docker_available: parse_docker_available(&docker_ver),
        image_present: parse_image_present(&docker_imgs, "dockurr/windows"),
    };
    evaluate(&probes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wsl2_from_status() {
        assert!(parse_wsl_installed("Default Version: 2\nWSL version: 2.6.1.0"));
        assert!(parse_wsl_installed("Versão do WSL: 2.6.1.0"));
        assert!(parse_wsl_installed("Default Version: 2"));
        assert!(!parse_wsl_installed("command not found"));
        assert!(!parse_wsl_installed(""));
    }

    #[test]
    fn running_distro_picks_first_nonblank() {
        assert_eq!(
            parse_running_distro("Ubuntu-24.04\nUbuntu-22.04\n").as_deref(),
            Some("Ubuntu-24.04")
        );
        assert_eq!(parse_running_distro("\n\n").as_deref(), None);
        assert_eq!(parse_running_distro("").as_deref(), None);
    }

    #[test]
    fn docker_available_needs_server() {
        assert!(parse_docker_available("Client: ...\nServer: Docker Engine"));
        assert!(parse_docker_available("Server Version: 29.1.3"));
        assert!(!parse_docker_available("Cannot connect to the Docker daemon"));
    }

    #[test]
    fn image_present_matches_repo() {
        assert!(parse_image_present("dockurr/windows\nubuntu\n", "dockurr/windows"));
        assert!(!parse_image_present("ubuntu\nalpine\n", "dockurr/windows"));
    }

    #[test]
    fn evaluate_all_ok_is_ready() {
        let p = Probes {
            wsl_installed: true,
            running_distro: Some("Ubuntu-24.04".into()),
            docker_available: true,
            image_present: true,
        };
        let s = evaluate(&p);
        assert!(s.ready);
        assert_eq!(s.checks.len(), 4);
        assert!(s.checks.iter().all(|c| c.state == StepState::Ok));
    }

    #[test]
    fn evaluate_missing_pieces_not_ready() {
        let p = Probes {
            wsl_installed: true,
            running_distro: None,
            docker_available: false,
            image_present: false,
        };
        let s = evaluate(&p);
        assert!(!s.ready);
        let missing: Vec<&str> = s
            .checks
            .iter()
            .filter(|c| c.state == StepState::Missing)
            .map(|c| c.id.as_str())
            .collect();
        assert!(missing.contains(&"distro"));
        assert!(missing.contains(&"docker"));
        assert!(missing.contains(&"image"));
    }
}

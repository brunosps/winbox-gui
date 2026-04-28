use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Stdio};

use super::{docker, env_file, gpu, paths, ports, profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    pub message: String,
    pub detail: String,
    pub action_hint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostHealthReport {
    pub overall_status: CheckStatus,
    pub generated_at: String,
    pub checks: Vec<HealthCheck>,
}

pub fn report() -> HostHealthReport {
    let checks = vec![
        check_docker_binary(),
        check_docker_daemon(),
        check_compose(),
        check_kvm(),
        check_storage(),
        check_profiles(),
        check_ports(),
        check_gpu_vfio(),
    ];

    HostHealthReport {
        overall_status: overall_status(&checks),
        generated_at: chrono::Local::now().to_rfc3339(),
        checks,
    }
}

pub fn overall_status(checks: &[HealthCheck]) -> CheckStatus {
    if checks.iter().any(|c| c.status == CheckStatus::Error) {
        CheckStatus::Error
    } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        CheckStatus::Warn
    } else {
        CheckStatus::Ok
    }
}

fn check(
    id: &str,
    label: &str,
    status: CheckStatus,
    message: impl Into<String>,
    detail: impl Into<String>,
    action_hint: impl Into<String>,
) -> HealthCheck {
    HealthCheck {
        id: id.into(),
        label: label.into(),
        status,
        message: message.into(),
        detail: detail.into(),
        action_hint: action_hint.into(),
    }
}

fn check_docker_binary() -> HealthCheck {
    match which::which("docker") {
        Ok(path) => check(
            "docker_binary",
            "Docker",
            CheckStatus::Ok,
            "Docker encontrado no PATH.",
            path.display().to_string(),
            "",
        ),
        Err(_) => check(
            "docker_binary",
            "Docker",
            CheckStatus::Error,
            "Docker não encontrado no PATH.",
            "O app precisa do Docker para iniciar e gerenciar os containers das VMs.",
            "Instale Docker Engine e reabra o winbox-gui.",
        ),
    }
}

fn check_docker_daemon() -> HealthCheck {
    if which::which("docker").is_err() {
        return check(
            "docker_daemon",
            "Docker daemon",
            CheckStatus::Error,
            "Daemon não verificado porque Docker não está instalado.",
            "",
            "Instale Docker antes de validar o daemon.",
        );
    }
    let status = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => check(
            "docker_daemon",
            "Docker daemon",
            CheckStatus::Ok,
            "Docker daemon acessível.",
            "",
            "",
        ),
        Ok(s) => check(
            "docker_daemon",
            "Docker daemon",
            CheckStatus::Error,
            "Docker daemon inacessível.",
            format!("docker info retornou exit={:?}", s.code()),
            "Inicie o serviço Docker e confirme que seu usuário pode acessá-lo.",
        ),
        Err(e) => check(
            "docker_daemon",
            "Docker daemon",
            CheckStatus::Error,
            "Falha ao executar docker info.",
            e.to_string(),
            "Verifique a instalação do Docker.",
        ),
    }
}

fn check_compose() -> HealthCheck {
    let compose_v2 = Command::new("docker")
        .args(["compose", "version"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if compose_v2 {
        return check(
            "compose",
            "Docker Compose",
            CheckStatus::Ok,
            "Docker Compose v2 disponível.",
            "docker compose version",
            "",
        );
    }
    let legacy = Command::new("docker-compose")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if legacy {
        return check(
            "compose",
            "Docker Compose",
            CheckStatus::Warn,
            "Usando docker-compose legado.",
            "docker compose v2 não respondeu, mas docker-compose está disponível.",
            "Prefira instalar o plugin Docker Compose v2.",
        );
    }
    check(
        "compose",
        "Docker Compose",
        CheckStatus::Error,
        "Docker Compose não disponível.",
        "Nem 'docker compose version' nem 'docker-compose version' responderam com sucesso.",
        "Instale o plugin docker-compose-v2 ou o binário docker-compose.",
    )
}

fn check_kvm() -> HealthCheck {
    let kvm = Path::new("/dev/kvm");
    if !kvm.exists() {
        return check(
            "kvm",
            "KVM",
            CheckStatus::Error,
            "/dev/kvm ausente.",
            "Sem KVM, QEMU roda sem aceleração ou falha ao iniciar.",
            "Habilite virtualização na BIOS/UEFI e carregue o módulo KVM.",
        );
    }
    match std::fs::OpenOptions::new().read(true).write(true).open(kvm) {
        Ok(_) => check("kvm", "KVM", CheckStatus::Ok, "/dev/kvm acessível.", "", ""),
        Err(e) => check(
            "kvm",
            "KVM",
            CheckStatus::Error,
            "/dev/kvm existe, mas não está acessível para este usuário.",
            e.to_string(),
            "Adicione seu usuário ao grupo kvm e faça login novamente.",
        ),
    }
}

fn check_storage() -> HealthCheck {
    let target = paths::data_dir();
    let probe = if target.exists() {
        target.as_path()
    } else {
        target.parent().unwrap_or_else(|| Path::new("/"))
    };
    let free = free_gb(probe);
    match free {
        Some(gb) if gb < 10 => check(
            "storage",
            "Storage",
            CheckStatus::Error,
            format!("Apenas {gb} GB livres para dados das VMs."),
            target.display().to_string(),
            "Libere espaço ou mova XDG_DATA_HOME antes de criar/atualizar VMs.",
        ),
        Some(gb) if gb < 32 => check(
            "storage",
            "Storage",
            CheckStatus::Warn,
            format!("{gb} GB livres. Pode ser pouco para Windows + snapshots."),
            target.display().to_string(),
            "Mantenha pelo menos 32 GB livres para uma experiência segura.",
        ),
        Some(gb) => check(
            "storage",
            "Storage",
            CheckStatus::Ok,
            format!("{gb} GB livres para dados das VMs."),
            target.display().to_string(),
            "",
        ),
        None => check(
            "storage",
            "Storage",
            CheckStatus::Warn,
            "Não foi possível calcular espaço livre.",
            target.display().to_string(),
            "Verifique permissões e montagem do diretório de dados.",
        ),
    }
}

fn free_gb(path: &Path) -> Option<u32> {
    let out = Command::new("df")
        .args(["-BG", path.to_str()?])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .nth(1)
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|v| v.trim_end_matches('G').parse::<u32>().ok())
}

fn check_profiles() -> HealthCheck {
    let names = profile::list_names();
    if names.is_empty() {
        return check(
            "profiles",
            "Profiles",
            CheckStatus::Ok,
            "Nenhum perfil criado ainda.",
            paths::profiles_cfg_dir().display().to_string(),
            "",
        );
    }
    let missing_env: Vec<String> = names
        .iter()
        .filter(|name| !profile::is_env_present(name))
        .cloned()
        .collect();
    if !missing_env.is_empty() {
        return check(
            "profiles",
            "Profiles",
            CheckStatus::Warn,
            format!("{} perfil(is) sem config.env.", missing_env.len()),
            missing_env.join(", "),
            "Remova ou recrie perfis incompletos.",
        );
    }
    check(
        "profiles",
        "Profiles",
        CheckStatus::Ok,
        format!("{} perfil(is) com configuração presente.", names.len()),
        paths::profiles_cfg_dir().display().to_string(),
        "",
    )
}

fn check_ports() -> HealthCheck {
    let names = profile::list_names();
    if names.is_empty() {
        return check(
            "ports",
            "Ports",
            CheckStatus::Ok,
            "Nenhuma porta reservada ainda.",
            "",
            "",
        );
    }

    let mut seen: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    let mut busy_when_stopped = Vec::new();
    let mut invalid = Vec::new();
    for name in names {
        let env_path = paths::profile_env_file(&name);
        let Ok(map) = env_file::read(&env_path) else {
            continue;
        };
        let container = paths::profile_container(&name);
        let runningish = matches!(
            docker::container_status(&container).as_str(),
            "running" | "paused"
        );
        for key in ["WEB_PORT", "RDP_PORT", "SSH_PORT"] {
            let port = env_file::get_u16(&map, key);
            if port == 0 {
                invalid.push(format!("{name}:{key}=0"));
                continue;
            }
            seen.entry(port).or_default().push(format!("{name}:{key}"));
            if !runningish && !ports::port_free(port) {
                busy_when_stopped.push(format!("{name}:{key}={port}"));
            }
        }
    }

    let duplicates: Vec<String> = seen
        .iter()
        .filter(|(_, refs)| refs.len() > 1)
        .map(|(port, refs)| format!("{port} ({})", refs.join(", ")))
        .collect();

    if !duplicates.is_empty() || !invalid.is_empty() {
        return check(
            "ports",
            "Ports",
            CheckStatus::Error,
            "Conflito ou porta inválida em perfis.",
            [duplicates.join("; "), invalid.join("; ")]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" | "),
            "Ajuste as portas no perfil antes de iniciar a VM.",
        );
    }
    if !busy_when_stopped.is_empty() {
        return check(
            "ports",
            "Ports",
            CheckStatus::Warn,
            "Há portas reservadas ocupadas enquanto a VM não está rodando.",
            busy_when_stopped.join(", "),
            "Feche o processo que usa a porta ou altere o port forwarding.",
        );
    }
    check(
        "ports",
        "Ports",
        CheckStatus::Ok,
        "Portas dos perfis sem conflito aparente.",
        "",
        "",
    )
}

fn check_gpu_vfio() -> HealthCheck {
    let gpus = gpu::list();
    let selected = selected_gpu_bdfs();
    if gpus.is_empty() && selected.is_empty() {
        return check(
            "gpu_vfio",
            "GPU / VFIO",
            CheckStatus::Ok,
            "Nenhuma GPU passthrough configurada.",
            "",
            "",
        );
    }
    if gpus.is_empty() && !selected.is_empty() {
        return check(
            "gpu_vfio",
            "GPU / VFIO",
            CheckStatus::Warn,
            "Perfis pedem GPU, mas GPUs não foram detectadas.",
            selected.into_iter().collect::<Vec<_>>().join(", "),
            "Instale lspci/pciutils e confirme a GPU no host.",
        );
    }
    let iommu_ok = gpus.iter().all(|g| g.vfio_capable);
    let mut missing = Vec::new();
    let mut not_ready = Vec::new();
    for bdf in selected {
        match gpus.iter().find(|g| g.bdf == bdf) {
            Some(g) if !g.vfio_ready => not_ready.push(format!("{bdf} ({})", g.model)),
            Some(_) => {}
            None => missing.push(bdf),
        }
    }
    if !iommu_ok {
        return check(
            "gpu_vfio",
            "GPU / VFIO",
            CheckStatus::Warn,
            "IOMMU/VFIO não parece pronto para passthrough.",
            "O diretório /sys/kernel/iommu_groups não foi detectado.",
            "Habilite intel_iommu=on ou amd_iommu=on e reinicie.",
        );
    }
    if !missing.is_empty() || !not_ready.is_empty() {
        return check(
            "gpu_vfio",
            "GPU / VFIO",
            CheckStatus::Error,
            "GPU configurada em perfil não está pronta para VFIO.",
            [missing.join(", "), not_ready.join(", ")]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" | "),
            "Abra Settings do perfil e configure/remova a GPU antes de iniciar.",
        );
    }
    check(
        "gpu_vfio",
        "GPU / VFIO",
        CheckStatus::Ok,
        format!(
            "{} GPU(s) detectada(s); VFIO sem bloqueios críticos.",
            gpus.len()
        ),
        "",
        "",
    )
}

fn selected_gpu_bdfs() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for name in profile::list_names() {
        let env_path = paths::profile_env_file(&name);
        let Ok(map) = env_file::read(&env_path) else {
            continue;
        };
        let bdf = env_file::get(&map, "GPU_BDF").trim();
        if !bdf.is_empty() {
            out.insert(bdf.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(status: CheckStatus) -> HealthCheck {
        check("x", "X", status, "", "", "")
    }

    #[test]
    fn overall_status_uses_worst_check() {
        assert_eq!(overall_status(&[sample(CheckStatus::Ok)]), CheckStatus::Ok);
        assert_eq!(
            overall_status(&[sample(CheckStatus::Ok), sample(CheckStatus::Warn)]),
            CheckStatus::Warn
        );
        assert_eq!(
            overall_status(&[sample(CheckStatus::Warn), sample(CheckStatus::Error)]),
            CheckStatus::Error
        );
    }
}

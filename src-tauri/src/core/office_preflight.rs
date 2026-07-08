use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{
    docker::DockerClient,
    flatpak::{self, FlatpakClient, FlatpakInstallConsent},
    host, paths,
};

pub const MIN_RAM_GB: u32 = 4;
pub const RECOMMENDED_RAM_GB: u32 = 8;
pub const MIN_CPU_CORES: u32 = 2;
pub const RECOMMENDED_CPU_CORES: u32 = 4;
pub const MIN_DISK_GB: u32 = 64;
pub const RECOMMENDED_DISK_GB: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightStatus {
    Ok,
    Warning,
    Blocker,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreflightCheck {
    pub id: String,
    pub status: PreflightStatus,
    pub requirement: String,
    pub impact: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resources {
    #[serde(alias = "ram_gb")]
    pub ram_gb: u32,
    #[serde(alias = "cpu_cores")]
    pub cpu_cores: u32,
    #[serde(alias = "disk_gb")]
    pub disk_gb: u32,
    #[serde(
        default,
        alias = "storage_path",
        skip_serializing_if = "Option::is_none"
    )]
    pub storage_path: Option<String>,
    #[serde(default, alias = "warning_override")]
    pub warning_override: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficePreflightResult {
    pub checks: Vec<PreflightCheck>,
    pub warnings: usize,
    pub blockers: usize,
    pub adoption_candidates: Vec<serde_json::Value>,
}

// Task 10 expands this enum with the full Office taxonomy. Task 4 only needs
// the preflight subset so callers can rely on stable codes from day one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum OfficeError {
    PreflightKvmMissing,
    PreflightDockerMissing,
    PreflightSubnetConflict,
    FlatpakFreerdpMissing,
    FlatpakHomeOverrideMissing,
    NativeFreerdpTooOld,
}

impl PreflightCheck {
    pub fn ok(id: &str, requirement: &str, impact: &str) -> Self {
        Self::new(id, PreflightStatus::Ok, requirement, impact, None, None)
    }

    pub fn warning(id: &str, requirement: &str, impact: &str, action_hint: &str) -> Self {
        Self::new(
            id,
            PreflightStatus::Warning,
            requirement,
            impact,
            Some(action_hint.to_string()),
            None,
        )
    }

    pub fn blocker(id: &str, requirement: &str, impact: &str, action_hint: &str) -> Self {
        Self::new(
            id,
            PreflightStatus::Blocker,
            requirement,
            impact,
            Some(action_hint.to_string()),
            None,
        )
    }

    pub fn new(
        id: &str,
        status: PreflightStatus,
        requirement: &str,
        impact: &str,
        action_hint: Option<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id: id.to_string(),
            status,
            requirement: requirement.to_string(),
            impact: impact.to_string(),
            action_hint,
            details,
        }
    }
}

impl Resources {
    pub fn validate(&self) -> Result<()> {
        if self.ram_gb == 0 {
            bail!("ramGb deve ser maior que zero.");
        }
        if self.cpu_cores == 0 {
            bail!("cpuCores deve ser maior que zero.");
        }
        if self.disk_gb == 0 {
            bail!("diskGb deve ser maior que zero.");
        }
        Ok(())
    }

    pub fn storage_path_buf(&self) -> PathBuf {
        self.storage_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(paths::data_dir)
    }
}

pub trait HostPreflight {
    fn kvm_status(&self) -> KvmStatus;
    fn connectivity_available(&self) -> bool;
    fn host_resources(&self, storage_path: &Path) -> HostResources;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KvmStatus {
    Available,
    Missing,
    PermissionDenied(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostResources {
    pub ram_gb: u32,
    pub cpu_cores: u32,
    pub free_disk_gb: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CliHostPreflight;

impl HostPreflight for CliHostPreflight {
    fn kvm_status(&self) -> KvmStatus {
        let kvm = Path::new("/dev/kvm");
        if !kvm.exists() {
            return KvmStatus::Missing;
        }
        match std::fs::OpenOptions::new().read(true).write(true).open(kvm) {
            Ok(_) => KvmStatus::Available,
            Err(err) => KvmStatus::PermissionDenied(err.to_string()),
        }
    }

    fn connectivity_available(&self) -> bool {
        let addr = SocketAddr::from(([1, 1, 1, 1], 443));
        TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
    }

    fn host_resources(&self, storage_path: &Path) -> HostResources {
        let info = host::info();
        HostResources {
            ram_gb: info.ram_gb,
            cpu_cores: info.cpu_cores,
            free_disk_gb: free_space_gb_for(storage_path).unwrap_or(info.free_gb),
        }
    }
}

pub fn run_preflight(
    resources: &Resources,
    docker: &dyn DockerClient,
    flatpak: &dyn FlatpakClient,
    host: &dyn HostPreflight,
) -> Result<OfficePreflightResult> {
    resources.validate()?;
    let storage_path = resources.storage_path_buf();
    let host_resources = host.host_resources(&storage_path);
    let checks = classify_preflight(resources, docker, flatpak, host, host_resources);
    validate_checks(&checks)?;
    let warnings = checks
        .iter()
        .filter(|check| check.status == PreflightStatus::Warning)
        .count();
    let blockers = checks
        .iter()
        .filter(|check| check.status == PreflightStatus::Blocker)
        .count();
    Ok(OfficePreflightResult {
        checks,
        warnings,
        blockers,
        adoption_candidates: Vec::new(),
    })
}

pub fn classify_preflight(
    resources: &Resources,
    docker: &dyn DockerClient,
    flatpak: &dyn FlatpakClient,
    host: &dyn HostPreflight,
    host_resources: HostResources,
) -> Vec<PreflightCheck> {
    let mut checks = vec![
        classify_kvm(host.kvm_status()),
        classify_docker(docker),
        flatpak::detect_freerdp(flatpak, FlatpakInstallConsent::Unknown).check,
        classify_connectivity(host.connectivity_available()),
    ];
    checks.extend(classify_resources(resources, host_resources));
    checks
}

pub fn validate_checks(checks: &[PreflightCheck]) -> Result<()> {
    for check in checks {
        if check.status == PreflightStatus::Blocker
            && check
                .action_hint
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            bail!("Preflight blocker '{}' não tem action_hint.", check.id);
        }
    }
    Ok(())
}

fn classify_kvm(status: KvmStatus) -> PreflightCheck {
    match status {
        KvmStatus::Available => PreflightCheck::ok(
            "preflight_kvm",
            "KVM acessível no host.",
            "A VM Windows pode usar aceleração de hardware.",
        ),
        KvmStatus::Missing => PreflightCheck::blocker(
            "preflight_kvm_missing",
            "/dev/kvm deve existir.",
            "Sem KVM, o Windows fica impraticável ou falha ao iniciar.",
            "Habilite virtualização na BIOS/UEFI e carregue o módulo KVM.",
        ),
        KvmStatus::PermissionDenied(detail) => PreflightCheck::new(
            "preflight_kvm_missing",
            PreflightStatus::Blocker,
            "/dev/kvm deve estar acessível ao usuário.",
            "Sem acesso ao KVM, o winbox não consegue iniciar a VM acelerada.",
            Some("Adicione seu usuário ao grupo kvm e faça login novamente.".to_string()),
            Some(serde_json::json!({ "detail": detail })),
        ),
    }
}

fn classify_docker(docker: &dyn DockerClient) -> PreflightCheck {
    if !docker.docker_binary_available() {
        return PreflightCheck::blocker(
            "preflight_docker_missing",
            "Docker deve estar instalado no PATH.",
            "O perfil Office depende de Docker para criar e operar a VM Windows.",
            "Instale Docker Engine e reabra o winbox.",
        );
    }
    if !docker.docker_daemon_available() {
        return PreflightCheck::blocker(
            "preflight_docker_missing",
            "Docker daemon deve estar acessível.",
            "Sem daemon Docker, o winbox não consegue criar ou iniciar containers.",
            "Inicie o serviço Docker e confirme que seu usuário acessa /var/run/docker.sock.",
        );
    }
    if !docker.docker_compose_available() {
        return PreflightCheck::blocker(
            "preflight_docker_missing",
            "Docker Compose deve estar disponível.",
            "Sem Compose, o winbox não consegue aplicar o compose.yml do perfil.",
            "Instale o plugin Docker Compose v2 ou o binário docker-compose.",
        );
    }
    PreflightCheck::ok(
        "preflight_docker",
        "Docker, daemon e Compose disponíveis.",
        "O winbox pode criar e gerenciar o container Windows.",
    )
}

fn classify_connectivity(available: bool) -> PreflightCheck {
    if available {
        return PreflightCheck::ok(
            "preflight_connectivity",
            "Host deve ter conectividade de saída.",
            "ODT e Docker podem baixar os artefatos necessários.",
        );
    }
    PreflightCheck::blocker(
        "preflight_connectivity",
        "Host deve alcançar a internet para provisionar Office.",
        "Docker/ODT podem falhar ao baixar imagem Windows ou payload Microsoft 365.",
        "Conecte o host à internet ou libere saída HTTPS antes de provisionar.",
    )
}

fn classify_resources(requested: &Resources, host_resources: HostResources) -> Vec<PreflightCheck> {
    vec![
        classify_ram(requested.ram_gb, host_resources.ram_gb),
        classify_cpu(requested.cpu_cores, host_resources.cpu_cores),
        classify_disk(requested.disk_gb, host_resources.free_disk_gb),
    ]
}

fn classify_ram(requested: u32, host_total: u32) -> PreflightCheck {
    if host_total > 0 && host_total < requested {
        return PreflightCheck::blocker(
            "preflight_resources",
            "RAM física suficiente para o perfil Office.",
            "A VM não deve receber mais RAM do que o host disponível.",
            "Reduza RAM do perfil ou use um host com mais memória.",
        );
    }
    if requested < MIN_RAM_GB {
        return PreflightCheck::warning(
            "preflight_resources",
            "RAM mínima recomendada para Office.",
            "Abaixo de 4 GB, Windows + Office pode travar ou ficar inutilizável.",
            "Ajuste para pelo menos 4 GB ou prossiga assumindo o risco.",
        );
    }
    if requested < RECOMMENDED_RAM_GB {
        return PreflightCheck::warning(
            "preflight_resources",
            "RAM recomendada para Office.",
            "Menos de 8 GB pode deixar Excel/Word/PowerPoint lentos.",
            "Use 8 GB ou mais quando possível.",
        );
    }
    PreflightCheck::ok(
        "preflight_resources",
        "RAM adequada para o perfil Office.",
        "A VM tem memória suficiente para o MVP.",
    )
}

fn classify_cpu(requested: u32, host_total: u32) -> PreflightCheck {
    if host_total > 0 && host_total < requested {
        return PreflightCheck::warning(
            "preflight_resources",
            "CPU disponível no host.",
            "Pedir mais vCPU do que o host expõe pode degradar a VM e o desktop Linux.",
            "Reduza cpuCores para caber nos cores disponíveis.",
        );
    }
    if requested < MIN_CPU_CORES {
        return PreflightCheck::warning(
            "preflight_resources",
            "CPU mínima para Windows + Office.",
            "Com menos de 2 vCPU, o provisionamento pode ficar muito lento.",
            "Use pelo menos 2 vCPU.",
        );
    }
    if requested < RECOMMENDED_CPU_CORES {
        return PreflightCheck::warning(
            "preflight_resources",
            "CPU recomendada para Windows + Office.",
            "Menos de 4 vCPU pode alongar instalação e cold-start.",
            "Use 4 vCPU ou mais quando possível.",
        );
    }
    PreflightCheck::ok(
        "preflight_resources",
        "CPU adequada para o perfil Office.",
        "A VM tem vCPU suficiente para o MVP.",
    )
}

fn classify_disk(requested: u32, free: u32) -> PreflightCheck {
    if free > 0 && free < requested {
        return PreflightCheck::blocker(
            "preflight_resources",
            "Espaço livre suficiente para o disco da VM.",
            "Sem espaço livre, Docker/Windows/Office podem falhar no meio da instalação.",
            "Libere espaço ou escolha outro storagePath antes de provisionar.",
        );
    }
    if requested < MIN_DISK_GB {
        return PreflightCheck::warning(
            "preflight_resources",
            "Disco mínimo para Windows + Office.",
            "Abaixo de 64 GB, Windows Update e Office podem ficar sem espaço.",
            "Use pelo menos 64 GB ou prossiga assumindo o risco.",
        );
    }
    if requested < RECOMMENDED_DISK_GB {
        return PreflightCheck::warning(
            "preflight_resources",
            "Disco recomendado para Windows + Office.",
            "Menos de 128 GB reduz margem para cache, updates e arquivos temporários.",
            "Use 128 GB quando possível.",
        );
    }
    PreflightCheck::ok(
        "preflight_resources",
        "Disco adequado para o perfil Office.",
        "Há espaço suficiente para o MVP.",
    )
}

fn free_space_gb_for(path: &Path) -> Option<u32> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best: Option<(usize, u64)> = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if path.starts_with(mount) {
            let len = mount.as_os_str().len();
            if best.map(|(old_len, _)| len > old_len).unwrap_or(true) {
                best = Some((len, disk.available_space()));
            }
        }
    }
    best.map(|(_, bytes)| (bytes / 1024 / 1024 / 1024) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::docker::mock::MockDocker;
    use crate::core::flatpak::mock::MockFlatpakClient;

    #[derive(Debug, Clone)]
    struct FakeHost {
        kvm: KvmStatus,
        connectivity: bool,
        resources: HostResources,
    }

    impl HostPreflight for FakeHost {
        fn kvm_status(&self) -> KvmStatus {
            self.kvm.clone()
        }

        fn connectivity_available(&self) -> bool {
            self.connectivity
        }

        fn host_resources(&self, _storage_path: &Path) -> HostResources {
            self.resources
        }
    }

    #[test]
    fn preflight_check_has_action_hint_for_blocker() {
        let checks = vec![
            PreflightCheck::blocker(
                "preflight_kvm_missing",
                "/dev/kvm deve existir.",
                "Sem KVM, a VM não inicia corretamente.",
                "Habilite virtualização na BIOS/UEFI.",
            ),
            PreflightCheck::warning(
                "preflight_resources",
                "RAM recomendada.",
                "Pouca RAM pode deixar Office lento.",
                "Use 8 GB quando possível.",
            ),
        ];

        validate_checks(&checks).expect("blocker com action_hint deve ser aceito");

        let invalid = vec![PreflightCheck::new(
            "preflight_docker_missing",
            PreflightStatus::Blocker,
            "Docker deve existir.",
            "Sem Docker não há VM.",
            None,
            None,
        )];
        let err = validate_checks(&invalid).expect_err("blocker sem hint deve falhar");
        assert!(format!("{err:#}").contains("action_hint"));
    }

    #[test]
    fn preflight_uses_mock_docker_without_daemon() {
        let docker = MockDocker::new();
        docker.seed_preflight(true, false, true);
        let flatpak = MockFlatpakClient::new();
        flatpak.seed_xfreerdp_version(
            "xfreerdp",
            Some(crate::core::flatpak::CommandOutput {
                success: true,
                stdout: "This is FreeRDP version 3.28.0".to_string(),
                stderr: String::new(),
            }),
        );
        let host = FakeHost {
            kvm: KvmStatus::Available,
            connectivity: true,
            resources: HostResources {
                ram_gb: 32,
                cpu_cores: 8,
                free_disk_gb: 512,
            },
        };
        let resources = Resources {
            ram_gb: 8,
            cpu_cores: 4,
            disk_gb: 128,
            storage_path: None,
            warning_override: false,
        };

        let result =
            run_preflight(&resources, &docker, &flatpak, &host).expect("preflight should classify");

        assert_eq!(result.blockers, 1);
        assert!(result.checks.iter().any(|check| {
            check.id == "preflight_docker_missing"
                && check.status == PreflightStatus::Blocker
                && check.requirement.contains("daemon")
        }));
    }
}

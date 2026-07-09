use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::{
    docker::DockerClient,
    flatpak::{self, FlatpakClient, FlatpakInstallConsent},
    host,
    launch_error::OfficeError,
    paths,
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
    fn ip_routes(&self) -> Result<String>;
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

    fn ip_routes(&self) -> Result<String> {
        let out = Command::new("ip").arg("route").output()?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
        }
        bail!(
            "ip route falhou: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )
    }
}

pub fn run_preflight(
    profile_name: Option<&str>,
    resources: &Resources,
    docker: &dyn DockerClient,
    flatpak: &dyn FlatpakClient,
    host: &dyn HostPreflight,
) -> Result<OfficePreflightResult> {
    resources.validate()?;
    let storage_path = resources.storage_path_buf();
    let host_resources = host.host_resources(&storage_path);
    let checks = classify_preflight(
        profile_name,
        resources,
        docker,
        flatpak,
        host,
        host_resources,
    );
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
    profile_name: Option<&str>,
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
    checks.push(classify_subnet(profile_name, docker, host));
    checks.extend(classify_resources(resources, host_resources));
    if resource_warning_override_required(resources, host_resources) {
        checks.push(PreflightCheck::blocker(
            "preflight_warning_override_required",
            "Warnings de RAM/disco precisam de confirmação explícita.",
            "O perfil Office pode prosseguir com recursos abaixo do recomendado, mas só depois do usuário aceitar o risco.",
            "Ative warningOverride no wizard ou aumente RAM/disco para os valores recomendados.",
        ));
    }
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
            OfficeError::PREFLIGHT_KVM_MISSING,
            "/dev/kvm deve existir.",
            "Sem KVM, o Windows fica impraticável ou falha ao iniciar.",
            "Habilite virtualização na BIOS/UEFI e carregue o módulo KVM.",
        ),
        KvmStatus::PermissionDenied(detail) => PreflightCheck::new(
            OfficeError::PREFLIGHT_KVM_MISSING,
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
            OfficeError::PREFLIGHT_DOCKER_MISSING,
            "Docker deve estar instalado no PATH.",
            "O perfil Office depende de Docker para criar e operar a VM Windows.",
            "Instale Docker Engine e reabra o winbox.",
        );
    }
    if !docker.docker_daemon_available() {
        return PreflightCheck::blocker(
            OfficeError::PREFLIGHT_DOCKER_MISSING,
            "Docker daemon deve estar acessível.",
            "Sem daemon Docker, o winbox não consegue criar ou iniciar containers.",
            "Inicie o serviço Docker e confirme que seu usuário acessa /var/run/docker.sock.",
        );
    }
    if !docker.docker_compose_available() {
        return PreflightCheck::blocker(
            OfficeError::PREFLIGHT_DOCKER_MISSING,
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
        return PreflightCheck::warning(
            "preflight_resources",
            "RAM física suficiente para o perfil Office.",
            "Pedir mais RAM do que o host reporta pode degradar o Linux e travar o Windows.",
            "Reduza RAM do perfil, use um host com mais memória ou prossiga assumindo o risco.",
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
        return PreflightCheck::warning(
            "preflight_resources",
            "Espaço livre suficiente para o disco da VM.",
            "Sem folga de disco, Docker/Windows/Office podem falhar no meio da instalação.",
            "Libere espaço, escolha outro storagePath ou prossiga assumindo o risco.",
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

fn resource_warning_override_required(
    requested: &Resources,
    host_resources: HostResources,
) -> bool {
    !requested.warning_override
        && (requested.ram_gb < RECOMMENDED_RAM_GB
            || requested.disk_gb < RECOMMENDED_DISK_GB
            || (host_resources.ram_gb > 0 && host_resources.ram_gb < requested.ram_gb)
            || (host_resources.free_disk_gb > 0 && host_resources.free_disk_gb < requested.disk_gb))
}

fn classify_subnet(
    profile_name: Option<&str>,
    docker: &dyn DockerClient,
    host: &dyn HostPreflight,
) -> PreflightCheck {
    let network = compose_default_network(profile_name);

    // Best-effort by design: Docker can pick a different pool after preflight.
    // Failures here must not hide blockers from KVM/Docker/FreeRDP/connectivity.
    let docker_cidrs = docker
        .docker_network_inspect(&network)
        .map(|output| parse_docker_network_cidrs(&output))
        .unwrap_or_default()
        .into_iter()
        .chain(
            docker
                .docker_daemon_json()
                .map(|output| parse_daemon_default_address_pools(&output))
                .unwrap_or_default(),
        )
        .collect::<Vec<_>>();
    let route_cidrs = host
        .ip_routes()
        .map(|output| parse_route_cidrs(&output))
        .unwrap_or_default();

    if let Some((docker_cidr, route_cidr)) = detect_subnet_conflict(&docker_cidrs, &route_cidrs) {
        let action_hint = "Configure default-address-pools em /etc/docker/daemon.json com uma faixa que não sobreponha a rede local e recrie a rede do perfil.";
        return PreflightCheck::new(
            OfficeError::PREFLIGHT_SUBNET_CONFLICT,
            PreflightStatus::Blocker,
            "Subnet Docker do perfil não deve sobrepor rotas locais.",
            "Conflito de rede pode impedir RDP, downloads do ODT ou acesso do guest à internet.",
            Some(action_hint.to_string()),
            Some(serde_json::json!({
                "bestEffort": true,
                "network": network,
                "dockerCidr": docker_cidr.to_string(),
                "hostRoute": route_cidr.to_string(),
                "action_hint": action_hint,
            })),
        );
    }

    PreflightCheck::new(
        "preflight_subnet",
        PreflightStatus::Ok,
        "Subnet Docker do perfil deve ser compatível com rotas locais.",
        "Nenhuma sobreposição foi detectada na checagem preventiva.",
        None,
        Some(serde_json::json!({
            "bestEffort": true,
            "network": network,
            "dockerCidrs": docker_cidrs.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "hostRoutes": route_cidrs.iter().map(ToString::to_string).collect::<Vec<_>>(),
        })),
    )
}

fn compose_default_network(profile_name: Option<&str>) -> String {
    let profile = profile_name
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("office");
    format!("winbox-{profile}_default")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Ipv4Cidr {
    addr: u32,
    prefix: u8,
}

impl Ipv4Cidr {
    fn parse(value: &str) -> Option<Self> {
        let (ip, prefix) = value.trim().split_once('/')?;
        let ip = ip.parse::<Ipv4Addr>().ok()?;
        let prefix = prefix.parse::<u8>().ok()?;
        if prefix > 32 {
            return None;
        }
        Some(Self {
            addr: u32::from(ip),
            prefix,
        })
    }

    fn bounds(self) -> (u32, u32) {
        let mask = if self.prefix == 0 {
            0
        } else {
            u32::MAX << (32 - self.prefix)
        };
        let start = self.addr & mask;
        let end = start | !mask;
        (start, end)
    }
}

impl std::fmt::Display for Ipv4Cidr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", Ipv4Addr::from(self.addr), self.prefix)
    }
}

fn parse_route_cidrs(output: &str) -> Vec<Ipv4Cidr> {
    output
        .lines()
        .filter(|line| !is_docker_owned_route(line))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|first| *first != "default")
        .filter_map(Ipv4Cidr::parse)
        .collect()
}

fn is_docker_owned_route(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "dev" {
            let device = tokens.next().unwrap_or("");
            return device.starts_with("docker")
                || device.starts_with("br-")
                || device.starts_with("veth")
                || device.starts_with("virbr");
        }
    }
    false
}

fn parse_docker_network_cidrs(output: &str) -> Vec<Ipv4Cidr> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(output) else {
        return Vec::new();
    };
    value
        .as_array()
        .into_iter()
        .flat_map(|networks| networks.iter())
        .flat_map(|network| {
            network
                .pointer("/IPAM/Config")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flat_map(|configs| configs.iter())
        })
        .filter_map(|config| config.get("Subnet").and_then(serde_json::Value::as_str))
        .filter_map(Ipv4Cidr::parse)
        .collect()
}

fn parse_daemon_default_address_pools(output: &str) -> Vec<Ipv4Cidr> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(output) else {
        return Vec::new();
    };
    value
        .get("default-address-pools")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flat_map(|pools| pools.iter())
        .filter_map(|pool| pool.get("base").and_then(serde_json::Value::as_str))
        .filter_map(Ipv4Cidr::parse)
        .collect()
}

fn detect_subnet_conflict(
    docker_cidrs: &[Ipv4Cidr],
    route_cidrs: &[Ipv4Cidr],
) -> Option<(Ipv4Cidr, Ipv4Cidr)> {
    docker_cidrs.iter().find_map(|docker_cidr| {
        route_cidrs
            .iter()
            .find(|route_cidr| cidr_overlaps(*docker_cidr, **route_cidr))
            .map(|route_cidr| (*docker_cidr, *route_cidr))
    })
}

fn cidr_overlaps(left: Ipv4Cidr, right: Ipv4Cidr) -> bool {
    let (left_start, left_end) = left.bounds();
    let (right_start, right_end) = right.bounds();
    left_start <= right_end && right_start <= left_end
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
        routes: Option<String>,
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

        fn ip_routes(&self) -> Result<String> {
            self.routes
                .clone()
                .ok_or_else(|| anyhow::anyhow!("ip route indisponível"))
        }
    }

    fn native_freerdp_ok(flatpak: &MockFlatpakClient) {
        flatpak.seed_xfreerdp_version(
            "xfreerdp",
            Some(crate::core::flatpak::CommandOutput {
                success: true,
                stdout: "This is FreeRDP version 3.28.0".to_string(),
                stderr: String::new(),
            }),
        );
    }

    fn good_host(routes: Option<&str>) -> FakeHost {
        FakeHost {
            kvm: KvmStatus::Available,
            connectivity: true,
            resources: HostResources {
                ram_gb: 32,
                cpu_cores: 8,
                free_disk_gb: 512,
            },
            routes: routes.map(str::to_string),
        }
    }

    fn recommended_resources() -> Resources {
        Resources {
            ram_gb: 8,
            cpu_cores: 4,
            disk_gb: 128,
            storage_path: None,
            warning_override: false,
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
        native_freerdp_ok(&flatpak);
        let host = FakeHost {
            kvm: KvmStatus::Available,
            connectivity: true,
            resources: HostResources {
                ram_gb: 32,
                cpu_cores: 8,
                free_disk_gb: 512,
            },
            routes: Some(String::new()),
        };
        let resources = recommended_resources();

        let result = run_preflight(Some("office"), &resources, &docker, &flatpak, &host)
            .expect("preflight should classify");

        assert_eq!(result.blockers, 1);
        assert!(result.checks.iter().any(|check| {
            check.id == "preflight_docker_missing"
                && check.status == PreflightStatus::Blocker
                && check.requirement.contains("daemon")
        }));
    }

    #[test]
    fn parse_route_cidrs_reads_real_ip_route_output() {
        let output = "\
default via 192.168.15.1 dev wlp0s20f3 proto dhcp metric 600
172.17.0.0/16 dev docker0 proto kernel scope link src 172.17.0.1
172.30.10.0/24 dev enp5s0 proto kernel scope link src 172.30.10.20
192.168.15.0/24 dev wlp0s20f3 proto kernel scope link src 192.168.15.44 metric 600
";

        let cidrs = parse_route_cidrs(output);

        assert_eq!(
            cidrs.iter().map(ToString::to_string).collect::<Vec<_>>(),
            vec!["172.30.10.0/24", "192.168.15.0/24"]
        );
        assert!(parse_route_cidrs("").is_empty());
    }

    #[test]
    fn parse_docker_cidrs_reads_network_inspect_and_daemon_pools() {
        let inspect = r#"[
          {
            "Name": "winbox-office_default",
            "IPAM": {
              "Config": [
                { "Subnet": "172.30.0.0/16", "Gateway": "172.30.0.1" }
              ]
            }
          }
        ]"#;
        let daemon = r#"{
          "default-address-pools": [
            { "base": "10.89.0.0/16", "size": 24 }
          ]
        }"#;

        assert_eq!(
            parse_docker_network_cidrs(inspect)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["172.30.0.0/16"]
        );
        assert_eq!(
            parse_daemon_default_address_pools(daemon)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            vec!["10.89.0.0/16"]
        );
        assert!(parse_docker_network_cidrs("not json").is_empty());
    }

    #[test]
    fn detect_subnet_conflict_detects_overlap_and_ignores_disjoint() {
        let docker = vec![Ipv4Cidr::parse("172.30.0.0/16").unwrap()];
        let route = vec![Ipv4Cidr::parse("172.30.10.0/24").unwrap()];
        let disjoint = vec![Ipv4Cidr::parse("192.168.15.0/24").unwrap()];

        let conflict = detect_subnet_conflict(&docker, &route).expect("should overlap");

        assert_eq!(conflict.0.to_string(), "172.30.0.0/16");
        assert_eq!(conflict.1.to_string(), "172.30.10.0/24");
        assert!(detect_subnet_conflict(&docker, &disjoint).is_none());
    }

    #[test]
    fn subnet_conflict_detected_between_docker_and_host_routes() {
        let docker = MockDocker::new();
        docker.seed_preflight(true, true, true);
        docker.seed_network_inspect(
            "winbox-office_default",
            Some(
                r#"[{"Name":"winbox-office_default","IPAM":{"Config":[{"Subnet":"172.30.0.0/16"}]}}]"#,
            ),
        );
        let flatpak = MockFlatpakClient::new();
        native_freerdp_ok(&flatpak);
        let host = good_host(Some(
            "172.30.10.0/24 dev enp5s0 proto kernel scope link src 172.30.10.20\n",
        ));
        let resources = recommended_resources();

        let result = run_preflight(Some("office"), &resources, &docker, &flatpak, &host)
            .expect("preflight should classify subnet conflict");
        let check = result
            .checks
            .iter()
            .find(|check| check.id == "preflight_subnet_conflict")
            .expect("subnet conflict should be reported");

        assert_eq!(check.status, PreflightStatus::Blocker);
        assert!(check
            .action_hint
            .as_deref()
            .unwrap_or("")
            .contains("default-address-pools"));
        assert!(check
            .details
            .as_ref()
            .and_then(|details| details.get("action_hint"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .contains("default-address-pools"));
    }

    #[test]
    fn resource_warning_requires_explicit_override() {
        let docker = MockDocker::new();
        docker.seed_preflight(true, true, true);
        let flatpak = MockFlatpakClient::new();
        native_freerdp_ok(&flatpak);
        let host = good_host(Some(""));
        let resources_without_override = Resources {
            ram_gb: 4,
            cpu_cores: 4,
            disk_gb: 64,
            storage_path: None,
            warning_override: false,
        };

        let blocked = run_preflight(
            Some("office"),
            &resources_without_override,
            &docker,
            &flatpak,
            &host,
        )
        .expect("resource warning should classify");

        assert!(blocked.checks.iter().any(|check| {
            check.id == "preflight_resources" && check.status == PreflightStatus::Warning
        }));
        assert!(blocked.checks.iter().any(|check| {
            check.id == "preflight_warning_override_required"
                && check.status == PreflightStatus::Blocker
        }));

        let resources_with_override = Resources {
            warning_override: true,
            ..resources_without_override
        };
        let allowed = run_preflight(
            Some("office"),
            &resources_with_override,
            &docker,
            &flatpak,
            &host,
        )
        .expect("explicit override should allow warnings");

        assert!(allowed.checks.iter().any(|check| {
            check.id == "preflight_resources" && check.status == PreflightStatus::Warning
        }));
        assert!(!allowed
            .checks
            .iter()
            .any(|check| check.id == "preflight_warning_override_required"));
    }

    #[test]
    fn subnet_check_best_effort_never_masks_other_blockers() {
        let docker = MockDocker::new();
        docker.seed_preflight(false, true, true);
        docker.seed_network_inspect("winbox-office_default", None);
        let flatpak = MockFlatpakClient::new();
        native_freerdp_ok(&flatpak);
        let host = FakeHost {
            routes: None,
            ..good_host(None)
        };
        let resources = recommended_resources();

        let result = run_preflight(Some("office"), &resources, &docker, &flatpak, &host)
            .expect("best-effort subnet failure should not fail preflight classification");

        assert!(result.checks.iter().any(|check| {
            check.id == "preflight_docker_missing" && check.status == PreflightStatus::Blocker
        }));
        assert!(!result
            .checks
            .iter()
            .any(|check| check.id == "preflight_subnet_conflict"));
    }
}

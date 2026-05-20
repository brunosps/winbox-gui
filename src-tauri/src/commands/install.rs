use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::core::image_family::ImageFamily;
use crate::core::{
    compose, desktop, docker, gpu_hooks, host, oem, paths, ports, profile, validation,
};

#[derive(Debug, Deserialize, Clone)]
pub struct InstallParams {
    pub name: String,
    /// Backwards-compat: empty for Linux profiles, "11"/"10"/etc. for Windows.
    #[serde(default)]
    pub version: String,
    pub ram: String,
    pub cpu: String,
    pub disk: String,
    /// Windows credentials. For Linux, optional (cloud-init may use them in
    /// Phase 2; for now Linux uses installer-driven account creation).
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub keyboard: String,
    #[serde(default)]
    pub bundles: String,
    /// Overwrite existing profile dir.
    #[serde(default)]
    pub force: bool,
    /// Optional PCI BDF (e.g. "0000:01:00.0") for GPU VFIO passthrough.
    #[serde(default, rename = "gpuBdf", alias = "gpu_bdf")]
    pub gpu_bdf: Option<String>,
    /// "windows" | "linux_distro" | "linux_iso". Defaults to windows for
    /// backwards compat with the legacy CLI/wizard.
    #[serde(default, rename = "imageFamily", alias = "image_family")]
    pub image_family: Option<String>,
    /// For LinuxDistro: BOOT keyword (ubuntu, debian, ...) or full URL.
    #[serde(default)]
    pub boot: Option<String>,
    /// For LinuxIso: absolute path on host to local .iso/.img/.qcow2 file.
    #[serde(default, rename = "isoPath", alias = "iso_path")]
    pub iso_path: Option<String>,
    /// Optional override for where the VM disk image lives. When None
    /// (or empty), falls back to `paths::profile_storage_dir(name)`.
    /// Must be an absolute, writable path with enough free space.
    #[serde(default, rename = "storagePath", alias = "storage_path")]
    pub storage_path: Option<String>,
}

pub fn run(p: InstallParams) -> Result<()> {
    profile::validate_name(&p.name)?;
    if profile::exists(&p.name) && !p.force {
        bail!(
            "Perfil '{}' já existe. Use --force ou escolha outro nome.",
            p.name
        );
    }

    let family = p
        .image_family
        .as_deref()
        .map(ImageFamily::parse)
        .unwrap_or(ImageFamily::Windows);

    validate_family_args(family, &p)?;
    validate_env_fields(family, &p)?;

    docker::check_kvm()?;
    docker::require_installed()?;
    docker::check_daemon()?;

    let (web_port, rdp_port, ssh_port) = ports::allocate()?;
    let ram = validation::normalize_ram(&p.ram)?;
    let cpu = validation::normalize_cpu(&p.cpu)?;
    let disk = validation::normalize_disk(&p.disk)?;
    let bundles = validation::parse_bundle_csv(&p.bundles)?.join(",");
    let gpu_bdf = match p
        .gpu_bdf
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(bdf) => {
            validation::validate_bdf(bdf)?;
            bdf.to_string()
        }
        None => String::new(),
    };
    let mem_limit = format!("{}G", ram.gib + 2);
    let tz = host::tz();

    // Validate the optional custom storage location before touching the
    // filesystem. Empty / None falls back to the default
    // `paths::profile_storage_dir(name)` path further down.
    let storage_override = p
        .storage_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let _storage_check = match storage_override.as_deref() {
        Some(path) => validation::validate_storage_path(path, &disk)?,
        None => None,
    };

    profile::ensure_dirs(&p.name)?;

    let env_path = paths::profile_env_file(&p.name);
    let body = format!(
        "IMAGE_FAMILY={family}\n\
         VERSION={ver}\n\
         BOOT={boot}\n\
         ISO_PATH={iso}\n\
         RAM_SIZE={ram}\n\
         CPU_CORES={cpu}\n\
         DISK_SIZE={disk}\n\
         USERNAME={user}\n\
         PASSWORD={pass}\n\
         LANGUAGE={lang}\n\
         REGION={region}\n\
         KEYBOARD={kb}\n\
         TZ={tz}\n\
         WEB_PORT={web}\n\
         RDP_PORT={rdp}\n\
         SSH_PORT={ssh}\n\
         CONTAINER_NAME={container}\n\
         STORAGE_DIR={storage}\n\
         SHARED_DIR={shared}\n\
         OEM_DIR={oem}\n\
         MEM_LIMIT={mem}\n\
         CPU_LIMIT={cpu}\n\
         BUNDLES={bundles}\n\
         GPU_BDF={gpu_bdf}\n",
        family = family.as_env_value(),
        ver = p.version,
        boot = p.boot.clone().unwrap_or_default(),
        iso = p.iso_path.clone().unwrap_or_default(),
        ram = &ram.env_value,
        cpu = &cpu,
        disk = &disk,
        user = p.user,
        pass = p.password,
        lang = p.language,
        region = p.region,
        kb = p.keyboard,
        tz = tz,
        web = web_port,
        rdp = rdp_port,
        ssh = ssh_port,
        container = paths::profile_container(&p.name),
        storage = storage_override
            .clone()
            .unwrap_or_else(|| paths::profile_storage_dir(&p.name).display().to_string()),
        shared = paths::profile_shared_dir(&p.name).display(),
        oem = paths::profile_oem_dir(&p.name).display(),
        mem = mem_limit,
        bundles = &bundles,
        gpu_bdf = &gpu_bdf,
    );
    std::fs::write(&env_path, body).with_context(|| format!("writing {}", env_path.display()))?;

    // chmod 600 for secrets (password in plaintext)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&env_path, std::fs::Permissions::from_mode(0o600));
    }

    compose::write(&p.name)?;

    // Windows-only: generate install.bat + firstlogon.ps1 from selected
    // bundles. Linux profiles run an unmodified installer for now (cloud-init
    // autoinstall comes in Phase 2).
    if matches!(family, ImageFamily::Windows) {
        oem::generate(&p.name, &bundles)?;
    }

    if profile::get_default().is_none() {
        profile::set_default(&p.name)?;
    }
    let _ = desktop::regenerate();

    // Start the container. Errors are returned so CLI/GUI can surface.
    gpu_hooks::prepare_for_start(&p.name)?;
    docker::compose_run(&p.name, &["up", "-d"])?;
    Ok(())
}

fn validate_family_args(family: ImageFamily, p: &InstallParams) -> Result<()> {
    match family {
        ImageFamily::Windows => {
            if p.version.trim().is_empty() {
                bail!("Família 'windows' exige --version (ex: 11, 10, 2025).");
            }
        }
        ImageFamily::LinuxDistro => {
            let boot = p.boot.as_deref().unwrap_or("").trim();
            if boot.is_empty() {
                bail!("Família 'linux_distro' exige BOOT (ex: ubuntu, fedora, arch).");
            }
        }
        ImageFamily::LinuxIso => {
            let iso = p.iso_path.as_deref().unwrap_or("").trim();
            if iso.is_empty() {
                bail!("Família 'linux_iso' exige iso_path (caminho absoluto da ISO).");
            }
            validation::validate_iso_path(iso)?;
        }
    }
    Ok(())
}

fn validate_env_fields(family: ImageFamily, p: &InstallParams) -> Result<()> {
    for (label, value) in [
        ("Versão", p.version.as_str()),
        ("BOOT", p.boot.as_deref().unwrap_or("")),
        ("ISO", p.iso_path.as_deref().unwrap_or("")),
        ("Usuário", p.user.as_str()),
        ("Idioma", p.language.as_str()),
        ("Região", p.region.as_str()),
        ("Teclado", p.keyboard.as_str()),
        ("Bundles", p.bundles.as_str()),
    ] {
        validation::validate_env_value(label, value)?;
    }
    if matches!(family, ImageFamily::Windows) {
        validation::require_non_empty("Usuário", &p.user)?;
    }
    validation::validate_password(&p.password, matches!(family, ImageFamily::Windows))?;
    Ok(())
}

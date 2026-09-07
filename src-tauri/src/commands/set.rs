use anyhow::{bail, Result};
use serde::Deserialize;

use crate::core::{
    compose, docker, env_file, gpu_hooks, office_state::OfficeProvisioningState, paths, profile,
    validation,
};

#[derive(Debug, Deserialize, Clone)]
pub struct SetParams {
    pub name: String,
    pub ram: Option<String>,
    pub cpu: Option<String>,
    pub disk: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "extraPorts", alias = "extra_ports")]
    pub extra_ports: Option<String>,
    /// Some("") clears GPU; Some("BDF") sets; None leaves untouched.
    #[serde(default, rename = "gpuBdf", alias = "gpu_bdf")]
    pub gpu_bdf: Option<String>,
    /// Immutable Office fields are not editable by `set`; accepting them here
    /// lets the backend reject destructive edits instead of silently ignoring
    /// stale or hostile clients.
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default, rename = "officeLanguage", alias = "office_language")]
    pub office_language: Option<String>,
    #[serde(default)]
    pub restart: bool,
}

pub fn run(p: SetParams) -> Result<String> {
    if !profile::exists(&p.name) {
        bail!("Perfil '{}' não existe.", p.name);
    }
    let env_path = paths::profile_env_file(&p.name);
    guard_office_immutable_update(&p, &env_path)?;
    let mut changed = false;
    let mut msgs: Vec<String> = Vec::new();

    if let Some(v) = p.ram.as_deref().filter(|s| !s.is_empty()) {
        let ram = validation::normalize_ram(v)?;
        env_file::set_key(&env_path, "RAM_SIZE", &ram.env_value)?;
        env_file::set_key(&env_path, "MEM_LIMIT", &format!("{}G", ram.gib + 2))?;
        msgs.push(format!(
            "RAM → {} (mem_limit={}G)",
            ram.env_value,
            ram.gib + 2
        ));
        changed = true;
    }
    if let Some(v) = p.cpu.as_deref().filter(|s| !s.is_empty()) {
        let cpu = validation::normalize_cpu(v)?;
        env_file::set_key(&env_path, "CPU_CORES", &cpu)?;
        env_file::set_key(&env_path, "CPU_LIMIT", &cpu)?;
        msgs.push(format!("CPU → {} cores", cpu));
        changed = true;
    }
    if let Some(v) = p.disk.as_deref().filter(|s| !s.is_empty()) {
        let disk = validation::normalize_disk(v)?;
        env_file::set_key(&env_path, "DISK_SIZE", &disk)?;
        msgs.push(format!(
            "Disco → {} (partição no Windows precisa ser expandida manualmente)",
            disk
        ));
        changed = true;
    }
    if let Some(v) = p.user.as_deref().filter(|s| !s.is_empty()) {
        validation::validate_env_value("Usuário", v)?;
        env_file::set_key(&env_path, "USERNAME", v)?;
        msgs.push(format!("Usuário RDP → {}", v));
        changed = true;
    }
    if let Some(v) = p.password.as_deref().filter(|s| !s.is_empty()) {
        validation::validate_password(v, false)?;
        env_file::set_key(&env_path, "PASSWORD", v)?;
        msgs.push("Senha RDP atualizada".into());
        changed = true;
    }
    // extra_ports: None means "do not touch"; Some("") means clear.
    if let Some(v) = p.extra_ports.as_deref() {
        let ports = validation::parse_extra_ports(v)?;
        let normalized = validation::format_extra_ports(&ports);
        env_file::set_key(&env_path, "EXTRA_PORTS", &normalized)?;
        msgs.push(format!(
            "Port forwarding → {}",
            if normalized.is_empty() {
                "(nenhum)"
            } else {
                &normalized
            }
        ));
        changed = true;
    }
    // gpu_bdf: None means "do not touch"; Some("") means clear; Some("BDF") means set.
    if let Some(v) = p.gpu_bdf.as_deref() {
        if !v.is_empty() {
            validation::validate_bdf(v)?;
        }
        env_file::set_key(&env_path, "GPU_BDF", v)?;
        msgs.push(format!(
            "GPU → {}",
            if v.is_empty() { "(nenhuma)" } else { v }
        ));
        changed = true;
    }

    if !changed {
        return Ok("Nada alterado.".into());
    }

    compose::write(&p.name)?;

    let c = paths::profile_container(&p.name);
    let status = docker::container_status(&c);
    let has_container = status != "absent";

    if has_container {
        if p.restart {
            let _ = docker::compose_run(&p.name, &["down"]);
            gpu_hooks::cleanup_after_stop(&p.name);
            gpu_hooks::prepare_for_start(&p.name)?;
            docker::compose_run(&p.name, &["up", "-d"])?;
            msgs.push(format!("'{}' recriado com as novas configurações.", p.name));
        } else if matches!(status.as_str(), "running" | "paused") {
            msgs.push(format!(
                "VM está '{}' — marque 'Reiniciar' ou rode 'winbox restart {}' para aplicar.",
                status, p.name
            ));
        } else {
            msgs.push(format!(
                "Próximo 'winbox launch {}' aplicará as novas configurações.",
                p.name
            ));
        }
    }

    let _ = c;
    Ok(msgs.join("\n"))
}

fn guard_office_immutable_update(p: &SetParams, env_path: &std::path::Path) -> Result<()> {
    let map = env_file::read(env_path)?;
    let config = validation::office_env_config_from_map(&map);
    let state =
        OfficeProvisioningState::load_or_default(&paths::profile_cfg_dir(&p.name), &p.name)?;
    validation::guard_office_immutable_fields(
        &config,
        validation::OfficeImmutableUpdate {
            version: p.version.as_deref(),
            language: p.language.as_deref(),
            office_language: p.office_language.as_deref(),
        },
        Some(&state),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::office_state::OfficePhase;
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn office_set_refuses_immutable_keys_after_provisioning() {
        let _lock = env_lock().lock().expect("env lock poisoned");
        let _env = TestXdgEnv::new("office-set-refuses-immutable");
        let profile_name = "office-set-immutable";
        let profile_dir = paths::profile_cfg_dir(profile_name);
        std::fs::create_dir_all(&profile_dir).expect("profile dir should be created");
        std::fs::write(
            paths::profile_env_file(profile_name),
            "PROFILE_KIND=office\n\
             VERSION=11\n\
             LANGUAGE=Portuguese\n\
             OFFICE_PRODUCT_ID=O365ProPlusRetail\n\
             OFFICE_LANGUAGE=pt-br\n\
             OFFICE_CHANNEL=Current\n\
             RAM_SIZE=8G\n\
             MEM_LIMIT=10G\n\
             CPU_CORES=2\n\
             DISK_SIZE=64G\n\
             BUNDLES=essentials\n",
        )
        .expect("config.env should be written");
        let mut state = OfficeProvisioningState::new(profile_name);
        state
            .mark_phase_running(OfficePhase::WindowsInstall)
            .expect("windows install should run");
        state
            .mark_phase_done(OfficePhase::WindowsInstall, None)
            .expect("windows install should finish");
        state
            .save_to_dir(&profile_dir)
            .expect("office state should be saved");

        let err = run(SetParams {
            name: profile_name.to_string(),
            ram: Some("16G".to_string()),
            cpu: None,
            disk: None,
            user: None,
            password: None,
            extra_ports: None,
            gpu_bdf: None,
            version: Some("10".to_string()),
            language: None,
            office_language: None,
            restart: false,
        })
        .expect_err("set must reject destructive Office version mutation");

        let msg = format!("{err:#}");
        assert!(msg.contains("VERSION"));
        assert!(msg.contains("reinstalação destrutiva"));
        let env_after =
            std::fs::read_to_string(paths::profile_env_file(profile_name)).expect("env exists");
        assert!(env_after.contains("VERSION=11"));
        assert!(env_after.contains("RAM_SIZE=8G"));
        assert!(!env_after.contains("RAM_SIZE=16G"));
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TestXdgEnv {
        old_config: Option<OsString>,
        old_data: Option<OsString>,
        root: PathBuf,
    }

    impl TestXdgEnv {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("winbox-{name}-{}-{nanos}", std::process::id()));
            let config = root.join("config");
            let data = root.join("data");
            std::fs::create_dir_all(&config).expect("config temp dir");
            std::fs::create_dir_all(&data).expect("data temp dir");
            let old_config = std::env::var_os("XDG_CONFIG_HOME");
            let old_data = std::env::var_os("XDG_DATA_HOME");
            std::env::set_var("XDG_CONFIG_HOME", &config);
            std::env::set_var("XDG_DATA_HOME", &data);
            Self {
                old_config,
                old_data,
                root,
            }
        }
    }

    impl Drop for TestXdgEnv {
        fn drop(&mut self) {
            match &self.old_config {
                Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
            match &self.old_data {
                Some(value) => std::env::set_var("XDG_DATA_HOME", value),
                None => std::env::remove_var("XDG_DATA_HOME"),
            }
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

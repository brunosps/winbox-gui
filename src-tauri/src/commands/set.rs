use anyhow::{bail, Result};
use serde::Deserialize;

use crate::core::{compose, docker, env_file, gpu_hooks, paths, profile, validation};

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
    #[serde(default)]
    pub restart: bool,
}

pub fn run(p: SetParams) -> Result<String> {
    if !profile::exists(&p.name) {
        bail!("Perfil '{}' não existe.", p.name);
    }
    let env_path = paths::profile_env_file(&p.name);
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

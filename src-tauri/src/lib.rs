pub mod cli;
pub mod commands;
pub mod core;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::core::connect::ConnectMode;
use crate::core::image_family::{ImageFamily, SUPPORTED_DISTROS};
use crate::core::{
    bundles as core_bundles, connect, env_file, gpu as core_gpu, health, host, paths, profile,
    snapshots, vfio_setup,
};

// ─── Structs expostas ao front ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OperationResult {
    pub profile: String,
    pub op: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationProgress {
    pub profile: String,
    pub op: String,
    pub step: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Profile {
    pub name: String,
    pub status: String,
    pub web_port: u16,
    pub rdp_port: u16,
    pub ssh_port: u16,
    pub ram: String,
    pub bundles: String,
    pub shared_dir: String,
    pub is_default: bool,
    pub image_family: ImageFamily,
    pub connect_mode: ConnectMode,
    /// For Linux distro profiles, the BOOT keyword (ubuntu, fedora, …).
    /// Empty for Windows or LinuxIso profiles.
    pub boot: String,
    /// For LinuxCloud profiles, the cloud-init bootstrap profile.
    pub cloud_init_profile: String,
    /// For Linux ISO profiles, absolute host path of the mounted ISO.
    pub iso_path: String,
}

#[derive(Debug, Serialize)]
pub struct ProfileConfig {
    pub name: String,
    pub image_family: ImageFamily,
    pub version: String,
    pub boot: String,
    pub cloud_init_profile: String,
    pub iso_path: String,
    pub ram: String,
    pub cpu: String,
    pub disk: String,
    pub user: String,
    pub password: String,
    pub language: String,
    pub region: String,
    pub keyboard: String,
    pub web_port: u16,
    pub rdp_port: u16,
    pub ssh_port: u16,
    pub bundles: String,
    pub shared_dir: String,
    pub storage_dir: String,
    pub extra_ports: String,
    pub gpu_bdf: String,
}

#[derive(Debug, Deserialize)]
pub struct InstallArgs {
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub ram: String,
    pub cpu: String,
    pub disk: String,
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
    #[serde(default, rename = "gpuBdf", alias = "gpu_bdf")]
    pub gpu_bdf: Option<String>,
    #[serde(default, rename = "imageFamily", alias = "image_family")]
    pub image_family: Option<String>,
    #[serde(default)]
    pub boot: Option<String>,
    #[serde(default, rename = "isoPath", alias = "iso_path")]
    pub iso_path: Option<String>,
    #[serde(default, rename = "storagePath", alias = "storage_path")]
    pub storage_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DistroOption {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArgs {
    pub name: String,
    pub ram: Option<String>,
    pub cpu: Option<String>,
    pub disk: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "extraPorts", alias = "extra_ports")]
    pub extra_ports: Option<String>,
    #[serde(default, rename = "gpuBdf", alias = "gpu_bdf")]
    pub gpu_bdf: Option<String>,
    pub restart: bool,
}

async fn run_blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))
}

fn emit_progress(
    app: &AppHandle,
    profile: &str,
    op: &str,
    step: &str,
    status: &str,
    message: impl Into<String>,
) {
    let _ = app.emit(
        "operation-progress",
        OperationProgress {
            profile: profile.into(),
            op: op.into(),
            step: step.into(),
            status: status.into(),
            message: message.into(),
        },
    );
}

fn actionable_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let hint = if lower.contains("docker não encontrado") || lower.contains("docker not found") {
        Some("Instale o Docker Engine e reabra o app.")
    } else if lower.contains("daemon") || lower.contains("docker info") {
        Some("Inicie o serviço Docker e confira permissões do usuário.")
    } else if lower.contains("/dev/kvm") || lower.contains("kvm") {
        Some("Habilite virtualização/KVM e adicione seu usuário ao grupo kvm.")
    } else if lower.contains("address already in use")
        || lower.contains("porta")
        || lower.contains("port")
    {
        Some("Verifique conflitos no Host Health ou altere as portas do perfil.")
    } else if lower.contains("vfio") || lower.contains("iommu") {
        Some("Confira o status GPU/VFIO no Host Health antes de iniciar.")
    } else if lower.contains("timeout") {
        Some("Abra os logs do perfil para ver onde o boot parou.")
    } else {
        None
    };

    match hint {
        Some(hint) if !raw.contains("Próxima ação:") => format!("{raw}\nPróxima ação: {hint}"),
        _ => raw.to_string(),
    }
}

fn emit_operation_result(
    app: &AppHandle,
    profile: &str,
    op: &str,
    result: Result<OperationResult, String>,
) -> Result<OperationResult, String> {
    match result {
        Ok(res) => {
            emit_progress(
                app,
                &res.profile,
                &res.op,
                "complete",
                "success",
                &res.message,
            );
            Ok(res)
        }
        Err(err) => {
            let msg = actionable_error(&err);
            emit_progress(app, profile, op, "failed", "error", &msg);
            Err(msg)
        }
    }
}

// ─── Tauri commands ────────────────────────────────────────────────────

#[tauri::command]
fn list_profiles() -> Result<Vec<Profile>, String> {
    Ok(commands::list::list()
        .into_iter()
        .map(|p| Profile {
            name: p.name,
            status: p.status,
            web_port: p.web_port,
            rdp_port: p.rdp_port,
            ssh_port: p.ssh_port,
            ram: p.ram,
            bundles: p.bundles,
            shared_dir: p.shared_dir,
            is_default: p.is_default,
            image_family: p.image_family,
            connect_mode: p.connect_mode,
            boot: p.boot,
            cloud_init_profile: p.cloud_init_profile,
            iso_path: p.iso_path,
        })
        .collect())
}

#[tauri::command]
fn list_supported_distros() -> Vec<DistroOption> {
    SUPPORTED_DISTROS
        .iter()
        .map(|(id, label)| DistroOption { id, label })
        .collect()
}

#[tauri::command]
fn host_info() -> Result<host::HostInfo, String> {
    Ok(host::info())
}

#[tauri::command]
fn host_health() -> Result<health::HostHealthReport, String> {
    Ok(health::report())
}

/// First-run backend detection: reports whether WSL2 / distro / Docker /
/// dockurr image are present. Read-only — never triggers installs.
#[tauri::command]
fn bootstrap_status() -> Result<crate::core::bootstrap::BootstrapStatus, String> {
    Ok(crate::core::bootstrap::check_status())
}

/// Run a single bootstrap step (user-initiated). For distro creation,
/// pass `force = true` only after the user confirmed overwriting an
/// existing distro — otherwise it returns `needs_confirmation`.
#[tauri::command]
async fn bootstrap_run_step(
    app: AppHandle,
    step: crate::core::bootstrap::BootstrapStep,
    force: bool,
) -> Result<crate::core::bootstrap::StepOutcome, String> {
    let plan = crate::core::bootstrap::step_plan(step);
    emit_progress(
        &app,
        "bootstrap",
        "bootstrap",
        "running",
        "running",
        &plan.description,
    );
    let outcome =
        tauri::async_runtime::spawn_blocking(move || crate::core::bootstrap::run_step(step, force))
            .await
            .map_err(|e| e.to_string())?;
    Ok(outcome)
}

#[tauri::command]
fn list_bundles() -> Result<Vec<core_bundles::Bundle>, String> {
    Ok(core_bundles::list())
}

#[tauri::command]
async fn launch_profile(
    app: AppHandle,
    name: String,
) -> Result<OperationResult, crate::core::launch_error::LaunchError> {
    use crate::core::docker::CliDocker;
    use crate::core::launch_error::LaunchError;
    let p = profile::resolve(Some(&name)).map_err(|e| LaunchError::Other {
        message: e.to_string(),
    })?;
    let mode = connect::resolve_for_profile(&p);
    let profile_name = p.clone();
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "launch",
        "prepare",
        "running",
        "Preparando conexão do perfil.",
    );
    let progress_app = app.clone();
    let progress_profile = profile_name.clone();
    let emit = move |step: &str, message: &str| {
        emit_progress(
            &progress_app,
            &progress_profile,
            "launch",
            step,
            "running",
            message,
        );
    };
    let _ = mode; // Reserved for future per-profile overrides; today everything goes web.
    let join =
        tauri::async_runtime::spawn_blocking(move || -> Result<OperationResult, LaunchError> {
            use crate::core::docker;
            use commands::launch as cmd_launch;
            let docker_cli = CliDocker;
            emit("preflight", "Verificando KVM e Docker...");
            docker::preflight_kvm()?;
            docker::preflight_docker_installed()?;
            docker::preflight_docker_daemon()?;

            emit("starting", "Subindo container Docker...");
            let transition = cmd_launch::ensure_started(&profile_name, &docker_cli)?;
            let needs_wait = matches!(transition, cmd_launch::ContainerTransition::Recreated);

            let env = crate::core::env_file::read(&paths::profile_env_file(&profile_name))
                .map_err(|e| LaunchError::Other {
                    message: format!("falha lendo env: {e:#}"),
                })?;
            let viewer_port = connect::viewer_port(&env);
            let viewer_url = connect::viewer_url(&env).ok_or_else(|| LaunchError::Other {
                message: format!("WEB_PORT não definido para '{profile_name}'"),
            })?;

            if needs_wait || !cmd_launch::web_port_reachable(viewer_port) {
                emit("wait_vnc", "Aguardando QEMU + noVNC responder...");
                cmd_launch::wait_for_web_port(&profile_name, viewer_port)?;
            }

            emit("launching", "Abrindo viewer noVNC no navegador...");
            open_web_vnc_url(&profile_name, &viewer_url).map_err(|e| LaunchError::Other {
                message: format!("{e:#}"),
            })?;

            Ok(OperationResult {
                profile: profile_name,
                op: "launch".into(),
                message: "Perfil iniciado.".into(),
            })
        })
        .await;
    let result = match join {
        Ok(inner) => inner,
        Err(e) => Err(LaunchError::Other {
            message: e.to_string(),
        }),
    };
    match &result {
        Ok(res) => {
            emit_progress(
                &app,
                &res.profile,
                &res.op,
                "complete",
                "success",
                &res.message,
            );
        }
        Err(err) => {
            emit_progress(
                &app,
                &profile_for_error,
                "launch",
                "failed",
                "error",
                err.to_string(),
            );
        }
    }
    result
}

fn open_web_vnc_url(profile_name: &str, url: &str) -> anyhow::Result<()> {
    if url.trim().is_empty() {
        anyhow::bail!("WEB_PORT inválida para perfil '{}'", profile_name);
    }
    // We previously launched noVNC inside a Tauri WebView, but WebKitGTK 4.1
    // does not deliver keyboard/mouse events to the noVNC canvas reliably
    // (the frame renders fine, input is dropped at the WM layer). Use the
    // user's default browser instead — it gets clipboard, full-screen, and
    // input forwarding for free, which matches the legacy CLI behavior.
    crate::core::opener::open_url(url)
        .map_err(|e| anyhow::anyhow!("falha ao abrir noVNC ({url}): {e:#}"))?;
    Ok(())
}

#[tauri::command]
async fn pick_iso_file(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter(
            "Disk image",
            &["iso", "img", "qcow2", "vhd", "vhdx", "vdi", "vmdk", "raw"],
        )
        .pick_file(move |file| {
            let _ = tx.send(file.and_then(|f| f.into_path().ok()));
        });
    let path = rx.recv().map_err(|e| e.to_string())?;
    Ok(path.map(|p| p.display().to_string()))
}

/// Open a folder picker so the user can pick where the VM disk image
/// will live for the profile being created. Mirrors `pick_iso_file`.
#[tauri::command]
async fn pick_storage_dir(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |dir| {
        let _ = tx.send(dir.and_then(|d| d.into_path().ok()));
    });
    let path = rx.recv().map_err(|e| e.to_string())?;
    Ok(path.map(|p| p.display().to_string()))
}

#[tauri::command]
fn open_web_vnc(app: AppHandle, name: String) -> Result<(), String> {
    let p = profile::resolve(Some(&name)).map_err(|e| e.to_string())?;
    let map = env_file::read(&paths::profile_env_file(&p)).map_err(|e| e.to_string())?;
    let url =
        connect::viewer_url(&map).ok_or_else(|| format!("WEB_PORT não definido para '{p}'"))?;
    let _ = app;
    open_web_vnc_url(&p, &url).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn stop_profile(app: AppHandle, name: String) -> Result<String, String> {
    // docker stop -t 120 — Windows graceful shutdown can take ~minutes. Run
    // on a blocking thread and emit progress so the UI shows a spinner
    // instead of locking up the Tauri runtime.
    emit_progress(
        &app,
        &name,
        "stop",
        "stopping",
        "running",
        "Desligando container (pode levar até 2min)...",
    );
    let n = name.clone();
    let join = tauri::async_runtime::spawn_blocking(move || commands::lifecycle::stop(&n)).await;
    let res = match join {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let step = if res.is_ok() { "complete" } else { "failed" };
    let status = if res.is_ok() { "success" } else { "error" };
    let msg = res
        .as_ref()
        .map(|_| "Container desligado.".into())
        .unwrap_or_else(|e| e.clone());
    emit_progress(&app, &name, "stop", step, status, &msg);
    res.map(|_| format!("stopped {}", name))
}

#[tauri::command]
async fn kill_profile(app: AppHandle, name: String) -> Result<String, String> {
    emit_progress(
        &app,
        &name,
        "kill",
        "killing",
        "running",
        "Forçando parada do container...",
    );
    let n = name.clone();
    let join = tauri::async_runtime::spawn_blocking(move || commands::lifecycle::kill(&n)).await;
    let res = match join {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let step = if res.is_ok() { "complete" } else { "failed" };
    let status = if res.is_ok() { "success" } else { "error" };
    let msg = res
        .as_ref()
        .map(|_| "Container forçado a parar.".into())
        .unwrap_or_else(|e| e.clone());
    emit_progress(&app, &name, "kill", step, status, &msg);
    res.map(|_| format!("killed {}", name))
}

#[tauri::command]
async fn pause_profile(app: AppHandle, name: String) -> Result<String, String> {
    emit_progress(
        &app,
        &name,
        "pause",
        "pausing",
        "running",
        "Pausando container...",
    );
    let n = name.clone();
    let join = tauri::async_runtime::spawn_blocking(move || commands::lifecycle::pause(&n)).await;
    let res = match join {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let step = if res.is_ok() { "complete" } else { "failed" };
    let status = if res.is_ok() { "success" } else { "error" };
    let msg = res
        .as_ref()
        .map(|_| "Container pausado.".into())
        .unwrap_or_else(|e| e.clone());
    emit_progress(&app, &name, "pause", step, status, &msg);
    res.map(|_| format!("paused {}", name))
}

#[tauri::command]
async fn resume_profile(app: AppHandle, name: String) -> Result<String, String> {
    emit_progress(
        &app,
        &name,
        "resume",
        "resuming",
        "running",
        "Retomando container...",
    );
    let n = name.clone();
    let join = tauri::async_runtime::spawn_blocking(move || commands::lifecycle::resume(&n)).await;
    let res = match join {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let step = if res.is_ok() { "complete" } else { "failed" };
    let status = if res.is_ok() { "success" } else { "error" };
    let msg = res
        .as_ref()
        .map(|_| "Container retomado.".into())
        .unwrap_or_else(|e| e.clone());
    emit_progress(&app, &name, "resume", step, status, &msg);
    res.map(|_| format!("resumed {}", name))
}

#[tauri::command]
fn set_default_profile(name: String) -> Result<String, String> {
    commands::lifecycle::set_default(&name).map_err(|e| e.to_string())?;
    Ok(format!("default → {}", name))
}

#[tauri::command]
async fn install_profile(app: AppHandle, params: InstallArgs) -> Result<OperationResult, String> {
    let cmd_params = commands::install::InstallParams {
        name: params.name.clone(),
        version: params.version,
        ram: params.ram,
        cpu: params.cpu,
        disk: params.disk,
        user: params.user,
        password: params.password,
        language: params.language,
        region: params.region,
        keyboard: params.keyboard,
        bundles: params.bundles,
        force: false,
        gpu_bdf: params.gpu_bdf,
        image_family: params.image_family,
        boot: params.boot,
        iso_path: params.iso_path,
        storage_path: params.storage_path,
    };
    let profile_name = params.name;
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "install",
        "create",
        "running",
        "Criando perfil e preparando a VM.",
    );
    let result = run_blocking(move || {
        commands::install::run(cmd_params)?;
        Ok(OperationResult {
            profile: profile_name,
            op: "install".into(),
            message: "Perfil criado e iniciado.".into(),
        })
    })
    .await;
    emit_operation_result(&app, &profile_for_error, "install", result)
}

#[tauri::command]
async fn remove_profile(app: AppHandle, name: String) -> Result<String, String> {
    emit_progress(
        &app,
        &name,
        "remove",
        "removing",
        "running",
        "Removendo container e arquivos do perfil...",
    );
    let n = name.clone();
    let join =
        tauri::async_runtime::spawn_blocking(move || commands::lifecycle::remove(&n, true)).await;
    let res = match join {
        Ok(inner) => inner.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let step = if res.is_ok() { "complete" } else { "failed" };
    let status = if res.is_ok() { "success" } else { "error" };
    let msg = res
        .as_ref()
        .map(|_| "Perfil removido.".into())
        .unwrap_or_else(|e| e.clone());
    emit_progress(&app, &name, "remove", step, status, &msg);
    res.map(|_| format!("removed {}", name))
}

#[tauri::command]
fn get_logs(name: String, tail: Option<usize>) -> Result<String, String> {
    commands::logs::tail(&name, tail.unwrap_or(200)).map_err(|e| e.to_string())
}

#[tauri::command]
fn version() -> Result<String, String> {
    Ok(format!("winbox {}", paths::WINBOX_VERSION))
}

#[tauri::command]
fn get_profile_config(name: String) -> Result<ProfileConfig, String> {
    let path = paths::profile_env_file(&name);
    let map = env_file::read(&path).map_err(|e| e.to_string())?;
    Ok(ProfileConfig {
        name,
        image_family: ImageFamily::from_env_map(&map),
        version: env_file::get(&map, "VERSION").to_string(),
        boot: env_file::get(&map, "BOOT").to_string(),
        cloud_init_profile: env_file::get(&map, "CLOUD_INIT_PROFILE").to_string(),
        iso_path: env_file::get(&map, "ISO_PATH").to_string(),
        ram: env_file::get(&map, "RAM_SIZE").to_string(),
        cpu: env_file::get(&map, "CPU_CORES").to_string(),
        disk: env_file::get(&map, "DISK_SIZE").to_string(),
        user: env_file::get(&map, "USERNAME").to_string(),
        password: String::new(),
        language: env_file::get(&map, "LANGUAGE").to_string(),
        region: env_file::get(&map, "REGION").to_string(),
        keyboard: env_file::get(&map, "KEYBOARD").to_string(),
        web_port: env_file::get_u16(&map, "WEB_PORT"),
        rdp_port: env_file::get_u16(&map, "RDP_PORT"),
        ssh_port: env_file::get_u16(&map, "SSH_PORT"),
        bundles: env_file::get(&map, "BUNDLES").to_string(),
        shared_dir: env_file::get(&map, "SHARED_DIR").to_string(),
        storage_dir: env_file::get(&map, "STORAGE_DIR").to_string(),
        extra_ports: env_file::get(&map, "EXTRA_PORTS").to_string(),
        gpu_bdf: env_file::get(&map, "GPU_BDF").to_string(),
    })
}

#[tauri::command]
async fn update_profile(app: AppHandle, params: UpdateArgs) -> Result<OperationResult, String> {
    let p = commands::set::SetParams {
        name: params.name.clone(),
        ram: params.ram,
        cpu: params.cpu,
        disk: params.disk,
        user: params.user,
        password: params.password,
        extra_ports: params.extra_ports,
        gpu_bdf: params.gpu_bdf,
        restart: params.restart,
    };
    let profile_name = params.name;
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "update",
        "apply",
        "running",
        "Aplicando configuração do perfil.",
    );
    let result = run_blocking(move || {
        let message = commands::set::run(p)?;
        Ok(OperationResult {
            profile: profile_name,
            op: "update".into(),
            message,
        })
    })
    .await;
    emit_operation_result(&app, &profile_for_error, "update", result)
}

#[tauri::command]
fn list_host_gpus() -> Result<Vec<core_gpu::GpuInfo>, String> {
    Ok(core_gpu::list())
}

#[tauri::command]
fn gpu_setup_status(bdf: String) -> Result<vfio_setup::SetupStatus, String> {
    Ok(vfio_setup::status(&bdf))
}

#[tauri::command]
fn gpu_setup_apply(bdf: String) -> Result<vfio_setup::SetupStatus, String> {
    vfio_setup::apply(&bdf).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn gpu_setup_revert(bdf: String) -> Result<vfio_setup::SetupStatus, String> {
    vfio_setup::revert(&bdf).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn restart_profile(app: AppHandle, name: String) -> Result<OperationResult, String> {
    let profile_name = name.clone();
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "restart",
        "recreate",
        "running",
        "Recriando container do perfil.",
    );
    let result = run_blocking(move || {
        commands::lifecycle::restart(&profile_name)?;
        Ok(OperationResult {
            profile: profile_name,
            op: "restart".into(),
            message: "Perfil reiniciado.".into(),
        })
    })
    .await;
    emit_operation_result(&app, &profile_for_error, "restart", result)
}

#[derive(Debug, Serialize)]
pub struct ReapplyResult {
    pub path: String,
    pub windows_path: String,
}

#[tauri::command]
fn reapply_bundles(name: String) -> Result<ReapplyResult, String> {
    let path = commands::reapply::run(&name).map_err(|e| e.to_string())?;
    Ok(ReapplyResult {
        path: path.display().to_string(),
        windows_path: "\\\\host.lan\\Data\\winbox-reapply.ps1".into(),
    })
}

#[tauri::command]
fn list_snapshots(name: String) -> Result<Vec<snapshots::SnapshotInfo>, String> {
    let p = profile::resolve(Some(&name)).map_err(|e| e.to_string())?;
    Ok(snapshots::list(&p))
}

#[tauri::command]
async fn create_snapshot(
    app: AppHandle,
    name: String,
    snapshot: String,
) -> Result<OperationResult, String> {
    let profile_name = profile::resolve(Some(&name)).map_err(|e| e.to_string())?;
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "snapshot",
        "create",
        "running",
        format!("Criando snapshot '{snapshot}'."),
    );
    let result = run_blocking(move || {
        snapshots::create(&profile_name, &snapshot)?;
        Ok(OperationResult {
            profile: profile_name,
            op: "snapshot".into(),
            message: format!("Snapshot '{snapshot}' criado."),
        })
    })
    .await;
    emit_operation_result(&app, &profile_for_error, "snapshot", result)
}

#[tauri::command]
async fn rollback_snapshot(
    app: AppHandle,
    name: String,
    snapshot: String,
) -> Result<OperationResult, String> {
    let profile_name = profile::resolve(Some(&name)).map_err(|e| e.to_string())?;
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "rollback",
        "restore",
        "running",
        format!("Restaurando snapshot '{snapshot}'."),
    );
    let result = run_blocking(move || {
        snapshots::rollback(&profile_name, &snapshot)?;
        Ok(OperationResult {
            profile: profile_name,
            op: "rollback".into(),
            message: format!("Snapshot '{snapshot}' restaurado."),
        })
    })
    .await;
    emit_operation_result(&app, &profile_for_error, "rollback", result)
}

#[tauri::command]
async fn update_profile_image(app: AppHandle, name: String) -> Result<OperationResult, String> {
    let profile_name = profile::resolve(Some(&name)).map_err(|e| e.to_string())?;
    let profile_for_error = profile_name.clone();
    emit_progress(
        &app,
        &profile_name,
        "update-image",
        "pull",
        "running",
        "Atualizando imagem e recriando perfil.",
    );
    let result = run_blocking(move || {
        commands::lifecycle::update(&profile_name)?;
        Ok(OperationResult {
            profile: profile_name,
            op: "update-image".into(),
            message: "Imagem atualizada e perfil recriado.".into(),
        })
    })
    .await;
    emit_operation_result(&app, &profile_for_error, "update-image", result)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            launch_profile,
            open_web_vnc,
            pick_iso_file,
            pick_storage_dir,
            stop_profile,
            kill_profile,
            pause_profile,
            resume_profile,
            set_default_profile,
            install_profile,
            remove_profile,
            get_logs,
            list_bundles,
            list_supported_distros,
            host_info,
            host_health,
            bootstrap_status,
            bootstrap_run_step,
            version,
            get_profile_config,
            update_profile,
            restart_profile,
            reapply_bundles,
            list_snapshots,
            create_snapshot,
            rollback_snapshot,
            update_profile_image,
            list_host_gpus,
            gpu_setup_status,
            gpu_setup_apply,
            gpu_setup_revert,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::actionable_error;

    #[test]
    fn actionable_error_adds_known_hint_once() {
        let msg = actionable_error("docker info falhou");
        assert!(msg.contains("Próxima ação:"));
        assert_eq!(msg.matches("Próxima ação:").count(), 1);
        assert_eq!(actionable_error(&msg).matches("Próxima ação:").count(), 1);
    }
}

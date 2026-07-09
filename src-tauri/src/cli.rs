use anyhow::{anyhow, bail, Result};
use clap::{Args, Parser, Subcommand};
use dialoguer::{Confirm, Input, Password};
use serde::Serialize;
use serde_json::{json, Value};

use crate::commands::{
    install as cmd_install, launch as cmd_launch, lifecycle, logs, set as cmd_set,
};
use crate::core::{bundles, connect, env_file, host, paths, profile, snapshots};

#[path = "cli_office.rs"]
mod cli_office;

#[derive(Parser, Debug)]
#[command(name = "winbox", version = paths::WINBOX_VERSION, about = "Gerenciador de VMs Windows (dockur/windows)", disable_help_subcommand = true)]
struct Cli {
    /// Emit machine-readable JSON. Human output remains the default.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Cmd {
    /// Mostra versão e metadata do provider
    Version,

    /// Lista distros suportadas para perfis Linux
    Distros,

    /// Diagnóstico operacional do host
    #[command(name = "host-health")]
    HostHealth,

    /// Dados de um perfil
    Profile { profile: String },

    /// URL do viewer noVNC para um perfil
    #[command(name = "viewer-url")]
    ViewerUrl { profile: String },

    /// Cria um novo perfil e baixa Windows + aplica bundles
    Install(InstallArgs),

    /// Sobe a VM (se necessário) e abre o RDP
    #[command(alias = "open")]
    Launch(LaunchArgs),

    /// Sobe a VM em background (sem RDP)
    #[command(alias = "up")]
    Start {
        profile: Option<String>,
        #[arg(long)]
        progress: Option<String>,
    },

    /// docker stop com ACPI
    Stop { profile: Option<String> },

    /// Hard stop (corrompe filesystem)
    Kill { profile: Option<String> },

    /// Pausa container (RAM mantida)
    Pause { profile: Option<String> },

    /// Retoma do pause
    #[command(alias = "unpause")]
    Resume { profile: Option<String> },

    /// Estado detalhado do container
    #[command(alias = "ps")]
    Status { profile: Option<String> },

    /// Lista perfis + status
    #[command(alias = "ls")]
    List,

    /// Mostra/define o perfil padrão
    Default { profile: Option<String> },

    /// Logs do container
    Logs {
        profile: Option<String>,
        /// Linhas a exibir (default: tail -f / 200 linhas)
        #[arg(long)]
        tail: Option<usize>,
    },

    /// Snapshot do disco
    Snapshot { profile: String, name: String },

    /// Lista snapshots
    #[command(alias = "snaps")]
    Snapshots { profile: Option<String> },

    /// Restaura snapshot
    Rollback {
        profile: String,
        name: String,
        #[arg(long)]
        yes: bool,
    },

    /// Snapshot + pull + recreate
    #[command(alias = "upgrade")]
    Update { profile: Option<String> },

    /// Edita configuração (ram/cpu/disk/user/pass/portas)
    #[command(alias = "edit", alias = "config")]
    Set(SetArgs),

    /// Recria o container
    #[command(alias = "recreate")]
    Restart { profile: Option<String> },

    /// Remove perfil e dados
    #[command(alias = "rm", alias = "uninstall")]
    Remove {
        profile: String,
        #[arg(long, short = 'y')]
        yes: bool,
        #[arg(long = "delete-storage", default_value_t = true)]
        delete_storage: bool,
    },

    /// Bundles (list, show)
    #[command(subcommand, alias = "bundle")]
    Bundles(BundlesCmd),

    /// Lista GPUs do host com info de VFIO
    #[command(alias = "gpus")]
    Gpu,

    /// Configura o host para passthrough da GPU (escreve modprobe + initramfs).
    /// Após rodar, é necessário reiniciar o sistema.
    #[command(name = "gpu-setup")]
    GpuSetup {
        /// PCI BDF da GPU, ex: 0000:01:00.0
        #[arg(long)]
        bdf: String,
    },

    /// Remove a configuração de host (modprobe drop-in) para uma GPU.
    /// Após rodar, é necessário reiniciar.
    #[command(name = "gpu-revert")]
    GpuRevert {
        #[arg(long)]
        bdf: String,
    },

    /// Provisiona, consulta e lança apps do perfil Office.
    Office(cli_office::OfficeArgs),

    /// Abre a GUI
    Gui,
}

#[derive(Args, Debug)]
struct InstallArgs {
    profile: Option<String>,
    #[arg(long = "bundle", alias = "bundles")]
    bundle: Option<String>,
    #[arg(long)]
    force: bool,
    #[arg(long, short = 'y')]
    yes: bool,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    ram: Option<String>,
    #[arg(long)]
    cpu: Option<String>,
    #[arg(long)]
    disk: Option<String>,
    #[arg(long)]
    user: Option<String>,
    #[arg(long = "pass", alias = "password")]
    pass: Option<String>,
    #[arg(long = "language", alias = "lang")]
    language: Option<String>,
    #[arg(long)]
    region: Option<String>,
    #[arg(long = "keyboard", alias = "kbd")]
    keyboard: Option<String>,
    /// PCI BDF of a GPU to pass through via VFIO, e.g. 0000:01:00.0.
    /// You still need to run `winbox gpu-setup --bdf <BDF>` once and reboot.
    #[arg(long)]
    gpu: Option<String>,
    /// Image family for non-interactive install: windows | linux_distro | linux_iso | linux_cloud.
    #[arg(long = "family", alias = "image-family", alias = "image_family")]
    family: Option<String>,
    /// BOOT keyword for linux_distro, or cloud-init profile for linux_cloud
    /// (server | xubuntu-desktop).
    #[arg(long)]
    boot: Option<String>,
    /// Absolute ISO path for linux_iso profiles.
    #[arg(long = "iso-path", alias = "iso_path")]
    iso_path: Option<String>,
    /// Optional custom storage path.
    #[arg(long = "storage-path", alias = "storage_path")]
    storage_path: Option<String>,
    /// Progress format for machine consumers: jsonl.
    #[arg(long)]
    progress: Option<String>,
}

#[derive(Args, Debug)]
struct LaunchArgs {
    profile: Option<String>,
    #[arg(long = "on-close", default_value = "keep")]
    on_close: String,
    #[arg(long = "keep-alive", short = 'k')]
    keep_alive: bool,
    #[arg(long)]
    progress: Option<String>,
}

#[derive(Args, Debug)]
struct SetArgs {
    profile: String,
    #[arg(long)]
    ram: Option<String>,
    #[arg(long)]
    cpu: Option<String>,
    #[arg(long)]
    disk: Option<String>,
    #[arg(long)]
    user: Option<String>,
    #[arg(long = "pass", alias = "password")]
    pass: Option<String>,
    #[arg(long = "extra-ports")]
    extra_ports: Option<String>,
    /// GPU BDF to pass through; pass empty string to clear.
    #[arg(long)]
    gpu: Option<String>,
    #[arg(long)]
    restart: bool,
}

#[derive(Subcommand, Debug)]
enum BundlesCmd {
    /// Lista bundles disponíveis
    #[command(alias = "ls")]
    List,
    /// Mostra o conteúdo de um bundle
    #[command(alias = "cat")]
    Show { name: String },
    /// Regenera o firstlogon.ps1 e copia pro SHARED_DIR como winbox-reapply.ps1
    Reapply { profile: Option<String> },
}

/// Run the CLI. Returns exit code.
pub fn run() -> i32 {
    match Cli::try_parse() {
        Ok(cli) => {
            let mode = cli_office::OutputMode::from_json(cli.json);
            match cli.cmd {
                Cmd::Office(args) => cli_office::run_office_cli(args, mode),
                cmd if cli.json => match dispatch_json(cmd) {
                    Ok(value) => {
                        print_json_success(value);
                        0
                    }
                    Err(e) => {
                        print_json_error(&e);
                        1
                    }
                },
                cmd => match dispatch(cmd) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("✗ {}", e);
                        1
                    }
                },
            }
        }
        Err(err) => {
            // clap already prints help/error to stderr/stdout
            err.print().ok();
            if err.kind() == clap::error::ErrorKind::DisplayHelp
                || err.kind() == clap::error::ErrorKind::DisplayVersion
            {
                0
            } else {
                2
            }
        }
    }
}

fn dispatch(cmd: Cmd) -> Result<()> {
    match cmd {
        Cmd::Version => {
            println!("{}", paths::WINBOX_VERSION);
            Ok(())
        }
        Cmd::Distros => {
            for (id, label) in crate::core::image_family::SUPPORTED_DISTROS {
                println!("{id}\t{label}");
            }
            Ok(())
        }
        Cmd::HostHealth => {
            let report = crate::core::health::report();
            println!("{:?}", report.overall_status);
            for check in report.checks {
                println!("{}\t{:?}\t{}", check.id, check.status, check.message);
            }
            Ok(())
        }
        Cmd::Profile { profile } => {
            let summary = profile_summary(&profile)?;
            println!("{}\t{}", summary.name, summary.status);
            Ok(())
        }
        Cmd::ViewerUrl { profile } => {
            println!("{}", viewer_url(&profile)?);
            Ok(())
        }
        Cmd::Install(a) => cmd_install_interactive(a),
        Cmd::Launch(a) => {
            let p = profile::resolve(a.profile.as_deref())?;
            let on_close = if a.keep_alive {
                cmd_launch::OnClose::Keep
            } else {
                a.on_close.parse()?
            };
            cmd_launch::launch(&p, on_close)
        }
        Cmd::Start { profile, .. } => {
            let p = profile::resolve(profile.as_deref())?;
            let docker = crate::core::docker::CliDocker;
            cmd_launch::start(&p, &docker).map_err(|e| anyhow::anyhow!("{e}"))
        }
        Cmd::Stop { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::stop(&p)
        }
        Cmd::Kill { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::kill(&p)
        }
        Cmd::Pause { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::pause(&p)
        }
        Cmd::Resume { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::resume(&p)
        }
        Cmd::Status { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            let c = paths::profile_container(&p);
            let status = crate::core::docker::container_status(&c);
            println!("{}\t{}", p, status);
            Ok(())
        }
        Cmd::List => {
            let profs = crate::commands::list::list();
            if profs.is_empty() {
                println!("Nenhum perfil. Rode: winbox install <nome>");
                return Ok(());
            }
            println!(
                "{:<20} {:<10} {:<10} {:<10} {:<10} BUNDLES",
                "PERFIL", "STATUS", "WEB", "RDP", "RAM"
            );
            for p in profs {
                let mark = if p.is_default { "*" } else { "" };
                println!(
                    "{:<20} {:<10} {:<10} {:<10} {:<10} {}",
                    format!("{}{}", p.name, mark),
                    p.status,
                    p.web_port,
                    p.rdp_port,
                    p.ram,
                    p.bundles
                );
            }
            Ok(())
        }
        Cmd::Default { profile } => {
            match profile {
                None => {
                    if let Some(d) = profile::get_default() {
                        println!("{d}");
                    } else {
                        println!("(nenhum padrão definido)");
                    }
                }
                Some(name) => lifecycle::set_default(&name)?,
            }
            Ok(())
        }
        Cmd::Logs { profile, tail } => {
            let p = profile::resolve(profile.as_deref())?;
            let out = logs::tail(&p, tail.unwrap_or(200))?;
            print!("{out}");
            Ok(())
        }
        Cmd::Snapshot { profile, name } => {
            let p = profile::resolve(Some(&profile))?;
            snapshots::create(&p, &name)?;
            Ok(())
        }
        Cmd::Snapshots { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            let snaps = snapshots::list(&p);
            if snaps.is_empty() {
                println!("Nenhum snapshot para '{}'.", p);
                return Ok(());
            }
            println!("{:<28} {:<18} TAMANHO", "NOME", "CRIADO EM");
            for s in snaps {
                println!("{:<28} {:<18} {}", s.name, s.created, s.size);
            }
            Ok(())
        }
        Cmd::Rollback { profile, name, yes } => {
            let p = profile::resolve(Some(&profile))?;
            if !yes {
                let ok = Confirm::new()
                    .with_prompt(format!(
                        "Substituir estado atual de '{p}' pelo snapshot '{name}'?"
                    ))
                    .default(false)
                    .interact()
                    .unwrap_or(false);
                if !ok {
                    return Ok(());
                }
            }
            snapshots::rollback(&p, &name)
        }
        Cmd::Update { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::update(&p)
        }
        Cmd::Set(a) => {
            let params = cmd_set::SetParams {
                name: a.profile,
                ram: a.ram,
                cpu: a.cpu,
                disk: a.disk,
                user: a.user,
                password: a.pass,
                extra_ports: a.extra_ports,
                gpu_bdf: a.gpu,
                version: None,
                language: None,
                office_language: None,
                restart: a.restart,
            };
            let msg = cmd_set::run(params)?;
            println!("{msg}");
            Ok(())
        }
        Cmd::Restart { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::restart(&p)
        }
        Cmd::Remove {
            profile,
            yes,
            delete_storage,
        } => {
            if !yes {
                let ok = Confirm::new()
                    .with_prompt(format!(
                        "Remover perfil '{profile}' E todos os dados (storage, snapshots)?"
                    ))
                    .default(false)
                    .interact()
                    .unwrap_or(false);
                if !ok {
                    return Ok(());
                }
                let typed: String = Input::new()
                    .with_prompt("Tem certeza? Digite 'sim'")
                    .interact_text()
                    .unwrap_or_default();
                if typed != "sim" {
                    return Ok(());
                }
            }
            // CLI default: also delete the custom storage path (matches the
            // GUI default with the checkbox pre-checked). Future flag could
            // expose --keep-storage if needed.
            lifecycle::remove(&profile, delete_storage)
        }
        Cmd::Bundles(BundlesCmd::List) => {
            for b in bundles::list() {
                let tag = if b.custom { " (custom)" } else { "" };
                println!("  {:<16}{}", b.name, tag);
            }
            Ok(())
        }
        Cmd::Bundles(BundlesCmd::Show { name }) => {
            let src = bundles::resolve(&name)?;
            print!("{src}");
            Ok(())
        }
        Cmd::Bundles(BundlesCmd::Reapply { profile }) => {
            let p = profile::resolve(profile.as_deref())?;
            let path = crate::commands::reapply::run(&p)?;
            println!("Script gerado em: {}", path.display());
            println!("Dentro do Windows (PowerShell como admin), rode:");
            println!(
                "  powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"\\\\host.lan\\Data\\winbox-reapply.ps1\""
            );
            Ok(())
        }
        Cmd::Gpu => {
            let gpus = crate::core::gpu::list();
            if gpus.is_empty() {
                println!("Nenhuma GPU detectada via lspci.");
                return Ok(());
            }
            for g in &gpus {
                let primary = if g.is_primary_display {
                    " (display do host)"
                } else {
                    ""
                };
                let ready = if g.vfio_ready {
                    "vfio-pci"
                } else {
                    "host driver"
                };
                println!(
                    "{bdf}  {vendor} {model}  driver={driver}  iommu_group={grp}  estado={ready}{primary}",
                    bdf = g.bdf,
                    vendor = g.vendor,
                    model = g.model,
                    driver = g.driver.as_deref().unwrap_or("(none)"),
                    grp = g
                        .iommu_group
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "?".into()),
                );
                for note in &g.vfio_notes {
                    println!("    ! {}", note);
                }
            }
            Ok(())
        }
        Cmd::GpuSetup { bdf } => {
            crate::core::vfio_setup::apply(&bdf)?;
            println!("Configuração aplicada para {bdf}. Reinicie o sistema para que o vfio-pci assuma a GPU.");
            Ok(())
        }
        Cmd::GpuRevert { bdf } => {
            crate::core::vfio_setup::revert(&bdf)?;
            println!(
                "Configuração removida para {bdf}. Reinicie para que o driver original volte."
            );
            Ok(())
        }
        Cmd::Office(_) => unreachable!("Cmd::Office é roteado antes do dispatch legado"),
        Cmd::Gui => spawn_gui(),
    }
}

fn dispatch_json(cmd: Cmd) -> Result<Value> {
    match cmd {
        Cmd::Version => Ok(json!({
            "name": "winbox",
            "version": paths::WINBOX_VERSION,
        })),
        Cmd::Distros => Ok(json!(crate::core::image_family::SUPPORTED_DISTROS
            .iter()
            .map(|(id, label)| json!({ "id": id, "label": label }))
            .collect::<Vec<_>>())),
        Cmd::HostHealth => json_value(crate::core::health::report()),
        Cmd::Profile { profile } => json_value(profile_summary(&profile)?),
        Cmd::ViewerUrl { profile } => {
            Ok(json!({ "profile": profile, "url": viewer_url(&profile)? }))
        }
        Cmd::List => json_value(crate::commands::list::list()),
        Cmd::Install(a) => install_json(a),
        Cmd::Launch(a) => {
            let p = profile::resolve(a.profile.as_deref())?;
            emit_json_progress_if_requested(
                a.progress.as_deref(),
                &p,
                "launch",
                "starting",
                "Starting profile.",
            );
            cmd_launch::launch(&p, a.on_close.parse()?)?;
            emit_json_progress_if_requested(
                a.progress.as_deref(),
                &p,
                "launch",
                "finished",
                "Profile launched.",
            );
            Ok(json!({ "profile": p, "operation": "launch", "status": "finished" }))
        }
        Cmd::Start { profile, progress } => {
            let p = profile::resolve(profile.as_deref())?;
            emit_json_progress_if_requested(
                progress.as_deref(),
                &p,
                "start",
                "starting",
                "Starting profile.",
            );
            let docker = crate::core::docker::CliDocker;
            cmd_launch::start(&p, &docker).map_err(|e| anyhow::anyhow!("{e}"))?;
            emit_json_progress_if_requested(
                progress.as_deref(),
                &p,
                "start",
                "finished",
                "Profile started.",
            );
            Ok(json!({ "profile": p, "operation": "start", "status": "finished" }))
        }
        Cmd::Stop { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::stop(&p)?;
            Ok(json!({ "profile": p, "operation": "stop", "status": "finished" }))
        }
        Cmd::Kill { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::kill(&p)?;
            Ok(json!({ "profile": p, "operation": "kill", "status": "finished" }))
        }
        Cmd::Pause { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::pause(&p)?;
            Ok(json!({ "profile": p, "operation": "pause", "status": "finished" }))
        }
        Cmd::Resume { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::resume(&p)?;
            Ok(json!({ "profile": p, "operation": "resume", "status": "finished" }))
        }
        Cmd::Status { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            let c = paths::profile_container(&p);
            Ok(json!({ "profile": p, "status": crate::core::docker::container_status(&c) }))
        }
        Cmd::Default { profile } => {
            if let Some(profile) = profile {
                lifecycle::set_default(&profile)?;
            }
            Ok(json!({ "default": profile::get_default() }))
        }
        Cmd::Logs { profile, tail } => {
            let p = profile::resolve(profile.as_deref())?;
            let output = logs::tail(&p, tail.unwrap_or(200))?;
            Ok(json!({ "profile": p, "logs": output }))
        }
        Cmd::Snapshot { profile, name } => {
            snapshots::create(&profile, &name)?;
            Ok(json!({ "profile": profile, "snapshot": name, "status": "finished" }))
        }
        Cmd::Snapshots { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            json_value(snapshots::list(&p))
        }
        Cmd::Rollback { profile, name, .. } => {
            snapshots::rollback(&profile, &name)?;
            Ok(json!({ "profile": profile, "snapshot": name, "status": "finished" }))
        }
        Cmd::Update { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::update(&p)?;
            Ok(json!({ "profile": p, "operation": "update", "status": "finished" }))
        }
        Cmd::Set(a) => {
            let params = cmd_set::SetParams {
                name: a.profile,
                ram: a.ram,
                cpu: a.cpu,
                disk: a.disk,
                user: a.user,
                password: a.pass,
                extra_ports: a.extra_ports,
                gpu_bdf: a.gpu,
                version: None,
                language: None,
                office_language: None,
                restart: a.restart,
            };
            let message = cmd_set::run(params)?;
            Ok(json!({ "message": message }))
        }
        Cmd::Restart { profile } => {
            let p = profile::resolve(profile.as_deref())?;
            lifecycle::restart(&p)?;
            Ok(json!({ "profile": p, "operation": "restart", "status": "finished" }))
        }
        Cmd::Remove {
            profile,
            delete_storage,
            ..
        } => {
            lifecycle::remove(&profile, delete_storage)?;
            Ok(json!({ "profile": profile, "operation": "remove", "status": "finished" }))
        }
        Cmd::Bundles(BundlesCmd::List) => json_value(bundles::list()),
        Cmd::Bundles(BundlesCmd::Show { name }) => {
            let src = bundles::resolve(&name)?;
            Ok(json!({ "name": name, "content": src }))
        }
        Cmd::Bundles(BundlesCmd::Reapply { profile }) => {
            let p = profile::resolve(profile.as_deref())?;
            let path = crate::commands::reapply::run(&p)?;
            Ok(json!({ "profile": p, "path": path }))
        }
        Cmd::Gpu => json_value(crate::core::gpu::list()),
        Cmd::GpuSetup { bdf } => {
            crate::core::vfio_setup::apply(&bdf)?;
            Ok(json!({ "bdf": bdf, "status": "finished" }))
        }
        Cmd::GpuRevert { bdf } => {
            crate::core::vfio_setup::revert(&bdf)?;
            Ok(json!({ "bdf": bdf, "status": "finished" }))
        }
        Cmd::Office(_) => unreachable!("Cmd::Office é roteado antes do dispatch_json legado"),
        Cmd::Gui => spawn_gui().map(|_| json!({ "status": "started" })),
    }
}

fn install_json(a: InstallArgs) -> Result<Value> {
    if !a.yes {
        bail!("JSON install requires --yes.");
    }
    let name = a
        .profile
        .clone()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("JSON install requires a profile name."))?;
    let family = a.family.clone().unwrap_or_else(|| "windows".to_string());
    let parsed_family = crate::core::image_family::ImageFamily::parse(&family);
    let is_windows = parsed_family == crate::core::image_family::ImageFamily::Windows;
    if is_windows && a.pass.as_deref().unwrap_or("").is_empty() {
        bail!("JSON Windows install requires --pass.");
    }
    emit_json_progress_if_requested(
        a.progress.as_deref(),
        &name,
        "install",
        "starting",
        "Creating profile.",
    );
    let params = cmd_install::InstallParams {
        name: name.clone(),
        version: a.version.unwrap_or_else(|| "11".to_string()),
        ram: a.ram.unwrap_or_else(|| "4G".to_string()),
        cpu: a.cpu.unwrap_or_else(|| "2".to_string()),
        disk: a.disk.unwrap_or_else(|| "64G".to_string()),
        user: a.user.unwrap_or_else(|| {
            if parsed_family == crate::core::image_family::ImageFamily::LinuxCloud {
                "bruno".to_string()
            } else {
                "docker".to_string()
            }
        }),
        password: a.pass.unwrap_or_default(),
        language: a.language.unwrap_or_else(|| "Portuguese".to_string()),
        region: a.region.unwrap_or_else(|| "pt-BR".to_string()),
        keyboard: a.keyboard.unwrap_or_else(|| "pt-BR".to_string()),
        bundles: a.bundle.unwrap_or_default(),
        force: a.force,
        gpu_bdf: a.gpu,
        image_family: Some(family),
        boot: a.boot,
        iso_path: a.iso_path,
        storage_path: a.storage_path,
    };
    cmd_install::run(params)?;
    emit_json_progress_if_requested(
        a.progress.as_deref(),
        &name,
        "install",
        "finished",
        "Profile created.",
    );
    let summary = profile_summary(&name)?;
    Ok(json!({ "profile": summary, "operation": "install", "status": "finished" }))
}

fn profile_summary(name: &str) -> Result<crate::commands::list::ProfileSummary> {
    crate::commands::list::list()
        .into_iter()
        .find(|profile| profile.name == name)
        .ok_or_else(|| anyhow!("Perfil '{name}' não existe."))
}

fn viewer_url(profile: &str) -> Result<String> {
    let env = env_file::read(&paths::profile_env_file(profile))?;
    connect::viewer_url(&env).ok_or_else(|| anyhow!("WEB_PORT não definido para '{profile}'."))
}

fn json_value<T: Serialize>(value: T) -> Result<Value> {
    serde_json::to_value(value).map_err(|error| anyhow!(error))
}

fn print_json_success(value: Value) {
    println!("{}", json!({ "ok": true, "value": value }));
}

fn print_json_error(error: &anyhow::Error) {
    println!(
        "{}",
        json!({
            "ok": false,
            "error": {
                "code": machine_error_code(&error.to_string()),
                "message": error.to_string(),
                "hint": machine_error_hint(&error.to_string()),
            }
        })
    );
}

fn emit_json_progress_if_requested(
    format: Option<&str>,
    profile: &str,
    operation: &str,
    phase: &str,
    message: &str,
) {
    if format != Some("jsonl") {
        return;
    }
    println!(
        "{}",
        json!({
            "type": if phase == "finished" { "finished" } else { "progress" },
            "profile": profile,
            "operation": operation,
            "phase": phase,
            "status": if phase == "finished" { "success" } else { "running" },
            "message": message,
            "timestamp": chrono::Local::now().to_rfc3339(),
        })
    );
}

fn machine_error_code(message: &str) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains("docker") && lower.contains("not found")
        || lower.contains("docker não encontrado")
    {
        "docker_not_installed"
    } else if lower.contains("daemon") || lower.contains("docker info") {
        "docker_daemon_unavailable"
    } else if lower.contains("kvm") || lower.contains("/dev/kvm") {
        "kvm_unavailable"
    } else if lower.contains("storage") || lower.contains("armazenamento") {
        "storage_path_invalid"
    } else if lower.contains("space") || lower.contains("disco") {
        "disk_full"
    } else if lower.contains("port") || lower.contains("porta") {
        "port_unavailable"
    } else if lower.contains("não existe") || lower.contains("not found") {
        "profile_not_found"
    } else if lower.contains("unsupported") || lower.contains("suport") {
        "unsupported_preset"
    } else {
        "operation_failed"
    }
}

fn machine_error_hint(message: &str) -> &'static str {
    match machine_error_code(message) {
        "docker_not_installed" => "Install Docker Engine and retry.",
        "docker_daemon_unavailable" => "Start Docker and verify user permissions.",
        "kvm_unavailable" => "Enable virtualization/KVM and retry.",
        "storage_path_invalid" => "Choose a writable local storage path with enough free space.",
        "disk_full" => "Free disk space or choose a larger storage location.",
        "port_unavailable" => "Check port conflicts in Host Health.",
        "profile_not_found" => "Refresh profiles and choose an existing profile.",
        "unsupported_preset" => "Choose one of the supported distro/version presets.",
        _ => "Check Winbox logs and retry.",
    }
}

fn cmd_install_interactive(a: InstallArgs) -> Result<()> {
    let name = if let Some(n) = a.profile.clone() {
        n
    } else if a.yes {
        bail!("Em --yes o nome do perfil é obrigatório.");
    } else {
        let ask: String = Input::new()
            .with_prompt("Nome do perfil")
            .default("default".into())
            .interact_text()?;
        ask
    };
    profile::validate_name(&name)?;

    let host_info = host::info();
    let def_ver = "11";
    let def_ram = format!("{}G", (host_info.ram_gb / 3).max(4));
    let def_cpu = (host_info.cpu_cores / 2).max(2).to_string();
    let def_disk = "128G";
    let def_user = std::env::var("USER").unwrap_or_else(|_| "docker".into());
    let def_lang = "Portuguese";
    let def_region = "pt-BR";
    let def_kb = "pt-BR";

    let (ver, ram, cpu, disk, user, pass, lang, region, kb, bundles_str) = if a.yes {
        if a.pass.as_deref().unwrap_or("").is_empty() {
            bail!("Em --yes, informe a senha com --pass.");
        }
        (
            a.version.unwrap_or_else(|| def_ver.into()),
            a.ram.unwrap_or(def_ram),
            a.cpu.unwrap_or(def_cpu),
            a.disk.unwrap_or_else(|| def_disk.into()),
            a.user.unwrap_or(def_user),
            a.pass.unwrap_or_default(),
            a.language.unwrap_or_else(|| def_lang.into()),
            a.region.unwrap_or_else(|| def_region.into()),
            a.keyboard.unwrap_or_else(|| def_kb.into()),
            a.bundle.unwrap_or_default(),
        )
    } else {
        println!(
            "Host: {}G RAM · {} cores · {}G livres em $HOME",
            host_info.ram_gb, host_info.cpu_cores, host_info.free_gb
        );
        let ver: String = prompt_default("Versão Windows", a.version, def_ver)?;
        let ram: String = prompt_default("RAM (ex: 4G, 8G)", a.ram, &def_ram)?;
        let cpu: String = prompt_default(
            &format!("CPU cores (1-{})", host_info.cpu_cores),
            a.cpu,
            &def_cpu,
        )?;
        let disk: String = prompt_default("Disco (ex: 64G, 128G)", a.disk, def_disk)?;
        let user: String = prompt_default("Usuário Windows", a.user, &def_user)?;
        let pass = if let Some(p) = a.pass {
            p
        } else {
            Password::new()
                .with_prompt("Senha Windows")
                .allow_empty_password(false)
                .interact()?
        };
        let lang: String = prompt_default("Idioma", a.language, def_lang)?;
        let region: String = prompt_default("Região", a.region, def_region)?;
        let kb: String = prompt_default("Teclado", a.keyboard, def_kb)?;
        let bundles_str = if let Some(b) = a.bundle {
            b
        } else {
            println!("\nBundles disponíveis (essentials sempre aplicado):");
            for b in bundles::list() {
                if b.name != "essentials" {
                    println!("  - {}", b.name);
                }
            }
            prompt_default("Bundles extras (separados por vírgula)", None, "")?
        };
        (
            ver,
            ram,
            cpu,
            disk,
            user,
            pass,
            lang,
            region,
            kb,
            bundles_str,
        )
    };

    let profile_name = name.clone();
    let params = cmd_install::InstallParams {
        name,
        version: ver,
        ram,
        cpu,
        disk,
        user,
        password: pass,
        language: lang,
        region,
        keyboard: kb,
        bundles: bundles_str,
        force: a.force,
        gpu_bdf: a.gpu,
        // CLI is Windows-only for now; Linux paths are GUI-only in Phase 1.
        image_family: None,
        boot: None,
        iso_path: None,
        storage_path: None,
    };
    cmd_install::run(params)?;
    // Open web viewer
    if let Ok(map) = env_file::read(&paths::profile_env_file(&profile_name)) {
        if let Some(url) = connect::viewer_url(&map) {
            let _ = crate::core::opener::open_url(&url);
        }
    }
    println!("Perfil '{}' criado.", profile_name);
    Ok(())
}

fn prompt_default(prompt: &str, preset: Option<String>, default: &str) -> Result<String> {
    if let Some(v) = preset {
        return Ok(v);
    }
    let s: String = Input::new()
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()?;
    Ok(s)
}

fn spawn_gui() -> Result<()> {
    // Re-exec current binary with no args, detached.
    let exe = std::env::current_exe().map_err(|e| anyhow!(e))?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                // setsid → detach from tty
                libc_setsid();
                Ok(())
            });
        }
    }
    cmd.spawn()?;
    Ok(())
}

#[cfg(unix)]
unsafe fn libc_setsid() {
    extern "C" {
        fn setsid() -> i32;
    }
    let _ = setsid();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_version_returns_success_value() {
        let value = dispatch_json(Cmd::Version).expect("version");
        assert_eq!(value["name"], "winbox");
        assert!(value["version"]
            .as_str()
            .is_some_and(|version| !version.is_empty()));
    }

    #[test]
    fn json_distros_exposes_required_linux_presets() {
        let value = dispatch_json(Cmd::Distros).expect("distros");
        let ids = value
            .as_array()
            .expect("array")
            .iter()
            .filter_map(|entry| entry["id"].as_str())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"ubuntu-server"));
        assert!(ids.contains(&"xubuntu"));
        assert!(!ids.contains(&"lubuntu"));
    }

    #[test]
    fn json_install_args_parse_linux_contract_fields() {
        let cli = Cli::try_parse_from([
            "winbox",
            "--json",
            "install",
            "dev-linux",
            "--yes",
            "--family",
            "linux_distro",
            "--boot",
            "xubuntu",
            "--progress",
            "jsonl",
        ])
        .expect("parse");

        assert!(cli.json);
        let Cmd::Install(args) = cli.cmd else {
            panic!("expected install command");
        };
        assert_eq!(args.profile.as_deref(), Some("dev-linux"));
        assert_eq!(args.family.as_deref(), Some("linux_distro"));
        assert_eq!(args.boot.as_deref(), Some("xubuntu"));
        assert_eq!(args.progress.as_deref(), Some("jsonl"));
    }

    #[test]
    fn json_install_args_parse_linux_cloud_family() {
        let cli = Cli::try_parse_from([
            "winbox",
            "--json",
            "install",
            "dev-cloud",
            "--yes",
            "--family",
            "linux_cloud",
            "--boot",
            "xubuntu-desktop",
        ])
        .expect("parse");

        let Cmd::Install(args) = cli.cmd else {
            panic!("expected install command");
        };
        assert_eq!(args.family.as_deref(), Some("linux_cloud"));
        assert_eq!(args.boot.as_deref(), Some("xubuntu-desktop"));
    }

    #[test]
    fn json_error_code_mapping_is_actionable() {
        assert_eq!(
            machine_error_code("docker info failed: daemon unavailable"),
            "docker_daemon_unavailable"
        );
        assert_eq!(
            machine_error_hint("Porta 8006 em uso"),
            "Check port conflicts in Host Health."
        );
    }

    #[test]
    fn legacy_cli_json_contract_unchanged_for_non_office() {
        let value = dispatch_json(Cmd::Version).expect("legacy json command should still dispatch");

        assert_eq!(value["name"], "winbox");
        assert_eq!(
            machine_error_code("docker info failed: daemon unavailable"),
            "docker_daemon_unavailable"
        );
    }

    #[test]
    fn office_cli_parser_accepts_global_json_after_subcommand() {
        let cli = Cli::try_parse_from(["winbox", "office", "status", "office", "--json"])
            .expect("office status should parse with global json at the end");

        assert!(cli.json);
        let Cmd::Office(cli_office::OfficeArgs {
            command: cli_office::OfficeSubcommand::Status { profile },
        }) = cli.cmd
        else {
            panic!("expected office status command");
        };
        assert_eq!(profile, "office");
    }
}

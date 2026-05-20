use anyhow::{anyhow, bail, Result};
use clap::{Args, Parser, Subcommand};
use dialoguer::{Confirm, Input, Password};

use crate::commands::{
    install as cmd_install, launch as cmd_launch, lifecycle, logs, set as cmd_set,
};
use crate::core::{bundles, env_file, host, paths, profile, snapshots};

#[derive(Parser, Debug)]
#[command(name = "winbox", version = paths::WINBOX_VERSION, about = "Gerenciador de VMs Windows (dockur/windows)", disable_help_subcommand = true)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Cmd {
    /// Cria um novo perfil e baixa Windows + aplica bundles
    Install(InstallArgs),

    /// Sobe a VM (se necessário) e abre o RDP
    #[command(alias = "open")]
    Launch(LaunchArgs),

    /// Sobe a VM em background (sem RDP)
    #[command(alias = "up")]
    Start { profile: Option<String> },

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
}

#[derive(Args, Debug)]
struct LaunchArgs {
    profile: Option<String>,
    #[arg(long = "on-close", default_value = "keep")]
    on_close: String,
    #[arg(long = "keep-alive", short = 'k')]
    keep_alive: bool,
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
        Ok(cli) => match dispatch(cli.cmd) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("✗ {}", e);
                1
            }
        },
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
        Cmd::Start { profile } => {
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
        Cmd::Remove { profile, yes } => {
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
            lifecycle::remove(&profile, true)
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
        Cmd::Gui => spawn_gui(),
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
        let port = env_file::get_u16(&map, "WEB_PORT");
        if port != 0 {
            let _ = std::process::Command::new("xdg-open")
                .arg(format!("http://{}:{}", paths::HOST, port))
                .spawn();
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

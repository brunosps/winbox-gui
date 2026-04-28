use anyhow::Result;
use std::process::Command;

use super::{paths, profile};

fn main_entry_body() -> String {
    "[Desktop Entry]\n\
     Type=Application\n\
     Version=1.0\n\
     Name=winbox\n\
     GenericName=Windows VM Manager\n\
     Comment=Gerenciador de VMs Windows (dockur/windows)\n\
     Exec=winbox gui\n\
     Icon=winbox\n\
     Terminal=false\n\
     Categories=System;\n\
     Keywords=windows;vm;rdp;dockur;winbox;\n\
     StartupNotify=true\n"
        .to_string()
}

fn profile_entry_body(p: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Windows: {p}\n\
         GenericName=Windows VM\n\
         Comment=Inicia a VM Windows '{p}' e conecta via RDP\n\
         Exec=winbox launch {p}\n\
         Icon=winbox\n\
         Terminal=false\n\
         Categories=System;\n\
         Keywords=windows;vm;rdp;dockur;winbox;\n\
         StartupNotify=true\n\
         Actions=pause;stop;\n\
         \n\
         [Desktop Action pause]\n\
         Name=Pausar VM\n\
         Exec=winbox pause {p}\n\
         \n\
         [Desktop Action stop]\n\
         Name=Desligar VM\n\
         Exec=winbox stop {p}\n",
        p = p
    )
}

pub fn regenerate() -> Result<()> {
    let apps = paths::apps_dir();
    std::fs::create_dir_all(&apps)?;

    let profiles = profile::list_names();

    // Clean stale winbox-*.desktop files for profiles that no longer exist
    if let Ok(rd) = std::fs::read_dir(&apps) {
        for e in rd.flatten() {
            let fname = e.file_name();
            let Some(f) = fname.to_str() else { continue };
            if let Some(rest) = f.strip_prefix("winbox-") {
                if let Some(profile_name) = rest.strip_suffix(".desktop") {
                    if !profiles.iter().any(|p| p == profile_name) {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
        }
    }

    std::fs::write(apps.join("winbox.desktop"), main_entry_body())?;
    for p in &profiles {
        std::fs::write(
            apps.join(format!("winbox-{}.desktop", p)),
            profile_entry_body(p),
        )?;
    }

    let _ = Command::new("update-desktop-database").arg(&apps).status();
    Ok(())
}

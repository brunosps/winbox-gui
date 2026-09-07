use anyhow::{bail, Context, Result};
use include_dir::{include_dir, Dir};
use std::path::PathBuf;

use crate::core::{
    bundles, env_file, office_state::OfficeProvisioningState, paths, profile, validation,
};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates");

fn template(name: &str) -> Result<&'static str> {
    TEMPLATES
        .get_file(name)
        .and_then(|f| f.contents_utf8())
        .ok_or_else(|| anyhow::anyhow!("template '{}' missing from binary", name))
}

/// Regenerate the firstlogon.ps1 based on the profile's current BUNDLES,
/// write it to both `OEM_DIR/firstlogon.ps1` and `SHARED_DIR/winbox-reapply.ps1`,
/// and return the shared-dir path (the one the user runs inside Windows).
pub fn run(profile_name: &str) -> Result<PathBuf> {
    if !profile::exists(profile_name) {
        bail!("Perfil '{}' não existe.", profile_name);
    }

    let env_path = paths::profile_env_file(profile_name);
    let map = env_file::read(&env_path)?;
    let config = validation::office_env_config_from_map(&map);
    let state = OfficeProvisioningState::load_or_default(
        &paths::profile_cfg_dir(profile_name),
        profile_name,
    )?;
    // Reapply only regenerates PowerShell from the BUNDLES list; it never
    // edits config.env, so there is no VERSION/LANGUAGE/OFFICE_LANGUAGE
    // candidate update to pass to the immutable-field guard.
    validation::guard_office_immutable_fields(
        &config,
        validation::OfficeImmutableUpdate::default(),
        Some(&state),
    )?;
    let bundles_csv = env_file::get(&map, "BUNDLES").to_string();

    let shared_dir = std::path::PathBuf::from(env_file::get(&map, "SHARED_DIR"));
    if shared_dir.as_os_str().is_empty() {
        bail!("SHARED_DIR não encontrado em {}", env_path.display());
    }
    std::fs::create_dir_all(&shared_dir)
        .with_context(|| format!("mkdir {}", shared_dir.display()))?;

    let ps = build_firstlogon(&bundles_csv)?;

    // Also refresh the OEM copy so next VM reinstall starts from the current bundles.
    let oem_path = paths::profile_oem_dir(profile_name).join("firstlogon.ps1");
    if let Some(parent) = oem_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&oem_path, &ps)?;

    let reapply_path = shared_dir.join("winbox-reapply.ps1");
    std::fs::write(&reapply_path, &ps)
        .with_context(|| format!("writing {}", reapply_path.display()))?;
    Ok(reapply_path)
}

fn build_firstlogon(bundles_csv: &str) -> Result<String> {
    let mut ps = String::new();
    ps.push_str(template("firstlogon-header.ps1")?);

    let mut final_list: Vec<String> = vec!["essentials".to_string()];
    for b in validation::parse_bundle_csv(bundles_csv)? {
        if b != "essentials" {
            final_list.push(b);
        }
    }

    for b in &final_list {
        match bundles::resolve(b) {
            Ok(src) => {
                ps.push('\n');
                ps.push_str(&format!("# ────── bundle: {} ──────\n", b));
                ps.push_str(&format!("Write-Host '--- bundle: {} ---'\n", b));
                ps.push_str(&src);
                if !src.ends_with('\n') {
                    ps.push('\n');
                }
            }
            Err(e) => anyhow::bail!("bundle '{}' não encontrado: {e}", b),
        }
    }

    ps.push_str(template("firstlogon-footer.ps1")?);
    Ok(ps)
}

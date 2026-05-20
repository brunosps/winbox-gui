use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};

use super::bundles;
use super::{paths, validation};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates");

fn template(name: &str) -> Result<&'static str> {
    TEMPLATES
        .get_file(name)
        .and_then(|f| f.contents_utf8())
        .ok_or_else(|| anyhow::anyhow!("template '{}' missing from binary", name))
}

/// Generate install.bat + firstlogon.ps1 + authorized_keys under profile_oem_dir.
/// `bundles_csv` is the user-provided string ("" = only essentials; "a,b" = essentials + a + b).
pub fn generate(profile: &str, bundles_csv: &str) -> Result<()> {
    let oem = paths::profile_oem_dir(profile);
    std::fs::create_dir_all(&oem).with_context(|| format!("mkdir {}", oem.display()))?;

    // authorized_keys: use first SSH pubkey found, else delete.
    // paths::home() is cross-platform ($HOME on unix, %USERPROFILE% on Windows).
    let home = paths::home();
    let mut key: Option<String> = None;
    for cand in ["id_ed25519.pub", "id_rsa.pub", "id_ecdsa.pub"] {
        let p = home.join(".ssh").join(cand);
        if let Ok(content) = std::fs::read_to_string(&p) {
            key = Some(content);
            break;
        }
    }
    let auth_path = oem.join("authorized_keys");
    match key {
        Some(k) => std::fs::write(&auth_path, k)?,
        None => {
            let _ = std::fs::remove_file(&auth_path);
        }
    }

    // install.bat — copy template verbatim
    std::fs::write(oem.join("install.bat"), template("install.bat")?)?;

    // firstlogon.ps1 = header + each bundle block + footer
    let mut ps = String::new();
    ps.push_str(template("firstlogon-header.ps1")?);

    // Dedup + essentials always first
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
    std::fs::write(oem.join("firstlogon.ps1"), ps)?;

    Ok(())
}

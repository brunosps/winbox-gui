use serde::Serialize;

use super::env_file;
use super::image_family::ImageFamily;
use super::paths;

/// How the user reaches a running guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectMode {
    Rdp,
    WebVnc,
}

impl ConnectMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ConnectMode::Rdp => "rdp",
            ConnectMode::WebVnc => "web_vnc",
        }
    }
}

/// Resolve which connect mode applies to a profile.
///
/// Phase 1: Windows → RDP, anything Linux → web-VNC.
/// Phase 2 will switch curated distros (ubuntu/kubuntu/xubuntu/mint/debian/fedora)
/// to RDP once the cloud-init xrdp autoinstall lands.
pub fn resolve(family: ImageFamily, _boot: &str) -> ConnectMode {
    match family {
        ImageFamily::Windows => ConnectMode::Rdp,
        ImageFamily::LinuxDistro | ImageFamily::LinuxIso => ConnectMode::WebVnc,
    }
}

/// Best-effort lookup from a profile's config.env. Falls back to RDP if env
/// is unreadable (matches legacy behavior — every existing profile is Windows).
pub fn resolve_for_profile(profile: &str) -> ConnectMode {
    let env_path = paths::profile_env_file(profile);
    let map = match env_file::read(&env_path) {
        Ok(m) => m,
        Err(_) => return ConnectMode::Rdp,
    };
    let family = ImageFamily::from_env_map(&map);
    let boot = env_file::get(&map, "BOOT");
    resolve(family, boot)
}

use serde::{Deserialize, Serialize};

use super::env_file;
use super::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFamily {
    Windows,
    LinuxDistro,
    LinuxIso,
    LinuxCloud,
}

impl ImageFamily {
    pub fn as_env_value(self) -> &'static str {
        match self {
            ImageFamily::Windows => "windows",
            ImageFamily::LinuxDistro => "linux_distro",
            ImageFamily::LinuxIso => "linux_iso",
            ImageFamily::LinuxCloud => "linux_cloud",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "linux_distro" | "linux-distro" | "linux" => ImageFamily::LinuxDistro,
            "linux_iso" | "linux-iso" | "iso" => ImageFamily::LinuxIso,
            "linux_cloud" | "linux-cloud" | "cloud" => ImageFamily::LinuxCloud,
            _ => ImageFamily::Windows,
        }
    }

    pub fn from_env_map(map: &env_file::EnvMap) -> Self {
        Self::parse(env_file::get(map, "IMAGE_FAMILY"))
    }

    pub fn is_linux(self) -> bool {
        matches!(
            self,
            ImageFamily::LinuxDistro | ImageFamily::LinuxIso | ImageFamily::LinuxCloud
        )
    }

    pub fn docker_image(self) -> &'static str {
        match self {
            ImageFamily::Windows => paths::IMAGE_WINDOWS,
            ImageFamily::LinuxDistro | ImageFamily::LinuxIso | ImageFamily::LinuxCloud => {
                paths::IMAGE_QEMU
            }
        }
    }
}

/// Curated distros recognized by qemux/qemu's BOOT keyword resolver.
/// Used by the GUI wizard dropdown and validated server-side.
pub const SUPPORTED_DISTROS: &[(&str, &str)] = &[
    ("ubuntu", "Ubuntu Desktop"),
    ("ubuntu-server", "Ubuntu Server"),
    ("kubuntu", "Kubuntu"),
    ("xubuntu", "Xubuntu"),
    ("mint", "Linux Mint"),
    ("debian", "Debian"),
    ("fedora", "Fedora"),
    ("alma", "Alma Linux"),
    ("rocky", "Rocky Linux"),
    ("centos", "CentOS"),
    ("opensuse", "openSUSE"),
    ("arch", "Arch"),
    ("manjaro", "Manjaro"),
    ("cachyos", "CachyOS"),
    ("kali", "Kali"),
    ("alpine", "Alpine"),
    ("gentoo", "Gentoo"),
    ("slackware", "Slackware"),
    ("nixos", "NixOS"),
    ("mx", "MX Linux"),
    ("zorin", "Zorin OS"),
    ("tails", "Tails"),
];

pub fn is_supported_distro_boot(boot: &str) -> bool {
    let normalized = boot.trim().to_ascii_lowercase();
    if normalized.starts_with("http://") || normalized.starts_with("https://") {
        return true;
    }
    SUPPORTED_DISTROS.iter().any(|(id, _)| *id == normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_distros_match_qemux_boot_keywords() {
        assert!(is_supported_distro_boot("ubuntu-server"));
        assert!(is_supported_distro_boot("xubuntu"));
        assert!(!is_supported_distro_boot("lubuntu"));
    }
}

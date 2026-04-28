use serde::{Deserialize, Serialize};

use super::env_file;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFamily {
    Windows,
    LinuxDistro,
    LinuxIso,
}

impl ImageFamily {
    pub fn as_env_value(self) -> &'static str {
        match self {
            ImageFamily::Windows => "windows",
            ImageFamily::LinuxDistro => "linux_distro",
            ImageFamily::LinuxIso => "linux_iso",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "linux_distro" | "linux-distro" | "linux" => ImageFamily::LinuxDistro,
            "linux_iso" | "linux-iso" | "iso" => ImageFamily::LinuxIso,
            _ => ImageFamily::Windows,
        }
    }

    pub fn from_env_map(map: &env_file::EnvMap) -> Self {
        Self::parse(env_file::get(map, "IMAGE_FAMILY"))
    }

    pub fn is_linux(self) -> bool {
        matches!(self, ImageFamily::LinuxDistro | ImageFamily::LinuxIso)
    }

    pub fn docker_image(self) -> &'static str {
        match self {
            ImageFamily::Windows => "dockurr/windows",
            ImageFamily::LinuxDistro | ImageFamily::LinuxIso => "qemux/qemu",
        }
    }
}

/// Curated 24 distros recognized by qemus/qemu's BOOT keyword resolver.
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

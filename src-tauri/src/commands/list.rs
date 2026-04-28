use serde::Serialize;

use crate::core::connect::{self, ConnectMode};
use crate::core::image_family::ImageFamily;
use crate::core::{docker, env_file, paths, profile};

#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    pub name: String,
    pub status: String,
    pub web_port: u16,
    pub rdp_port: u16,
    pub ram: String,
    pub bundles: String,
    pub is_default: bool,
    pub image_family: ImageFamily,
    pub connect_mode: ConnectMode,
    pub boot: String,
    pub iso_path: String,
}

pub fn list() -> Vec<ProfileSummary> {
    let default = profile::get_default();
    profile::list_names()
        .into_iter()
        .filter_map(|name| {
            let env = paths::profile_env_file(&name);
            let map = env_file::read(&env).ok()?;
            let container = paths::profile_container(&name);
            let status = docker::container_status(&container);
            let status = match status.as_str() {
                "absent" => "absent".to_string(),
                other => other.to_string(),
            };
            let family = ImageFamily::from_env_map(&map);
            let boot = env_file::get(&map, "BOOT").to_string();
            let iso_path = env_file::get(&map, "ISO_PATH").to_string();
            let connect_mode = connect::resolve(family, &boot);
            Some(ProfileSummary {
                is_default: default.as_deref() == Some(name.as_str()),
                web_port: env_file::get_u16(&map, "WEB_PORT"),
                rdp_port: env_file::get_u16(&map, "RDP_PORT"),
                ram: env_file::get(&map, "RAM_SIZE").to_string(),
                bundles: env_file::get(&map, "BUNDLES").to_string(),
                status,
                name,
                image_family: family,
                connect_mode,
                boot,
                iso_path,
            })
        })
        .collect()
}

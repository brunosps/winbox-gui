use std::path::PathBuf;

pub const WINBOX_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const IMAGE: &str = "dockurr/windows";
pub const HOST: &str = "127.0.0.1";
pub const BASE_WEB_PORT: u16 = 8006;
pub const BASE_RDP_PORT: u16 = 3389;
pub const BASE_SSH_PORT: u16 = 2222;

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg(var: &str, fallback_rel: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(fallback_rel))
}

pub fn config_dir() -> PathBuf {
    xdg("XDG_CONFIG_HOME", ".config").join("winbox")
}

pub fn data_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("winbox")
}

pub fn apps_dir() -> PathBuf {
    xdg("XDG_DATA_HOME", ".local/share").join("applications")
}

pub fn default_file() -> PathBuf {
    config_dir().join("default")
}

pub fn profiles_cfg_dir() -> PathBuf {
    config_dir().join("profiles")
}

pub fn profiles_data_dir() -> PathBuf {
    data_dir().join("profiles")
}

pub fn bundles_user_dir() -> PathBuf {
    config_dir().join("bundles")
}

pub fn shared_root() -> PathBuf {
    home().join("Windows")
}

pub fn profile_cfg_dir(p: &str) -> PathBuf {
    profiles_cfg_dir().join(p)
}

pub fn profile_data_dir(p: &str) -> PathBuf {
    profiles_data_dir().join(p)
}

pub fn profile_env_file(p: &str) -> PathBuf {
    profile_cfg_dir(p).join("config.env")
}

pub fn profile_compose_file(p: &str) -> PathBuf {
    profile_cfg_dir(p).join("compose.yml")
}

pub fn profile_oem_dir(p: &str) -> PathBuf {
    profile_cfg_dir(p).join("oem")
}

pub fn profile_storage_dir(p: &str) -> PathBuf {
    profile_data_dir(p).join("storage")
}

pub fn profile_snapshots_dir(p: &str) -> PathBuf {
    profile_data_dir(p).join("snapshots")
}

pub fn profile_shared_dir(p: &str) -> PathBuf {
    shared_root().join(p)
}

pub fn profile_container(p: &str) -> String {
    format!("winbox-{}", p)
}

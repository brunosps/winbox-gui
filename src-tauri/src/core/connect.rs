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
/// Now uniformly returns [`ConnectMode::WebVnc`]: both dockurr/windows
/// and dockurr/dockur serve a built-in noVNC web viewer on `WEB_PORT`,
/// and there is no longer a hard dependency on `xfreerdp3` / `mstsc` on
/// the host. The enum is kept to preserve the on-wire shape for the
/// frontend; the `Rdp` variant remains for serialization compat but is
/// not produced by this resolver.
pub fn resolve(_family: ImageFamily, _boot: &str) -> ConnectMode {
    ConnectMode::WebVnc
}

/// Best-effort lookup from a profile's config.env. Always returns
/// [`ConnectMode::WebVnc`]; the lookup is preserved so failures to
/// read the env still don't crash and future per-profile overrides
/// can be added here without changing call sites.
pub fn resolve_for_profile(profile: &str) -> ConnectMode {
    let _ = env_file::read(&paths::profile_env_file(profile));
    ConnectMode::WebVnc
}

pub fn viewer_port(map: &env_file::EnvMap) -> u16 {
    let desktop_port = env_file::get_u16(map, "DESKTOP_WEB_PORT");
    if desktop_port != 0 {
        desktop_port
    } else {
        env_file::get_u16(map, "WEB_PORT")
    }
}

pub fn viewer_url(map: &env_file::EnvMap) -> Option<String> {
    let desktop_port = env_file::get_u16(map, "DESKTOP_WEB_PORT");
    if desktop_port != 0 {
        return Some(format!(
            "http://{}:{}/vnc.html?autoconnect=true&resize=scale",
            paths::HOST,
            desktop_port
        ));
    }

    let port = env_file::get_u16(map, "WEB_PORT");
    if port == 0 {
        return None;
    }
    Some(format!(
        "http://{}:{}/?autoconnect=true&resize=scale",
        paths::HOST,
        port
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_viewer_prefers_guest_novnc_port() {
        let map =
            env_file::parse("WEB_PORT=8007\nDESKTOP_WEB_PORT=8017\nRDP_PORT=3390\nSSH_PORT=2223\n");
        assert_eq!(viewer_port(&map), 8017);
        assert_eq!(
            viewer_url(&map).as_deref(),
            Some("http://127.0.0.1:8017/vnc.html?autoconnect=true&resize=scale")
        );
    }

    #[test]
    fn qemu_viewer_falls_back_to_web_port() {
        let map = env_file::parse("WEB_PORT=8007\nRDP_PORT=3390\nSSH_PORT=2223\n");
        assert_eq!(viewer_port(&map), 8007);
        assert_eq!(
            viewer_url(&map).as_deref(),
            Some("http://127.0.0.1:8007/?autoconnect=true&resize=scale")
        );
    }
}

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

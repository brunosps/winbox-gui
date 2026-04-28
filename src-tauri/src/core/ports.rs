use anyhow::Result;
use std::net::{TcpListener, UdpSocket};

use super::env_file;
use super::paths;

/// A (web, rdp, ssh) tuple reserved for a new profile.
pub type Trio = (u16, u16, u16);

pub fn port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok() && UdpSocket::bind(("127.0.0.1", port)).is_ok()
}

fn used_ports() -> Vec<u16> {
    let mut used = Vec::new();
    let dir = paths::profiles_cfg_dir();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let env = e.path().join("config.env");
            if !env.is_file() {
                continue;
            }
            if let Ok(map) = env_file::read(&env) {
                for k in ["WEB_PORT", "RDP_PORT", "SSH_PORT"] {
                    let v = env_file::get_u16(&map, k);
                    if v != 0 {
                        used.push(v);
                    }
                }
            }
        }
    }
    used
}

/// Allocate (web, rdp, ssh) starting from BASE_* and skipping both in-use
/// by other profiles and busy on the host.
pub fn allocate() -> Result<Trio> {
    let used = used_ports();
    let mut web = paths::BASE_WEB_PORT;
    let mut rdp = paths::BASE_RDP_PORT;
    let mut ssh = paths::BASE_SSH_PORT;

    let taken = |p: u16, used: &[u16]| used.contains(&p) || !port_free(p);

    while taken(web, &used) {
        web += 1;
    }
    while taken(rdp, &used) || rdp == web {
        rdp += 1;
    }
    while taken(ssh, &used) || ssh == web || ssh == rdp {
        ssh += 1;
    }

    Ok((web, rdp, ssh))
}

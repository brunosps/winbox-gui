use anyhow::Result;

use crate::core::{docker, paths};

pub fn tail(profile: &str, n: usize) -> Result<String> {
    let c = paths::profile_container(profile);
    let n_str = n.to_string();
    docker::logs(&c, &["--tail", &n_str])
}

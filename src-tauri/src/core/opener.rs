//! Cross-platform "open this URL in the default browser" helper.
//!
//! Linux/WSL uses `xdg-open` (which, inside WSL with `wslu` installed,
//! forwards to the Windows default browser). Windows uses `cmd /c start`.
//! macOS uses `open` for completeness.

use anyhow::{anyhow, Result};
use std::process::Command;

/// Open `url` in the user's default browser. Non-blocking (spawns and
/// returns). Returns an error only if the launcher process fails to spawn.
pub fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // `start` is a cmd builtin, not an exe — must go through cmd.
        // The empty "" is the window title arg that `start` expects when
        // the first quoted token would otherwise be taken as the title.
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|e| anyhow!("falha ao abrir URL ({url}): {e}"))?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| anyhow!("falha ao abrir URL ({url}): {e}"))?;
        Ok(())
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| anyhow!("falha ao abrir URL ({url}): {e}"))?;
        Ok(())
    }
}

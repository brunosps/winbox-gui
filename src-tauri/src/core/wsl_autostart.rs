//! WSL2 autostart plumbing — A1 of the backend roadmap.
//!
//! When Windows boots, no WSL distro is automatically running, so
//! containers with `restart: unless-stopped` never come back without
//! manual intervention. This module renders the two pieces needed to
//! fix that:
//!
//! 1. A `/etc/wsl.conf` patch that adds `[boot] command=...` so the
//!    distro starts the requested systemd services on first invocation.
//! 2. A Windows Scheduled Task that fires `wsl.exe -d <distro> -- true`
//!    on user logon, which "wakes" the distro and triggers the boot
//!    command without opening a terminal.
//!
//! Render functions are pure (no I/O) so they can be unit-tested. The
//! `install`/`uninstall` functions live behind `cfg(target_os = "windows")`
//! and shell out to `wsl.exe` and `schtasks.exe`.

#[cfg(target_os = "windows")]
use anyhow::Context;
use anyhow::{anyhow, Result};
use serde::Serialize;
#[cfg(target_os = "windows")]
use std::io::Write;
#[cfg(target_os = "windows")]
use std::process::{Command, Stdio};

/// What the autostart hook should bring up. `distro` is the WSL distro
/// name (`wsl --list --verbose` NAME column). `services` are the
/// systemd unit names to `systemctl start` on boot — order is preserved.
#[derive(Debug, Clone, Serialize)]
pub struct WslAutostartPlan {
    pub distro: String,
    pub services: Vec<String>,
}

impl WslAutostartPlan {
    pub fn new(distro: impl Into<String>) -> Self {
        Self {
            distro: distro.into(),
            services: Vec::new(),
        }
    }

    pub fn with_service(mut self, service: impl Into<String>) -> Self {
        self.services.push(service.into());
        self
    }
}

/// Stable identifier used both as scheduled-task name and as a marker
/// in the rendered `wsl.conf` so we can detect prior installs.
pub const TASK_NAME: &str = "winbox-wsl-autostart";
const CONF_MARKER: &str = "# managed-by: winbox-wsl-autostart";

/// Render the boot command line that will live under `[boot]` in
/// `/etc/wsl.conf`. Pure function.
pub fn render_boot_command(plan: &WslAutostartPlan) -> String {
    if plan.services.is_empty() {
        return String::new();
    }
    format!("systemctl start {}", plan.services.join(" "))
}

/// Apply the autostart plan onto an existing `/etc/wsl.conf` body and
/// return the new body. Adds or updates the `[boot]` section so it
/// contains `command=...` plus the managed-by marker. Other sections
/// are preserved verbatim. Pure function — does not touch the filesystem.
pub fn render_wsl_conf(existing: &str, plan: &WslAutostartPlan) -> String {
    let command = render_boot_command(plan);
    if command.is_empty() {
        return existing.to_string();
    }

    // Walk the existing conf line by line, copying sections, and emit
    // a fresh `[boot]` section. If a `[boot]` section already exists,
    // we drop its previous `command=` line (if any) and re-emit a new
    // one with the managed-by marker.
    let mut out: Vec<String> = Vec::new();
    let mut in_boot = false;
    let mut boot_emitted = false;

    for raw in existing.lines() {
        let trimmed = raw.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            // Section header. If we were inside [boot] and never emitted
            // command, do it now before moving on.
            if in_boot && !boot_emitted {
                out.push(format!("command = {command}"));
                out.push(CONF_MARKER.to_string());
                boot_emitted = true;
            }
            in_boot = trimmed.eq_ignore_ascii_case("[boot]");
            out.push(raw.to_string());
            continue;
        }

        if in_boot {
            // Drop prior managed line and any prior command= we owned.
            if trimmed == CONF_MARKER {
                continue;
            }
            let lower = trimmed.to_lowercase();
            if lower.starts_with("command") && lower.contains('=') {
                // Replace it with ours.
                if !boot_emitted {
                    out.push(format!("command = {command}"));
                    out.push(CONF_MARKER.to_string());
                    boot_emitted = true;
                }
                continue;
            }
        }

        out.push(raw.to_string());
    }

    // If file ended inside [boot] without command, append now.
    if in_boot && !boot_emitted {
        out.push(format!("command = {command}"));
        out.push(CONF_MARKER.to_string());
        boot_emitted = true;
    }

    // If we never saw a [boot] section, append a fresh one.
    if !boot_emitted {
        if !out.is_empty() && !out.last().map(|l| l.is_empty()).unwrap_or(false) {
            out.push(String::new());
        }
        out.push("[boot]".to_string());
        out.push(format!("command = {command}"));
        out.push(CONF_MARKER.to_string());
    }

    // Normalize trailing newline.
    let mut body = out.join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    body
}

/// Render the Windows scheduled-task XML that wakes the distro on user
/// logon. The task simply runs `wsl.exe -d <distro> -- true` which is
/// enough to trigger the `[boot] command` from `wsl.conf`. Pure function.
pub fn render_scheduled_task_xml(plan: &WslAutostartPlan) -> String {
    // schtasks /create /xml expects this Task Scheduler 1.2 schema.
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>winbox-gui: wake WSL distro '{distro}' at logon so dockur containers come back up.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <RunOnlyIfNetworkAvailable>false</RunOnlyIfNetworkAvailable>
    <IdleSettings>
      <StopOnIdleEnd>false</StopOnIdleEnd>
      <RestartOnIdle>false</RestartOnIdle>
    </IdleSettings>
    <AllowStartOnDemand>true</AllowStartOnDemand>
    <Enabled>true</Enabled>
    <Hidden>false</Hidden>
    <RunOnlyIfIdle>false</RunOnlyIfIdle>
    <WakeToRun>false</WakeToRun>
    <ExecutionTimeLimit>PT5M</ExecutionTimeLimit>
    <Priority>7</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>wsl.exe</Command>
      <Arguments>-d {distro} -- /bin/true</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        distro = plan.distro
    )
}

/// Was this wsl.conf body produced (or updated) by us? Useful for the
/// UI to indicate "autostart is configured" without re-parsing.
pub fn is_managed(existing: &str) -> bool {
    existing.contains(CONF_MARKER)
}

#[cfg(target_os = "windows")]
pub fn install(plan: &WslAutostartPlan) -> Result<()> {
    // 1. Read /etc/wsl.conf inside the target distro, patch it, write back.
    let read = Command::new("wsl.exe")
        .args([
            "-d",
            &plan.distro,
            "-u",
            "root",
            "--",
            "cat",
            "/etc/wsl.conf",
        ])
        .output()
        .context("failed to read /etc/wsl.conf via wsl.exe")?;
    let existing = if read.status.success() {
        String::from_utf8_lossy(&read.stdout).to_string()
    } else {
        // /etc/wsl.conf may not exist yet — treat as empty.
        String::new()
    };
    let new_body = render_wsl_conf(&existing, plan);

    // tee the new body into /etc/wsl.conf as root via stdin to avoid
    // any shell-quoting trouble with the multi-line content.
    write_wsl_conf(&plan.distro, &new_body).context("failed to write /etc/wsl.conf")?;

    // 2. Drop the scheduled-task XML to a temp file, then register it.
    let xml = render_scheduled_task_xml(plan);
    let tmp = std::env::temp_dir().join(format!("{TASK_NAME}.xml"));
    std::fs::write(&tmp, xml.as_bytes()).context("failed to write task XML")?;

    let st = Command::new("schtasks.exe")
        .args([
            "/Create",
            "/F",
            "/TN",
            TASK_NAME,
            "/XML",
            &tmp.to_string_lossy(),
        ])
        .status()
        .context("failed to invoke schtasks.exe")?;
    if !st.success() {
        return Err(anyhow!(
            "schtasks.exe /Create returned non-zero (need elevated shell?)"
        ));
    }
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn uninstall(plan: &WslAutostartPlan) -> Result<()> {
    let _ = Command::new("schtasks.exe")
        .args(["/Delete", "/F", "/TN", TASK_NAME])
        .status();

    // Re-render wsl.conf with empty services to strip our boot command.
    let read = Command::new("wsl.exe")
        .args([
            "-d",
            &plan.distro,
            "-u",
            "root",
            "--",
            "cat",
            "/etc/wsl.conf",
        ])
        .output()
        .context("failed to read /etc/wsl.conf")?;
    let existing = if read.status.success() {
        String::from_utf8_lossy(&read.stdout).to_string()
    } else {
        return Ok(());
    };

    let stripped = strip_managed_block(&existing);
    write_wsl_conf(&plan.distro, &stripped).context("failed to rewrite /etc/wsl.conf")?;
    Ok(())
}

/// Pipe `body` into `tee /etc/wsl.conf` inside the target distro,
/// running as root. Stdin avoids shell escape issues with multi-line
/// content or embedded quotes.
#[cfg(target_os = "windows")]
fn write_wsl_conf(distro: &str, body: &str) -> Result<()> {
    let mut child = Command::new("wsl.exe")
        .args(["-d", distro, "-u", "root", "--", "tee", "/etc/wsl.conf"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn wsl.exe tee")?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow!("no stdin handle on wsl.exe tee"))?;
        stdin.write_all(body.as_bytes()).context("write stdin")?;
    }
    let status = child.wait().context("wait for wsl.exe tee")?;
    if !status.success() {
        return Err(anyhow!("wsl.exe tee returned non-zero"));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install(_plan: &WslAutostartPlan) -> Result<()> {
    Err(anyhow!(
        "wsl_autostart::install is only available on Windows"
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn uninstall(_plan: &WslAutostartPlan) -> Result<()> {
    Err(anyhow!(
        "wsl_autostart::uninstall is only available on Windows"
    ))
}

/// Remove the `command = ...` line we own (identified by the marker)
/// plus the marker itself, but leave the rest of `[boot]` and other
/// sections intact. Pure function.
pub fn strip_managed_block(existing: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut in_boot = false;
    let mut drop_next_command = false;

    for raw in existing.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_boot = trimmed.eq_ignore_ascii_case("[boot]");
            drop_next_command = false;
            out.push(raw.to_string());
            continue;
        }
        if in_boot && trimmed == CONF_MARKER {
            // Drop marker, and remember to drop the preceding command
            // we already emitted (back up the vec).
            if let Some(last) = out.last() {
                let ll = last.trim().to_lowercase();
                if ll.starts_with("command") {
                    out.pop();
                }
            }
            drop_next_command = true;
            continue;
        }
        if in_boot && drop_next_command {
            let lower = trimmed.to_lowercase();
            if lower.starts_with("command") {
                continue;
            }
            drop_next_command = false;
        }
        out.push(raw.to_string());
    }

    let mut body = out.join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> WslAutostartPlan {
        WslAutostartPlan::new("Ubuntu-24.04")
            .with_service("docker")
            .with_service("mint-vm")
    }

    #[test]
    fn render_boot_command_joins_services() {
        let p = plan();
        assert_eq!(render_boot_command(&p), "systemctl start docker mint-vm");
    }

    #[test]
    fn render_boot_command_empty_plan_is_empty() {
        let p = WslAutostartPlan::new("Ubuntu");
        assert_eq!(render_boot_command(&p), "");
    }

    #[test]
    fn render_wsl_conf_appends_boot_section_to_empty_input() {
        let body = render_wsl_conf("", &plan());
        assert!(body.contains("[boot]"));
        assert!(body.contains("command = systemctl start docker mint-vm"));
        assert!(body.contains(CONF_MARKER));
        assert!(is_managed(&body));
    }

    #[test]
    fn render_wsl_conf_preserves_other_sections() {
        let existing = "[user]\ndefault=bruno\n\n[interop]\nappendWindowsPath=false\n";
        let body = render_wsl_conf(existing, &plan());
        assert!(body.contains("[user]"));
        assert!(body.contains("default=bruno"));
        assert!(body.contains("[interop]"));
        assert!(body.contains("appendWindowsPath=false"));
        assert!(body.contains("[boot]"));
        assert!(body.contains("command = systemctl start docker mint-vm"));
    }

    #[test]
    fn render_wsl_conf_replaces_existing_boot_command() {
        let existing = "[boot]\ncommand = systemctl start docker\n";
        let body = render_wsl_conf(existing, &plan());
        assert_eq!(
            body.matches("command").count(),
            1,
            "should not duplicate command lines"
        );
        assert!(body.contains("command = systemctl start docker mint-vm"));
        assert!(body.contains(CONF_MARKER));
    }

    #[test]
    fn render_wsl_conf_idempotent_when_marker_already_present() {
        let first = render_wsl_conf("", &plan());
        let second = render_wsl_conf(&first, &plan());
        assert_eq!(first, second);
    }

    #[test]
    fn render_wsl_conf_no_services_returns_input_unchanged() {
        let existing = "[user]\ndefault=bruno\n";
        let body = render_wsl_conf(existing, &WslAutostartPlan::new("Ubuntu"));
        assert_eq!(body, existing);
    }

    #[test]
    fn strip_managed_block_removes_our_command_only() {
        let with_managed =
            render_wsl_conf("[user]\ndefault=bruno\n\n[boot]\nsystemd=true\n", &plan());
        let stripped = strip_managed_block(&with_managed);
        assert!(!stripped.contains(CONF_MARKER));
        assert!(!stripped.contains("systemctl start docker mint-vm"));
        assert!(stripped.contains("[boot]"));
        assert!(stripped.contains("systemd=true"));
        assert!(stripped.contains("default=bruno"));
    }

    #[test]
    fn scheduled_task_xml_mentions_distro_and_command() {
        let xml = render_scheduled_task_xml(&plan());
        assert!(xml.contains("<Command>wsl.exe</Command>"));
        assert!(xml.contains("-d Ubuntu-24.04"));
        assert!(xml.contains("<LogonTrigger>"));
        assert!(xml.contains("<Enabled>true</Enabled>"));
        assert!(xml.starts_with("<?xml"));
    }

    #[test]
    fn scheduled_task_xml_uses_least_privilege() {
        let xml = render_scheduled_task_xml(&plan());
        // We don't ask for admin — autostart should run as the logged-in user.
        assert!(xml.contains("<RunLevel>LeastPrivilege</RunLevel>"));
    }
}

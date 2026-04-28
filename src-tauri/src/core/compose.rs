use anyhow::{Context, Result};
use std::path::Path;

use super::env_file;
use super::image_family::ImageFamily;
use super::paths;
use super::validation;

/// Generate compose.yml at profile_compose_file(name). Branches on
/// IMAGE_FAMILY: Windows uses dockurr/windows; Linux distros and
/// custom-ISO profiles use qemus/qemu. VFIO GPU passthrough is added
/// when GPU_BDF is set on the host config.
pub fn write(profile: &str) -> Result<()> {
    let env_path = paths::profile_env_file(profile);
    let (family, extra_ports, extra_devices, extra_caps, arguments, iso_path) =
        if env_path.is_file() {
            let map = env_file::read(&env_path)?;
            let family = ImageFamily::from_env_map(&map);
            let ports = build_extra_ports(env_file::get(&map, "EXTRA_PORTS"))?;
            let gpu_bdf = env_file::get(&map, "GPU_BDF").trim().to_string();
            let (devs, caps, args) = if gpu_bdf.is_empty() {
                (String::new(), String::new(), default_arguments())
            } else {
                build_gpu_block(&gpu_bdf)
            };
            let iso = env_file::get(&map, "ISO_PATH").to_string();
            (family, ports, devs, caps, args, iso)
        } else {
            (
                ImageFamily::Windows,
                String::new(),
                String::new(),
                String::new(),
                default_arguments(),
                String::new(),
            )
        };

    let body = match family {
        ImageFamily::Windows => {
            render_windows(&arguments, &extra_devices, &extra_caps, &extra_ports)
        }
        ImageFamily::LinuxDistro => {
            render_linux_distro(&arguments, &extra_devices, &extra_caps, &extra_ports)
        }
        ImageFamily::LinuxIso => render_linux_iso(
            &arguments,
            &extra_devices,
            &extra_caps,
            &extra_ports,
            &iso_path,
        ),
    };

    let path = paths::profile_compose_file(profile);
    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn render_windows(arguments: &str, devs: &str, caps: &str, extra: &str) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: dockurr/windows\n    \
         container_name: ${{CONTAINER_NAME}}\n    \
         environment:\n      \
         VERSION: ${{VERSION}}\n      \
         RAM_SIZE: ${{RAM_SIZE}}\n      \
         CPU_CORES: ${{CPU_CORES}}\n      \
         DISK_SIZE: ${{DISK_SIZE}}\n      \
         USERNAME: ${{USERNAME}}\n      \
         PASSWORD: ${{PASSWORD}}\n      \
         LANGUAGE: ${{LANGUAGE}}\n      \
         REGION: ${{REGION}}\n      \
         KEYBOARD: ${{KEYBOARD}}\n      \
         TZ: ${{TZ}}\n      \
         ARGUMENTS: \"{args}\"\n    \
         devices:\n      \
         - /dev/kvm\n      \
         - /dev/net/tun{devs}\n    \
         cap_add:\n      \
         - NET_ADMIN{caps}\n    \
         ports:\n      \
         - \"127.0.0.1:${{WEB_PORT}}:8006\"\n      \
         - \"127.0.0.1:${{RDP_PORT}}:3389/tcp\"\n      \
         - \"127.0.0.1:${{RDP_PORT}}:3389/udp\"{extra}\n    \
         volumes:\n      \
         - ${{STORAGE_DIR}}:/storage\n      \
         - ${{SHARED_DIR}}:/shared\n      \
         - ${{OEM_DIR}}:/oem\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
    )
}

fn render_linux_distro(arguments: &str, devs: &str, caps: &str, extra: &str) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: qemux/qemu\n    \
         container_name: ${{CONTAINER_NAME}}\n    \
         environment:\n      \
         BOOT: ${{BOOT}}\n      \
         RAM_SIZE: ${{RAM_SIZE}}\n      \
         CPU_CORES: ${{CPU_CORES}}\n      \
         DISK_SIZE: ${{DISK_SIZE}}\n      \
         TZ: ${{TZ}}\n      \
         ARGUMENTS: \"{args}\"\n    \
         devices:\n      \
         - /dev/kvm\n      \
         - /dev/net/tun{devs}\n    \
         cap_add:\n      \
         - NET_ADMIN{caps}\n    \
         ports:\n      \
         - \"127.0.0.1:${{WEB_PORT}}:8006\"\n      \
         - \"127.0.0.1:${{RDP_PORT}}:3389/tcp\"\n      \
         - \"127.0.0.1:${{RDP_PORT}}:3389/udp\"\n      \
         - \"127.0.0.1:${{SSH_PORT}}:22/tcp\"{extra}\n    \
         volumes:\n      \
         - ${{STORAGE_DIR}}:/storage\n      \
         - ${{SHARED_DIR}}:/shared\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
    )
}

fn render_linux_iso(
    arguments: &str,
    devs: &str,
    caps: &str,
    extra: &str,
    iso_path: &str,
) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: qemux/qemu\n    \
         container_name: ${{CONTAINER_NAME}}\n    \
         environment:\n      \
         BOOT: \"/boot.iso\"\n      \
         RAM_SIZE: ${{RAM_SIZE}}\n      \
         CPU_CORES: ${{CPU_CORES}}\n      \
         DISK_SIZE: ${{DISK_SIZE}}\n      \
         TZ: ${{TZ}}\n      \
         ARGUMENTS: \"{args}\"\n    \
         devices:\n      \
         - /dev/kvm\n      \
         - /dev/net/tun{devs}\n    \
         cap_add:\n      \
         - NET_ADMIN{caps}\n    \
         ports:\n      \
         - \"127.0.0.1:${{WEB_PORT}}:8006\"\n      \
         - \"127.0.0.1:${{RDP_PORT}}:3389/tcp\"\n      \
         - \"127.0.0.1:${{RDP_PORT}}:3389/udp\"\n      \
         - \"127.0.0.1:${{SSH_PORT}}:22/tcp\"{extra}\n    \
         volumes:\n      \
         - {iso_volume}\n      \
         - ${{STORAGE_DIR}}:/storage\n      \
         - ${{SHARED_DIR}}:/shared\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
        iso_volume = yaml_double_quote(&format!("{iso_path}:/boot.iso:ro")),
    )
}

fn default_arguments() -> String {
    "-rtc base=localtime,clock=host,driftfix=slew".to_string()
}

fn build_gpu_block(bdf: &str) -> (String, String, String) {
    // devices: mount /dev/vfio/vfio and /dev/vfio/<group>
    let group = read_iommu_group(bdf);
    let mut devs = String::new();
    devs.push_str("\n      - /dev/vfio/vfio");
    if let Some(g) = group {
        devs.push_str(&format!("\n      - /dev/vfio/{}", g));
    }
    // SYS_ADMIN is needed for ioctls on vfio groups.
    let caps = "\n      - SYS_ADMIN".to_string();
    // Always ship Code 43 mitigation: the NVIDIA consumer Windows driver
    // refuses to load when it detects a VM. Spoofing kvm=off + hv_vendor_id
    // is harmless for Quadro/datacenter cards but rescues every GeForce.
    let args = format!(
        "-rtc base=localtime,clock=host,driftfix=slew \
         -cpu host,kvm=off,hv_vendor_id=whatever \
         -device vfio-pci,host={bdf},multifunction=on"
    );
    (devs, caps, args)
}

fn read_iommu_group(bdf: &str) -> Option<u32> {
    let link = format!("/sys/bus/pci/devices/{}/iommu_group", bdf);
    let target = std::fs::read_link(Path::new(&link)).ok()?;
    target
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.parse().ok())
}

fn build_extra_ports(spec: &str) -> Result<String> {
    let mut out = String::new();
    for port in validation::parse_extra_ports(spec)? {
        out.push_str(&format!(
            "\n      - \"127.0.0.1:{}:{}/{}\"",
            port.host, port.container, port.proto
        ));
    }
    Ok(out)
}

fn yaml_double_quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_valid_extra_ports() {
        let rendered = build_extra_ports("8080:80,5353:53/udp").unwrap();
        assert!(rendered.contains("\"127.0.0.1:8080:80/tcp\""));
        assert!(rendered.contains("\"127.0.0.1:5353:53/udp\""));
    }

    #[test]
    fn rejects_invalid_extra_ports() {
        assert!(build_extra_ports("bad").is_err());
        assert!(build_extra_ports("8080:80/http").is_err());
    }

    #[test]
    fn quotes_iso_volume() {
        assert_eq!(
            yaml_double_quote("/tmp/Windows ISO/test.iso:/boot.iso:ro"),
            "\"/tmp/Windows ISO/test.iso:/boot.iso:ro\""
        );
    }
}

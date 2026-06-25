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
    let (family, extra_ports, extra_devices, extra_caps, extra_ulimits, arguments, iso_path) =
        if env_path.is_file() {
            let map = env_file::read(&env_path)?;
            let family = ImageFamily::from_env_map(&map);
            let ports = build_extra_ports(env_file::get(&map, "EXTRA_PORTS"))?;
            let gpu_bdf = env_file::get(&map, "GPU_BDF").trim().to_string();
            let (devs, caps, args, ulimits) = if gpu_bdf.is_empty() {
                (
                    String::new(),
                    String::new(),
                    default_arguments(family),
                    String::new(),
                )
            } else {
                build_gpu_block(family, &gpu_bdf)
            };
            let iso = env_file::get(&map, "ISO_PATH").to_string();
            (family, ports, devs, caps, ulimits, args, iso)
        } else {
            (
                ImageFamily::Windows,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                default_arguments(ImageFamily::Windows),
                String::new(),
            )
        };

    let body = match family {
        ImageFamily::Windows => render_windows(
            &arguments,
            &extra_devices,
            &extra_caps,
            &extra_ports,
            &extra_ulimits,
        ),
        ImageFamily::LinuxDistro => render_linux_distro(
            &arguments,
            &extra_devices,
            &extra_caps,
            &extra_ports,
            &extra_ulimits,
        ),
        ImageFamily::LinuxCloud => render_linux_cloud(
            &arguments,
            &extra_devices,
            &extra_caps,
            &extra_ports,
            &extra_ulimits,
        ),
        ImageFamily::LinuxIso => render_linux_iso(
            &arguments,
            &extra_devices,
            &extra_caps,
            &extra_ports,
            &extra_ulimits,
            &iso_path,
        ),
    };

    let path = paths::profile_compose_file(profile);
    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn render_windows(arguments: &str, devs: &str, caps: &str, extra: &str, ulimits: &str) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: {image}\n    \
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
         - ${{OEM_DIR}}:/oem{ulimits}\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        image = paths::IMAGE_WINDOWS,
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
        ulimits = ulimits,
    )
}

fn render_linux_distro(
    arguments: &str,
    devs: &str,
    caps: &str,
    extra: &str,
    ulimits: &str,
) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: {image}\n    \
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
         - ${{SHARED_DIR}}:/shared{ulimits}\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        image = paths::IMAGE_QEMU,
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
        ulimits = ulimits,
    )
}

fn render_linux_cloud(
    arguments: &str,
    devs: &str,
    caps: &str,
    extra: &str,
    ulimits: &str,
) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: {image}\n    \
         container_name: ${{CONTAINER_NAME}}\n    \
         environment:\n      \
         BOOT: \"/storage/boot.qcow2\"\n      \
         BOOT_INDEX: \"1\"\n      \
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
         - ${{SHARED_DIR}}:/shared{ulimits}\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        image = paths::IMAGE_QEMU,
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
        ulimits = ulimits,
    )
}

fn render_linux_iso(
    arguments: &str,
    devs: &str,
    caps: &str,
    extra: &str,
    ulimits: &str,
    iso_path: &str,
) -> String {
    format!(
        "services:\n  \
         winbox:\n    \
         image: {image}\n    \
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
         - ${{SHARED_DIR}}:/shared{ulimits}\n    \
         mem_limit: ${{MEM_LIMIT}}\n    \
         cpus: \"${{CPU_LIMIT}}\"\n    \
         restart: unless-stopped\n    \
         stop_grace_period: 2m\n",
        image = paths::IMAGE_QEMU,
        args = arguments,
        devs = devs,
        caps = caps,
        extra = extra,
        ulimits = ulimits,
        iso_volume = yaml_double_quote(&format!("{iso_path}:/boot.iso:ro")),
    )
}

fn default_arguments(family: ImageFamily) -> String {
    let mut args = "-rtc base=localtime,clock=host,driftfix=slew".to_string();
    if family.is_linux() {
        args.push_str(" -machine graphics=on -device usb-kbd -device virtio-keyboard-pci");
    }
    args
}

fn build_gpu_block(family: ImageFamily, bdf: &str) -> (String, String, String, String) {
    // devices: mount /dev/vfio/vfio and /dev/vfio/<group>
    let group = read_iommu_group(bdf);
    let mut devs = String::new();
    devs.push_str("\n      - /dev/vfio/vfio");
    if let Some(g) = group {
        devs.push_str(&format!("\n      - /dev/vfio/{}", g));
    }
    // SYS_ADMIN: ioctls on vfio groups. IPC_LOCK: VFIO pins all guest RAM
    // via mlock; the dockerd default RLIMIT_MEMLOCK (8 MiB on systemd) makes
    // QEMU exit with vfio_container_dma_map = -12 ENOMEM without it.
    let caps = "\n      - SYS_ADMIN\n      - IPC_LOCK".to_string();
    // Belt and suspenders: even with IPC_LOCK some kernels still honor
    // RLIMIT_MEMLOCK. -1 lifts the soft+hard locked-memory limit so the
    // pinning of guest RAM (RAM_SIZE) succeeds.
    let ulimits = "\n    ulimits:\n      memlock: -1".to_string();
    // Always ship Code 43 mitigation: the NVIDIA consumer Windows driver
    // refuses to load when it detects a VM. Spoofing kvm=off + hv_vendor_id
    // is harmless for Quadro/datacenter cards but rescues every GeForce.
    let args = format!(
        "-rtc base=localtime,clock=host,driftfix=slew \
         -cpu host,kvm=off,hv_vendor_id=whatever \
        -device vfio-pci,host={bdf},multifunction=on{}",
        if family.is_linux() {
            " -machine graphics=on -device usb-kbd -device virtio-keyboard-pci"
        } else {
            ""
        }
    );
    (devs, caps, args, ulimits)
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

    #[test]
    fn gpu_block_emits_memlock_and_ipc_lock() {
        let (devs, caps, args, ulimits) = build_gpu_block(ImageFamily::Windows, "0000:01:00.0");
        assert!(devs.contains("/dev/vfio/vfio"));
        assert!(caps.contains("SYS_ADMIN"));
        assert!(
            caps.contains("IPC_LOCK"),
            "GPU profiles must request IPC_LOCK so QEMU can mlock guest RAM"
        );
        assert!(
            ulimits.contains("memlock: -1"),
            "GPU profiles must lift memlock rlimit; got {ulimits:?}"
        );
        assert!(args.contains("vfio-pci,host=0000:01:00.0"));
    }

    #[test]
    fn linux_qemu_profiles_include_usb_keyboard() {
        assert!(!default_arguments(ImageFamily::Windows).contains("usb-kbd"));

        for family in [
            ImageFamily::LinuxDistro,
            ImageFamily::LinuxIso,
            ImageFamily::LinuxCloud,
        ] {
            assert!(
                default_arguments(family)
                    .contains("-machine graphics=on -device usb-kbd -device virtio-keyboard-pci"),
                "Linux qemux profiles need explicit keyboard devices for noVNC login"
            );
        }
    }

    #[test]
    fn render_functions_pin_image_tags() {
        let win = render_windows("a", "", "", "", "");
        assert!(
            win.contains(paths::IMAGE_WINDOWS) && win.contains(':'),
            "Windows compose must use pinned image; got line with `image:`: {:?}",
            win.lines().find(|l| l.contains("image:"))
        );
        let linux = render_linux_distro("a", "", "", "", "");
        assert!(
            linux.contains(paths::IMAGE_QEMU),
            "Linux distro compose must use pinned qemu image"
        );
        let cloud = render_linux_cloud("a", "", "", "", "");
        assert!(
            cloud.contains(paths::IMAGE_QEMU),
            "Linux cloud compose must use pinned qemu image"
        );
        assert!(cloud.contains("BOOT: \"/storage/boot.qcow2\""));
        assert!(cloud.contains("BOOT_INDEX: \"1\""));
        let iso = render_linux_iso("a", "", "", "", "", "/tmp/x.iso");
        assert!(
            iso.contains(paths::IMAGE_QEMU),
            "Linux ISO compose must use pinned qemu image"
        );
        // Defensive: regression — never emit an unpinned `image: dockurr/windows\n`
        // (the bare repo with no tag implies `:latest`, breaking reproducibility).
        for sample in [&win, &linux, &cloud, &iso] {
            assert!(
                !sample.contains("image: dockurr/windows\n")
                    && !sample.contains("image: qemux/qemu\n"),
                "compose template regressed to unpinned image"
            );
        }
    }

    #[test]
    fn windows_render_with_gpu_includes_ulimits() {
        let with_gpu = render_windows(
            "qemu-args",
            "\n      - /dev/vfio/vfio",
            "\n      - SYS_ADMIN\n      - IPC_LOCK",
            "",
            "\n    ulimits:\n      memlock: -1",
        );
        assert!(with_gpu.contains("ulimits:"));
        assert!(with_gpu.contains("memlock: -1"));
        // ulimits must sit at service indent (4 spaces), between volumes and mem_limit
        let ul_idx = with_gpu.find("ulimits:").unwrap();
        let mem_idx = with_gpu.find("mem_limit:").unwrap();
        assert!(ul_idx < mem_idx);

        let without_gpu = render_windows("qemu-args", "", "", "", "");
        assert!(
            !without_gpu.contains("ulimits:"),
            "non-GPU profiles must not emit a ulimits key"
        );
    }
}

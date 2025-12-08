// backend/src/adapters.rs

// This module will handle interactions with the underlying operating system.
// For example, wrappers for `dladm`, `ipadm`, `zfs`, and `svcadm`.

use std::process::Command;

use crate::core::{NetworkInterface, PackageInfo, SystemInfo};
use crate::error::AppError;

pub fn get_hostname() -> Result<String, std::io::Error> {
    let output = Command::new("hostname").output()?;
    if output.status.success() {
        let raw = String::from_utf8(output.stdout)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()))?;
        Ok(raw.trim().to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to get hostname",
        ))
    }
}

pub fn get_system_info() -> Result<SystemInfo, std::io::Error> {
    let hostname = get_hostname().unwrap_or_else(|_| "unknown".to_string());

    let kernel_version = Command::new("uname")
        .args(["-srv"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let boot_time = Command::new("kstat")
        .args(["-p", "unix:0:system_misc:boot_time"])
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|s| {
            s.split_whitespace()
                .last()
                .and_then(|v| v.parse::<u64>().ok())
        });

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let uptime = boot_time.map(|boot| now.saturating_sub(boot)).unwrap_or(0);

    Ok(SystemInfo {
        hostname,
        kernel_version,
        uptime,
    })
}

pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>, AppError> {
    let output = Command::new("ipadm")
        .args(["show-addr", "-p", "-o", "addrobj,state,addr"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let parsed = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 3 {
                        Some(NetworkInterface {
                            name: parts[0].to_string(),
                            state: parts[1].to_string(),
                            address: parts[2].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            return Ok(parsed);
        }
    }

    // Fallback when ipadm is unavailable (e.g., dev hosts)
    Ok(vec![NetworkInterface {
        name: "net0".to_string(),
        state: "unknown".to_string(),
        address: "0.0.0.0".to_string(),
    }])
}

pub fn get_pkg_list() -> Result<Vec<PackageInfo>, AppError> {
    let output = Command::new("pkg").args(["list", "-Hv"]).output();
    if let Ok(out) = output {
        if out.status.success() {
            let parsed = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        Some(PackageInfo {
                            name: parts[0].to_string(),
                            version: parts[1].to_string(),
                            status: "installed".to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            return Ok(parsed);
        }
    }

    // Fallback when pkg is unavailable (e.g., dev hosts)
    Ok(vec![PackageInfo {
        name: "system/library".to_string(),
        version: "unknown".to_string(),
        status: "unknown".to_string(),
    }])
}

use std::process::Command;

use crate::core::SystemInfo;

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

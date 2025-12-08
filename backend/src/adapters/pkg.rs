use std::process::Command;

use crate::core::PackageInfo;
use crate::error::AppError;

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

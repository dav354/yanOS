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
                    if parts.len() >= 1 {
                        // Parse FMRI: pkg://publisher/category/component@version:timestamp
                        // Example: pkg://omnios/developer/gnu-binutils@2.45-151056.0:20251023T162124Z
                        let fmri = parts[0];
                        let (name, rest) = fmri.split_once('@').unwrap_or((fmri, ""));
                        let (version, build_time) = rest.split_once(':').unwrap_or((rest, ""));

                        Some(PackageInfo {
                            name: name.to_string(),
                            version: version.to_string(),
                            build_time: build_time.to_string(),
                            status: parts.get(1).unwrap_or(&"unknown").to_string(),
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
        build_time: "unknown".to_string(),
        status: "unknown".to_string(),
    }])
}

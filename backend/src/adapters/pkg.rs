use std::collections::HashMap;
use std::process::Command;
use tracing::warn;
use std::thread;
use std::time::Duration;

use crate::core::PackageInfo;
use crate::error::AppError;

fn parse_fmri(fmri: &str) -> (String, String, String) {
    // Parse FMRI: pkg://publisher/category/component@version:timestamp
    // We want to strip the publisher to get a clean name (category/component)
    let (raw_name, rest) = fmri.split_once('@').unwrap_or((fmri, ""));
    
    let no_scheme = raw_name.strip_prefix("pkg://").unwrap_or(raw_name);
    // Strip publisher (everything before first /)
    let name = if let Some((_publisher, clean)) = no_scheme.split_once('/') {
        clean
    } else {
        no_scheme
    };

    let (version, build_time) = rest.split_once(':').unwrap_or((rest, ""));
    
    (name.to_string(), version.to_string(), build_time.to_string())
}

pub fn refresh_catalog() -> Result<(), AppError> {
    let output = Command::new("pkg").arg("refresh").output();
    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            warn!(target: "zos::pkg_adapter", "pkg refresh failed: {}", err);
            Err(AppError::InternalServerError(format!("pkg refresh failed: {}", err)))
        }
        Err(e) => Err(AppError::InternalServerError(format!("pkg refresh execution failed: {}", e))),
    }
}

pub fn get_pkg_list() -> Result<Vec<PackageInfo>, AppError> {
    for attempt in 1..=3 {
        let output = Command::new("pkg").args(["list", "-Hv"]).output();
        match output {
            Ok(out) if out.status.success() => {
                let parsed = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|line| {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 1 {
                            let (name, version, build_time) = parse_fmri(parts[0]);
                            Some(PackageInfo {
                                name,
                                version,
                                build_time,
                                status: parts.get(1).unwrap_or(&"unknown").to_string(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                return Ok(parsed);
            }
            Ok(out) => {
                warn!(
                    target: "zos::pkg_adapter",
                    code = ?out.status.code(),
                    "pkg list failed (attempt {attempt})"
                );
            }
            Err(err) => {
                warn!(
                    target: "zos::pkg_adapter",
                    error = ?err,
                    "pkg list execution failed (attempt {attempt})"
                );
            }
        }

        if attempt < 3 {
            thread::sleep(Duration::from_millis(250));
        }
    }

    Err(AppError::ServiceUnavailable(
        "Failed to list packages via pkg".to_string(),
    ))
}

pub fn get_pkg_updates() -> Result<Vec<PackageInfo>, AppError> {
    // Step 1: Get list of installed packages that have updates available.
    // output: NAME FMRI (of installed version)
    let list_output = Command::new("pkg").args(["list", "-uH", "-o", "name,fmri"]).output();
    
    let mut base_updates: Vec<PackageInfo> = Vec::new();
    let mut names: Vec<String> = Vec::new();

    match list_output {
        Ok(out) if out.status.success() => {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let fmri = parts[1];
                    let (_, version, build_time) = parse_fmri(fmri);
                    
                    names.push(name.clone());
                    base_updates.push(PackageInfo {
                        name,
                        version,
                        build_time,
                        status: "upgrade_available".to_string(),
                    });
                }
            }
        }
        Ok(out) => {
            warn!(
                target: "zos::pkg_adapter",
                code = ?out.status.code(),
                stderr = %String::from_utf8_lossy(&out.stderr),
                "pkg list -u failed"
            );
            return Err(AppError::ServiceUnavailable(
                "Failed to query pkg updates".to_string(),
            ));
        }
        Err(err) => {
            warn!(
                target: "zos::pkg_adapter",
                error = ?err,
                "pkg list -u execution failed"
            );
            return Err(AppError::ServiceUnavailable(
                "Failed to query pkg updates".to_string(),
            ));
        }
    }

    if names.is_empty() {
        return Ok(vec![]);
    }

    // Step 2: Get remote info for these packages to get the NEW version/timestamp
    // This might fail or return partial results. We use a Map to merge.
    let mut remote_info: HashMap<String, PackageInfo> = HashMap::new();
    
    let mut cmd = Command::new("pkg");
    // OmniOS pkg(1) lacks -H for info; use default header and parse lines.
    cmd.args(["info", "-r", "-o", "name,fmri"]);
    cmd.args(&names);

    if let Ok(out) = cmd.output() {
        if !out.status.success() {
            warn!(
                target: "zos::pkg_adapter",
                "pkg info failed (partial results?): code={:?}, stderr={}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            );
        }

        if !out.stdout.is_empty() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0].to_string();
                    let fmri = parts[1];
                    let (_, version, build_time) = parse_fmri(fmri);
                    
                    remote_info.insert(name.clone(), PackageInfo {
                        name,
                        version,
                        build_time,
                        status: "upgrade_available".to_string(),
                    });
                }
            }
        }
    }
    
    // Step 3: Merge. Use remote info if available, else fallback to base (installed) info.
    // This ensures we always show the update in the list, even if we couldn't fetch the new version details.
    let final_updates = base_updates.into_iter().map(|base| {
        if let Some(remote) = remote_info.get(&base.name) {
            remote.clone()
        } else {
            base // Fallback: Shows "Update Available" but with old version/time (no diff arrows)
        }
    }).collect();

    Ok(final_updates)
}

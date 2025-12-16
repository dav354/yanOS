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
                        if !parts.is_empty() {
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

/// Get list of packages with available updates and their NEW version info.
///
/// Returns PackageInfo with the NEW (remote) version/build_time, not the installed one.
/// The frontend compares this against the installed list to show the diff.
pub fn get_pkg_updates() -> Result<Vec<PackageInfo>, AppError> {
    // Step 1: Get list of package names that have updates available.
    // `pkg list -u` shows installed packages with newer versions in the repo.
    let list_output = Command::new("pkg").args(["list", "-uH", "-o", "name"]).output();

    let mut names: Vec<String> = Vec::new();

    match list_output {
        Ok(out) if out.status.success() => {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let name = line.trim();
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
        Ok(out) => {
            // Exit code 4 = no updates available (not an error)
            if out.status.code() == Some(4) {
                return Ok(vec![]);
            }
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

    // Step 2: Get remote (NEW) version info for these packages.
    // `pkg info -r` shows the latest available version in the repository.
    let mut updates: Vec<PackageInfo> = Vec::new();

    let mut cmd = Command::new("pkg");
    cmd.args(["info", "-r"]);
    cmd.args(&names);

    if let Ok(out) = cmd.output() {
        if !out.status.success() {
            warn!(
                target: "zos::pkg_adapter",
                "pkg info -r failed (partial results?): code={:?}, stderr={}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            );
        }

        if !out.stdout.is_empty() {
            let mut current_name: Option<String> = None;
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let line = line.trim();
                if let Some(val) = line.strip_prefix("Name: ") {
                    current_name = Some(val.to_string());
                } else if let Some(val) = line.strip_prefix("FMRI: ")
                    && let Some(name) = current_name.take() {
                        let fmri = val;
                        let (_, version, build_time) = parse_fmri(fmri);

                        updates.push(PackageInfo {
                            name,
                            version,
                            build_time,
                            status: "upgrade_available".to_string(),
                        });
                    }
            }
        }
    }

    // If pkg info -r failed to return info for some packages, they won't be in the list.
    // This is acceptable - we only show updates we can confirm the new version for.
    Ok(updates)
}

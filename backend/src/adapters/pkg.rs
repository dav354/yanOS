//! Package management adapter for illumos IPS.
//!
//! Provides functions to query and manage packages via the pkg(1) command.

use std::process::Command;
use std::thread;
use std::time::Duration;

use tracing::{debug, warn};

use crate::core::PackageInfo;
use crate::error::AppError;

/// Parse an IPS FMRI into (name, version, build_time).
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

/// Refresh the IPS catalog from remote repositories.
pub fn refresh_catalog() -> Result<(), AppError> {
    debug!(target: "yanos::pkg", "Refreshing package catalog");

    let output = Command::new("pkg").arg("refresh").output();
    match output {
        Ok(out) if out.status.success() => {
            debug!(target: "yanos::pkg", "Catalog refresh complete");
            Ok(())
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            warn!(target: "yanos::pkg", stderr = %stderr, "pkg refresh failed");

            // Detect SSL certificate errors and provide actionable message
            if stderr.contains("E_SSL") || stderr.contains("SSL certificate problem") {
                let msg = if stderr.contains("certificate is not yet valid") {
                    "Catalog refresh failed: SSL certificate is not yet valid. \
                     Please verify system time is correct."
                } else if stderr.contains("certificate has expired") {
                    "Catalog refresh failed: SSL certificate has expired. \
                     Consider updating CA certificates."
                } else {
                    "Catalog refresh failed: SSL error. Check network and CA certificates."
                };
                return Err(AppError::ServiceUnavailable(msg.to_string()));
            }

            Err(AppError::InternalServerError(format!("pkg refresh failed: {}", stderr)))
        }
        Err(e) => {
            warn!(target: "yanos::pkg", error = %e, "pkg refresh execution failed");
            Err(AppError::InternalServerError(format!("pkg refresh execution failed: {}", e)))
        }
    }
}

/// Get list of all installed packages.
pub fn get_pkg_list() -> Result<Vec<PackageInfo>, AppError> {
    debug!(target: "yanos::pkg", "Listing installed packages");

    for attempt in 1..=3 {
        let output = Command::new("pkg").args(["list", "-Hv"]).output();
        match output {
            Ok(out) if out.status.success() => {
                let parsed: Vec<PackageInfo> = String::from_utf8_lossy(&out.stdout)
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

                debug!(target: "yanos::pkg", count = parsed.len(), "Package list retrieved");
                return Ok(parsed);
            }
            Ok(out) => {
                warn!(
                    target: "yanos::pkg",
                    code = ?out.status.code(),
                    attempt,
                    "pkg list failed"
                );
            }
            Err(err) => {
                warn!(
                    target: "yanos::pkg",
                    error = ?err,
                    attempt,
                    "pkg list execution failed"
                );
            }
        }

        if attempt < 3 {
            debug!(target: "yanos::pkg", attempt, "Retrying pkg list");
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
    debug!(target: "yanos::pkg", "Checking for package updates");

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
            debug!(target: "yanos::pkg", packages_with_updates = names.len(), "Found packages with updates");
        }
        Ok(out) => {
            // Exit code 4 = no updates available (not an error)
            if out.status.code() == Some(4) {
                debug!(target: "yanos::pkg", "No updates available");
                return Ok(vec![]);
            }
            let stderr = String::from_utf8_lossy(&out.stderr);
            warn!(
                target: "yanos::pkg",
                code = ?out.status.code(),
                stderr = %stderr,
                "pkg list -u failed"
            );

            // Detect SSL certificate errors and provide actionable message
            if stderr.contains("E_SSL") || stderr.contains("SSL certificate problem") {
                let msg = if stderr.contains("certificate is not yet valid") {
                    "Package repository SSL error: certificate is not yet valid. \
                     Please verify system time is correct (check with 'date' command)."
                } else if stderr.contains("certificate has expired") {
                    "Package repository SSL error: certificate has expired. \
                     Consider updating CA certificates or check repository status."
                } else {
                    "Package repository SSL error. Check network connectivity and CA certificates."
                };
                return Err(AppError::ServiceUnavailable(msg.to_string()));
            }

            return Err(AppError::ServiceUnavailable(
                "Failed to query pkg updates".to_string(),
            ));
        }
        Err(err) => {
            warn!(
                target: "yanos::pkg",
                error = ?err,
                "pkg list -u execution failed"
            );
            return Err(AppError::ServiceUnavailable(
                "Failed to query pkg updates".to_string(),
            ));
        }
    }

    if names.is_empty() {
        debug!(target: "yanos::pkg", "No updates available");
        return Ok(vec![]);
    }

    // Step 2: Get remote (NEW) version info for these packages.
    // `pkg info -r` shows the latest available version in the repository.
    debug!(target: "yanos::pkg", count = names.len(), "Fetching remote version info");
    let mut updates: Vec<PackageInfo> = Vec::new();

    let mut cmd = Command::new("pkg");
    cmd.args(["info", "-r"]);
    cmd.args(&names);

    if let Ok(out) = cmd.output() {
        if !out.status.success() {
            warn!(
                target: "yanos::pkg",
                code = ?out.status.code(),
                stderr = %String::from_utf8_lossy(&out.stderr),
                "pkg info -r failed (partial results?)"
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

                        debug!(
                            target: "yanos::pkg",
                            package = %name,
                            new_version = %version,
                            "Found update"
                        );

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

    debug!(target: "yanos::pkg", count = updates.len(), "Update check complete");
    // If pkg info -r failed to return info for some packages, they won't be in the list.
    // This is acceptable - we only show updates we can confirm the new version for.
    Ok(updates)
}

// backend/src/core.rs

// This module contains the shared data structures and types for the project.
// For example, custom error types, and domain-specific models.

pub struct SystemInfo {
    pub hostname: String,
    pub kernel_version: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkInterface {
    pub name: String,
    pub state: String,
    pub address: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub build_time: String,
    pub status: String,
}

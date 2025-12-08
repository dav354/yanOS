// backend/src/core.rs

// This module contains the shared data structures and types for the project.
// For example, custom error types, and domain-specific models.

pub struct SystemInfo {
    pub hostname: String,
    pub kernel_version: String,
    pub uptime: u64,
}

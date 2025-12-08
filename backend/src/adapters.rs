// backend/src/adapters.rs

// This module will handle interactions with the underlying operating system.
// For example, wrappers for `dladm`, `ipadm`, `zfs`, and `svcadm`.

use std::process::Command;

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

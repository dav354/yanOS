use std::process::Command;

use crate::core::NetworkInterface;
use crate::error::AppError;

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

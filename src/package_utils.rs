use crate::config::Package;
use std::process::Command;

/// Search for pacman packages using pacman -Ss
pub fn search_pacman_packages(search_term: &str) -> Result<Vec<Package>, String> {
    let output = Command::new("pacman")
        .args(["-Ss", search_term])
        .output()
        .map_err(|e| format!("Failed to run pacman: {}", e))?;

    if !output.status.success() {
        return Err("pacman command failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut packages = Vec::new();

    let mut lines = stdout.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            continue;
        }

        // Check if this is a package line (starts with repo/)
        if line.contains('/') && !line.starts_with(' ') {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let repo_package = parts[0];
                let version_info = parts[1];

                // Parse repo/package
                if let Some(slash_pos) = repo_package.find('/') {
                    let repo = repo_package[..slash_pos].to_string();
                    let name = repo_package[slash_pos + 1..].to_string();

                    // Parse version (remove [installed] if present)
                    let version = if version_info.contains("[installed]") {
                        version_info.replace("[installed]", "").trim().to_string()
                    } else {
                        version_info.to_string()
                    };

                    let installed = version_info.contains("[installed]");

                    // Get description from next line
                    let description = lines
                        .next()
                        .map(|desc_line| desc_line.trim().to_string())
                        .unwrap_or_else(|| "No description".to_string());

                    packages.push(Package {
                        repo,
                        name,
                        version,
                        installed,
                        description,
                    });
                }
            }
        }
    }

    Ok(packages)
}

/// Search for AUR packages using curl and AUR RPC API
pub fn search_aur_packages(search_term: &str) -> Result<Vec<Package>, String> {
    let url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=search&arg={}",
        search_term
    );

    let output = Command::new("curl")
        .args(["-s", &url])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        return Err("curl command failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON response
    let json: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut packages = Vec::new();

    if let Some(results) = json["results"].as_array() {
        for result in results {
            if let (Some(name), Some(desc), Some(version)) = (
                result["Name"].as_str(),
                result["Description"].as_str(),
                result["Version"].as_str(),
            ) {
                packages.push(Package {
                    repo: "aur".to_string(),
                    name: name.to_string(),
                    version: version.to_string(),
                    installed: false, // AUR packages are never installed on live ISO
                    description: desc.to_string(),
                });
            }
        }
    }

    Ok(packages)
}

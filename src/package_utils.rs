use crate::config::Package;
use crate::process_guard::CommandProcessGroup;
use std::process::Command;

/// Search for pacman packages using pacman -Ss
pub fn search_pacman_packages(search_term: &str) -> Result<Vec<Package>, String> {
    // Validate search term to prevent command injection
    if search_term.contains(";")
        || search_term.contains("|")
        || search_term.contains("&")
        || search_term.contains("$")
        || search_term.contains("`")
        || search_term.contains("\"")
        || search_term.contains("'")
    {
        return Err("Invalid characters in search term".to_string());
    }

    let output = Command::new("pacman")
        .args(["-Ss", search_term])
        .in_new_process_group()
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
    // Validate search term to prevent URL injection
    if search_term.contains(";")
        || search_term.contains("|")
        || search_term.contains("&")
        || search_term.contains("$")
        || search_term.contains("`")
        || search_term.contains("\"")
        || search_term.contains("'")
        || search_term.contains(" ")
    {
        return Err("Invalid characters in search term".to_string());
    }

    let url = format!(
        "https://aur.archlinux.org/rpc/?v=5&type=search&arg={}",
        search_term
    );

    let output = Command::new("curl")
        .args(["-s", &url])
        .in_new_process_group()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_pacman_packages_invalid_command() {
        // Test with an invalid search term that should return no results
        let result = search_pacman_packages("this_package_does_not_exist_12345");
        // In test environments, pacman may not be available, so we accept both success and failure
        match result {
            Ok(packages) => {
                // If successful, should return empty results for non-existent package
                assert!(packages.is_empty());
            }
            Err(_) => {
                // Pacman not available in test environment - this is acceptable
                println!("Pacman not available in test environment (expected)");
            }
        }
    }

    #[test]
    fn test_search_aur_packages_invalid_search() {
        // Test with an invalid search term that should return no results
        let result = search_aur_packages("this_package_does_not_exist_12345");
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_search_aur_packages_network_handling() {
        // Test that AUR search handles network errors gracefully
        // This test may fail if network is unavailable, which is expected
        let result = search_aur_packages("linux");
        // We don't assert success here because network may be unavailable in test environments
        match result {
            Ok(packages) => {
                // If successful, packages should have valid structure
                for package in packages {
                    assert!(!package.name.is_empty());
                    assert!(!package.repo.is_empty());
                    assert_eq!(package.repo, "aur");
                }
            }
            Err(_) => {
                // Network error is acceptable in test environments
                println!("AUR search failed (expected in test environment without network)");
            }
        }
    }
}

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub dependencies: BTreeMap<String, String>,
}

/// Read and parse a japl.toml manifest file.
pub fn read_manifest(path: &str) -> Result<PackageManifest, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path, e))?;
    parse_manifest(&contents)
}

/// Parse the contents of a japl.toml manifest.
fn parse_manifest(input: &str) -> Result<PackageManifest, String> {
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut dependencies = BTreeMap::new();

    enum Section {
        None,
        Package,
        Dependencies,
    }

    let mut section = Section::None;

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Section headers
        if trimmed == "[package]" {
            section = Section::Package;
            continue;
        }
        if trimmed == "[dependencies]" {
            section = Section::Dependencies;
            continue;
        }
        if trimmed.starts_with('[') {
            // Unknown section, skip
            section = Section::None;
            continue;
        }

        // Key = value parsing
        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim();
            let val = trimmed[eq_pos + 1..].trim();
            // Strip surrounding quotes
            let val = val.trim_matches('"');

            match section {
                Section::Package => {
                    if key == "name" {
                        name = Some(val.to_string());
                    } else if key == "version" {
                        version = Some(val.to_string());
                    }
                }
                Section::Dependencies => {
                    dependencies.insert(key.to_string(), val.to_string());
                }
                Section::None => {}
            }
        }
    }

    let name = name.ok_or_else(|| "missing 'name' in [package]".to_string())?;
    let version = version.ok_or_else(|| "missing 'version' in [package]".to_string())?;

    Ok(PackageManifest {
        name,
        version,
        dependencies,
    })
}

/// Create a new japl.toml manifest in the given directory.
pub fn init_manifest(dir: &Path) -> Result<String, String> {
    let manifest_path = dir.join("japl.toml");
    if manifest_path.exists() {
        return Err(format!("japl.toml already exists in {}", dir.display()));
    }

    let dir_name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-project");

    let content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[dependencies]
"#,
        dir_name
    );

    fs::write(&manifest_path, &content)
        .map_err(|e| format!("failed to write japl.toml: {}", e))?;

    Ok(manifest_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let input = r#"
[package]
name = "my-app"
version = "1.2.3"

[dependencies]
http = "0.1.0"
json = "0.2.0"
"#;
        let m = parse_manifest(input).unwrap();
        assert_eq!(m.name, "my-app");
        assert_eq!(m.version, "1.2.3");
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(m.dependencies["http"], "0.1.0");
        assert_eq!(m.dependencies["json"], "0.2.0");
    }

    #[test]
    fn test_parse_no_deps() {
        let input = r#"
[package]
name = "bare"
version = "0.1.0"

[dependencies]
"#;
        let m = parse_manifest(input).unwrap();
        assert_eq!(m.name, "bare");
        assert_eq!(m.dependencies.len(), 0);
    }
}

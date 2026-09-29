use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_PROFILES: &[&str] = &["development", "staging", "production"];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
    pub variables: BTreeMap<String, String>,
}

pub fn profile_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!(".env.{name}"))
}

pub fn profile_names(root: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("cannot read {}", root.display()))? {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if let Some(name) = file_name.strip_prefix(".env.")
            && !name.is_empty()
            && entry.file_type()?.is_file()
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

pub fn load_profile(root: &Path, name: &str) -> Result<Profile> {
    validate_name(name)?;
    let path = profile_path(root, name);
    if !path.is_file() {
        bail!("profile '{name}' does not exist at {}", path.display());
    }
    let variables = dotenvy::from_path_iter(&path)
        .with_context(|| format!("cannot parse {}", path.display()))?
        .collect::<std::result::Result<BTreeMap<_, _>, _>>()
        .with_context(|| format!("invalid variable in {}", path.display()))?;
    Ok(Profile {
        name: name.to_string(),
        path,
        variables,
    })
}

pub fn ensure_profile(root: &Path, name: &str) -> Result<Profile> {
    validate_name(name)?;
    let path = profile_path(root, name);
    if !path.exists() {
        fs::write(&path, "# Environment profile\n")
            .with_context(|| format!("cannot create {}", path.display()))?;
    }
    load_profile(root, name)
}

pub fn set_variable(root: &Path, name: &str, key: &str, value: &str) -> Result<()> {
    validate_key(key)?;
    let profile = ensure_profile(root, name)?;
    let content = fs::read_to_string(&profile.path)?;
    let replacement = format!("{key}={}", quote_value(value));
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim_start().starts_with(&format!("{key}="))
            || line.trim_start().starts_with(&format!("export {key}="))
        {
            lines.push(replacement.clone());
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        if !lines.is_empty() && !lines.last().is_some_and(|line| line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(replacement);
    }
    fs::write(profile.path, lines.join("\n") + "\n")?;
    Ok(())
}

pub fn unset_variable(root: &Path, name: &str, key: &str) -> Result<bool> {
    validate_key(key)?;
    let profile = load_profile(root, name)?;
    let content = fs::read_to_string(&profile.path)?;
    let mut removed = false;
    let lines = content
        .lines()
        .filter(|line| {
            let is_match = line.trim_start().starts_with(&format!("{key}="))
                || line.trim_start().starts_with(&format!("export {key}="));
            removed |= is_match;
            !is_match
        })
        .collect::<Vec<_>>();
    if removed {
        fs::write(profile.path, lines.join("\n") + "\n")?;
    }
    Ok(removed)
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("invalid profile name '{name}'; use letters, numbers, '-' or '_'");
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<()> {
    if key.is_empty()
        || !key
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || (c.is_ascii_alphanumeric() && (i > 0 || !c.is_ascii_digit())))
    {
        bail!("invalid variable name '{key}'");
    }
    Ok(())
}

fn quote_value(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || c == '#' || c == '"')
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discovers_and_loads_profiles() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".env.staging"),
            "API_URL=https://staging\nPORT=4000\n",
        )
        .unwrap();
        fs::write(dir.path().join(".env.example"), "IGNORED=1\n").unwrap();
        assert_eq!(profile_names(dir.path()).unwrap(), ["example", "staging"]);
        assert_eq!(
            load_profile(dir.path(), "staging").unwrap().variables["PORT"],
            "4000"
        );
    }

    #[test]
    fn set_updates_and_unset_removes_variables() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".env.development"),
            "PORT=3000\nDEBUG=true\n",
        )
        .unwrap();
        set_variable(dir.path(), "development", "PORT", "4001").unwrap();
        set_variable(dir.path(), "development", "GREETING", "hello world").unwrap();
        assert_eq!(
            load_profile(dir.path(), "development").unwrap().variables["PORT"],
            "4001"
        );
        assert!(unset_variable(dir.path(), "development", "DEBUG").unwrap());
        assert!(
            !load_profile(dir.path(), "development")
                .unwrap()
                .variables
                .contains_key("DEBUG")
        );
    }

    #[test]
    fn rejects_path_traversal_and_invalid_keys() {
        assert!(validate_name("../secrets").is_err());
        assert!(validate_key("BAD-NAME").is_err());
    }
}

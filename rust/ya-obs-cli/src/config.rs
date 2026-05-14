use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// One [default] or [profiles.<name>] section from the TOML file.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    /// "v4" or "v2".
    pub signing_version: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    default: Profile,
    #[serde(default)]
    profiles: HashMap<String, Profile>,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedConfig {
    pub profile: Profile,
    /// Warning surfaced to the user (e.g. world-readable file with secrets).
    pub warnings: Vec<String>,
}

/// Resolve the config path the CLI should read. Order:
///   1. `$YA_OBS_CONFIG` (explicit override, useful for tests)
///   2. `$XDG_CONFIG_HOME/ya-obs/config.toml`
///   3. `~/.config/ya-obs/config.toml`
pub fn default_config_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("YA_OBS_CONFIG") {
        return Some(PathBuf::from(p));
    }
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(dirs::config_dir)?;
    Some(dir.join("ya-obs").join("config.toml"))
}

/// Load and select a profile. `profile_name = None` selects `[default]`;
/// `Some(name)` selects `[profiles.<name>]` and errors if absent.
pub fn load(path: &Path, profile_name: Option<&str>) -> Result<LoadedConfig, String> {
    if !path.exists() {
        return Ok(LoadedConfig::default());
    }

    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed: RawFile =
        toml::from_str(&raw).map_err(|e| format!("parse {}: {e}", path.display()))?;

    let profile = match profile_name {
        None => parsed.default,
        Some(name) => parsed
            .profiles
            .get(name)
            .cloned()
            .ok_or_else(|| format!("profile '{name}' not found in {}", path.display()))?,
    };

    let mut warnings = Vec::new();
    if (profile.access_key.is_some() || profile.secret_key.is_some())
        && is_world_or_group_readable(path)
    {
        warnings.push(format!(
            "{} contains credentials but is readable by other users; \
             consider `chmod 600 {}`",
            path.display(),
            path.display()
        ));
    }

    Ok(LoadedConfig { profile, warnings })
}

#[cfg(unix)]
fn is_world_or_group_readable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(md) => md.permissions().mode() & 0o077 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_world_or_group_readable(_path: &Path) -> bool {
    false
}

/// Merge two `Option<String>` slots: `cli` wins, then `env`, then `file`.
pub fn pick_first(slots: [Option<String>; 3]) -> Option<String> {
    let [cli, env, file] = slots;
    cli.or(env).or(file)
}

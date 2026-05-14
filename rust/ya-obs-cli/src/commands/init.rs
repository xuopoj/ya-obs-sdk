use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

const TEMPLATE: &str = r#"# ya-obs CLI config file
# Docs: https://github.com/xuopoj/ya-obs-sdk
#
# Precedence (highest first): CLI flag > env var > this file.

[default]
# region = "cn-north-4"
# endpoint = "https://obs.cn-north-4.myhuaweicloud.com"
# signing_version = "v4"

# Credentials are typically set via env vars (HUAWEICLOUD_SDK_AK / _SK) rather
# than this file. If you put them here, keep the file mode at 0600.
# access_key = "..."
# secret_key = "..."

# Named profiles. Select with `--profile <name>` or `$YA_OBS_PROFILE`.
# [profiles.nisco]
# region = "cn-global-1"
# endpoint = "https://obsv3.cloud.nisco.cn"
"#;

pub fn run(path: Option<&str>, force: bool, fallback_default: Option<PathBuf>) -> Result<()> {
    let target: PathBuf = match path {
        Some(p) => PathBuf::from(p),
        None => fallback_default
            .ok_or_else(|| anyhow!("cannot determine default config path; pass --path"))?,
    };

    if target.exists() && !force {
        return Err(anyhow!(
            "{} already exists; pass --force to overwrite",
            target.display()
        ));
    }

    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow!("create {}: {e}", parent.display()))?;
        }
    }

    std::fs::write(&target, TEMPLATE).map_err(|e| anyhow!("write {}: {e}", target.display()))?;
    set_owner_only(&target)?;

    println!("Wrote {}", target.display());
    println!("Edit it, then run `ya-obs ls obs://your-bucket` to get going.");
    Ok(())
}

#[cfg(unix)]
fn set_owner_only(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| anyhow!("chmod {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) -> Result<()> {
    Ok(())
}

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ts_rs::TS;

const RELEASES_URL: &str = "https://api.github.com/repos/damyanp/ghui/releases/latest";

/// Information about an available update.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReleaseInfo {
    /// The release tag, e.g. `"v0.2.0"`.
    pub tag_name: String,
    /// The direct download URL for the NSIS installer exe.
    pub download_url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Checks GitHub for a newer release.
///
/// Returns `None` if the latest release is the same version as the running
/// binary, or if no `.exe` installer asset is found.
pub async fn check_for_update() -> Result<Option<ReleaseInfo>> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("ghui/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let release: GithubRelease = client
        .get(RELEASES_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let current = env!("CARGO_PKG_VERSION");
    let latest = release.tag_name.trim_start_matches('v');

    if latest == current {
        return Ok(None);
    }

    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name.ends_with(".exe"));

    match asset {
        Some(a) => Ok(Some(ReleaseInfo {
            tag_name: release.tag_name,
            download_url: a.browser_download_url,
        })),
        None => Ok(None),
    }
}

/// Downloads the installer exe to a temporary file and returns its path.
pub async fn download_installer(download_url: &str, tag_name: &str) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("ghui/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let bytes = client
        .get(download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let filename = format!("ghui-installer-{}.exe", tag_name);
    let path = std::env::temp_dir().join(filename);
    tokio::fs::write(&path, &bytes).await?;
    Ok(path)
}

/// Spawns the installer interactively and returns immediately.
///
/// The caller is responsible for exiting the app after this returns so the
/// installer can replace the running binary.
pub fn launch_installer(path: &Path) -> Result<()> {
    std::process::Command::new(path)
        .spawn()
        .map_err(|e| anyhow!("failed to launch installer: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_release(tag: &str, assets: Vec<GithubAsset>) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_string(),
            assets,
        }
    }

    fn make_asset(name: &str, url: &str) -> GithubAsset {
        GithubAsset {
            name: name.to_string(),
            browser_download_url: url.to_string(),
        }
    }

    fn extract_release_info(release: GithubRelease, current_version: &str) -> Option<ReleaseInfo> {
        let latest = release.tag_name.trim_start_matches('v');
        if latest == current_version {
            return None;
        }
        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name.ends_with(".exe"))?;
        Some(ReleaseInfo {
            tag_name: release.tag_name,
            download_url: asset.browser_download_url,
        })
    }

    #[test]
    fn test_check_no_update_when_versions_match() {
        let current = env!("CARGO_PKG_VERSION");
        let release = make_release(
            &format!("v{current}"),
            vec![make_asset(
                "ghui_0.2.0_x64-setup.exe",
                "https://example.com/ghui.exe",
            )],
        );
        let result = extract_release_info(release, current);
        assert!(result.is_none());
    }

    #[test]
    fn test_check_update_available() {
        let release = make_release(
            "v9.9.9",
            vec![make_asset(
                "ghui_9.9.9_x64-setup.exe",
                "https://example.com/ghui.exe",
            )],
        );
        let result = extract_release_info(release, "0.2.0");
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.tag_name, "v9.9.9");
        assert_eq!(info.download_url, "https://example.com/ghui.exe");
    }

    #[test]
    fn test_check_no_exe_asset_returns_none() {
        let release = make_release(
            "v9.9.9",
            vec![make_asset(
                "ghui_9.9.9_x64.msi",
                "https://example.com/ghui.msi",
            )],
        );
        let result = extract_release_info(release, "0.2.0");
        assert!(result.is_none());
    }

    #[test]
    fn test_check_no_assets_returns_none() {
        let release = make_release("v9.9.9", vec![]);
        let result = extract_release_info(release, "0.2.0");
        assert!(result.is_none());
    }
}

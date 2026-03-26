use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
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

/// Parses a version string (with optional leading `v`) into a `(major, minor, patch)` tuple.
fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let v = v.trim_start_matches('v');
    let mut parts = v.splitn(3, '.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.split('-').next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Extracts a `ReleaseInfo` from a GitHub release if `release` is newer than
/// `current_version` and contains an NSIS installer asset.
fn extract_release_info(release: GithubRelease, current_version: &str) -> Option<ReleaseInfo> {
    let latest = parse_version(&release.tag_name)?;
    let current = parse_version(current_version)?;
    if latest <= current {
        return None;
    }
    let asset = release
        .assets
        .into_iter()
        .find(|a| a.name.ends_with("-setup.exe"))?;
    Some(ReleaseInfo {
        tag_name: release.tag_name,
        download_url: asset.browser_download_url,
    })
}

/// Checks GitHub for a newer release.
///
/// Returns `None` if the latest release is not newer than the running binary,
/// or if no NSIS installer asset is found.
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

    Ok(extract_release_info(release, env!("CARGO_PKG_VERSION")))
}

/// Downloads the installer exe to a temporary file and returns its path.
///
/// The response body is streamed directly to disk to avoid loading the entire
/// installer into memory at once.
pub async fn download_installer(download_url: &str, tag_name: &str) -> Result<PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("ghui/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let safe_tag = tag_name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "-");
    let filename = format!("ghui-installer-{}.exe", safe_tag);
    let path = std::env::temp_dir().join(filename);
    let mut file = tokio::fs::File::create(&path).await?;
    let mut response = client.get(download_url).send().await?.error_for_status()?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }

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

    #[test]
    fn test_check_older_release_returns_none() {
        let release = make_release(
            "v0.1.0",
            vec![make_asset(
                "ghui_0.1.0_x64-setup.exe",
                "https://example.com/ghui.exe",
            )],
        );
        let result = extract_release_info(release, "0.2.0");
        assert!(result.is_none());
    }

    #[test]
    fn test_non_setup_exe_not_picked() {
        let release = make_release(
            "v9.9.9",
            vec![
                make_asset("helper.exe", "https://example.com/helper.exe"),
                make_asset("ghui_9.9.9_x64-setup.exe", "https://example.com/setup.exe"),
            ],
        );
        let result = extract_release_info(release, "0.2.0");
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().download_url,
            "https://example.com/setup.exe"
        );
    }
}

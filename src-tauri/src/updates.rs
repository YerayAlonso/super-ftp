use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{AppError, AppResult};

const RELEASES_API: &str =
    "https://api.github.com/repos/YerayAlonso/super-ftp/releases?per_page=100";
const RELEASES_PAGE: &str = "https://github.com/YerayAlonso/super-ftp/releases";
const MAX_RESPONSE: usize = 1024 * 1024;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    body: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    prerelease: bool,
    release_notes: Option<String>,
}

fn select_release(releases: &[Release], include_prereleases: bool) -> AppResult<UpdateResult> {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|_| AppError::new("UPDATE", "The application version is invalid."))?;
    Ok(select_release_for_version(
        releases,
        include_prereleases,
        &current,
    ))
}

fn select_release_for_version(
    releases: &[Release],
    include_prereleases: bool,
    current: &semver::Version,
) -> UpdateResult {
    let latest = releases
        .iter()
        .filter_map(|release| {
            if release.draft || (!include_prereleases && release.prerelease) {
                return None;
            }
            let version = semver::Version::parse(
                release
                    .tag_name
                    .strip_prefix('v')
                    .unwrap_or(&release.tag_name),
            )
            .ok()?;
            if !include_prereleases && !version.pre.is_empty() {
                return None;
            }
            Some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0));
    UpdateResult {
        current_version: current.to_string(),
        update_available: latest
            .as_ref()
            .is_some_and(|(version, _)| version > current),
        prerelease: latest
            .as_ref()
            .is_some_and(|(version, release)| release.prerelease || !version.pre.is_empty()),
        release_notes: latest
            .as_ref()
            .and_then(|(_, release)| release.body.as_ref())
            .map(|body| body.chars().take(3000).collect()),
        latest_version: latest.map(|(version, _)| version.to_string()),
    }
}

#[tauri::command]
pub async fn check_updates(include_prereleases: bool) -> AppResult<UpdateResult> {
    let fail = || {
        AppError::new(
            "UPDATE",
            "Could not check GitHub releases. The repository may be private, unavailable, or rate-limited.",
        )
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("super-ftp/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| fail())?;
    let mut response = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| fail())?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE as u64)
    {
        return Err(fail());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| fail())? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE {
            return Err(fail());
        }
        bytes.extend_from_slice(&chunk);
    }
    let releases: Vec<Release> = serde_json::from_slice(&bytes).map_err(|_| fail())?;
    select_release(&releases, include_prereleases)
}

#[tauri::command]
pub async fn open_releases_page() -> AppResult<()> {
    tokio::task::spawn_blocking(|| open::that(RELEASES_PAGE))
        .await
        .map_err(|_| AppError::new("UPDATE", "Could not open the releases page."))?
        .map_err(|_| AppError::new("UPDATE", "Could not open the releases page."))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filters_and_compares_release_versions() {
        let list = [
            Release {
                tag_name: "v0.2.0".into(),
                draft: true,
                prerelease: false,
                body: None,
            },
            Release {
                tag_name: "v0.3.0-beta.1".into(),
                draft: false,
                prerelease: true,
                body: Some("Preview notes".into()),
            },
            Release {
                tag_name: "v0.1.2".into(),
                draft: false,
                prerelease: false,
                body: None,
            },
            Release {
                tag_name: "not-a-version".into(),
                draft: false,
                prerelease: false,
                body: None,
            },
        ];
        let current = semver::Version::parse("0.1.1").unwrap();
        let stable = select_release_for_version(&list, false, &current);
        assert_eq!(stable.latest_version.as_deref(), Some("0.1.2"));
        assert!(stable.update_available);
        assert!(!stable.prerelease);
        let preview = select_release_for_version(&list, true, &current);
        assert_eq!(preview.latest_version.as_deref(), Some("0.3.0-beta.1"));
        assert!(preview.prerelease);
        assert_eq!(preview.release_notes.as_deref(), Some("Preview notes"));
        assert!(
            select_release_for_version(&[], false, &current)
                .latest_version
                .is_none()
        );
        for version in ["0.1.2", "0.1.3"] {
            let current = semver::Version::parse(version).unwrap();
            assert!(!select_release_for_version(&list, false, &current).update_available);
        }
    }
}

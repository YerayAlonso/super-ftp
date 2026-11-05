use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use tauri_plugin_updater::UpdaterExt;

const RELEASES_API: &str =
    "https://api.github.com/repos/YerayAlonso/super-ftp/releases?per_page=100";
const RELEASES_PAGE: &str = "https://github.com/YerayAlonso/super-ftp/releases";
const MAX_RESPONSE: usize = 1024 * 1024;
const MAX_INSTALLER: usize = 100 * 1024 * 1024;
const STABLE_FEED: &str =
    "https://raw.githubusercontent.com/YerayAlonso/super-ftp/main/updates/stable.json";
const PREVIEW_FEED: &str =
    "https://raw.githubusercontent.com/YerayAlonso/super-ftp/main/updates/preview.json";

fn allowed_installer_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .path()
            .starts_with("/YerayAlonso/super-ftp/releases/download/")
}

fn allowed_download_redirect(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && matches!(
            url.host_str(),
            Some(
                "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

async fn signed_update(
    app: &tauri::AppHandle,
    previews: bool,
) -> AppResult<Option<tauri_plugin_updater::Update>> {
    let fail = || {
        AppError::new(
            "UPDATE",
            "Could not check GitHub releases. The repository may be private, unavailable, or rate-limited.",
        )
    };
    let endpoint = if previews { PREVIEW_FEED } else { STABLE_FEED };
    let url = reqwest::Url::parse(endpoint).map_err(|_| fail())?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|_| fail())?
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| fail())?;
    // The updater plugin's network parser has no response size limit. Fetch
    // the fixed manifest ourselves without redirects and bound it before parsing.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("super-ftp/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| fail())?;
    let Some(manifest) = fetch_manifest(&client, endpoint).await? else {
        return Ok(None);
    };
    let update = updater.restore_update(manifest).map_err(|_| fail())?;
    if let Some(update) = &update {
        // Metadata is not authenticated until the artifact signature is verified.
        // Avoid allowing a modified feed to trigger requests to arbitrary hosts.
        if !allowed_installer_url(&update.download_url) {
            return Err(fail());
        }
    }
    Ok(update)
}

async fn fetch_manifest(
    client: &reqwest::Client,
    endpoint: &str,
) -> AppResult<Option<serde_json::Value>> {
    let fail = || {
        AppError::new(
            "UPDATE",
            "Could not check GitHub releases. The repository may be private, unavailable, or rate-limited.",
        )
    };
    let mut response = client.get(endpoint).send().await.map_err(|_| fail())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND
        || response.status() == reqwest::StatusCode::NO_CONTENT
    {
        return Ok(None);
    }
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE as u64)
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
    Ok(Some(serde_json::from_slice(&bytes).map_err(|_| fail())?))
}

#[tauri::command]
pub async fn check_signed_update(
    app: tauri::AppHandle,
    include_prereleases: bool,
) -> AppResult<Option<String>> {
    if !cfg!(target_os = "windows") {
        return Ok(None);
    }
    Ok(signed_update(&app, include_prereleases)
        .await?
        .map(|update| update.version))
}

#[tauri::command]
pub async fn install_signed_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    include_prereleases: bool,
    expected_version: String,
) -> AppResult<()> {
    if !cfg!(target_os = "windows") {
        return Err(AppError::new(
            "UPDATE",
            "Automatic installation is only available on Windows.",
        ));
    }
    let fail = || AppError::new("UPDATE", "Could not install the signed update.");
    let update = signed_update(&app, include_prereleases)
        .await?
        .ok_or_else(fail)?;
    if update.version != expected_version {
        return Err(fail());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() < 5 && allowed_download_redirect(url) {
                attempt.follow()
            } else {
                attempt.error("Untrusted update redirect")
            }
        }))
        .build()
        .map_err(|_| fail())?;
    let bytes = download_verified(&client, &update).await?;
    // Holding this lock prevents enqueue from starting new tasks between the
    // active-transfer check and the Windows updater closing the application.
    let tasks = lock_idle_transfers(&state).await?;
    update.install(bytes).map_err(|_| fail())?;
    drop(tasks);
    Ok(())
}

async fn lock_idle_transfers(
    state: &AppState,
) -> AppResult<
    tokio::sync::MutexGuard<'_, std::collections::HashMap<String, crate::state::TransferTask>>,
> {
    let tasks = state.transfers.lock().await;
    if tasks.values().any(|task| task.transfer.status.is_active()) {
        return Err(AppError::new(
            "UPDATE",
            "Finish or cancel active transfers before installing an update.",
        ));
    }
    Ok(tasks)
}

async fn download_verified(
    client: &reqwest::Client,
    update: &tauri_plugin_updater::Update,
) -> AppResult<Vec<u8>> {
    let fail = || AppError::new("UPDATE", "Could not install the signed update.");
    let mut response = client
        .get(update.download_url.clone())
        .send()
        .await
        .map_err(|_| fail())?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|length| length > MAX_INSTALLER as u64)
    {
        return Err(fail());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| fail())? {
        if bytes.len().saturating_add(chunk.len()) > MAX_INSTALLER {
            return Err(fail());
        }
        bytes.extend_from_slice(&chunk);
    }
    update.verify(&bytes).map_err(|_| fail())?;
    Ok(bytes)
}

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
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    #[tokio::test]
    async fn install_guard_rejects_active_transfers_and_excludes_queue_mutation() {
        use crate::models::{Direction, Transfer, TransferStatus};
        use crate::state::TransferTask;
        use tokio_util::sync::CancellationToken;

        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(crate::storage::Store::load(temp.path().join("config")).unwrap());
        let transfer = Transfer {
            id: "test".into(),
            session_id: "session".into(),
            name: "file".into(),
            direction: Direction::Upload,
            source: "source".into(),
            destination: "destination".into(),
            status: TransferStatus::Queued,
            bytes_transferred: 0,
            total_bytes: 0,
            files_total: 0,
            files_done: 0,
            skipped: 0,
            bytes_per_second: 0,
            error: None,
            error_translation: None,
            conflict: None,
        };
        state.transfers.lock().await.insert(
            "test".into(),
            TransferTask {
                transfer,
                cancel: CancellationToken::new(),
                decision: None,
            },
        );
        for status in [
            TransferStatus::Queued,
            TransferStatus::Preparing,
            TransferStatus::Transferring,
            TransferStatus::AwaitingDecision,
        ] {
            state
                .transfers
                .lock()
                .await
                .get_mut("test")
                .unwrap()
                .transfer
                .status = status;
            assert!(lock_idle_transfers(&state).await.is_err());
        }
        for status in [
            TransferStatus::Completed,
            TransferStatus::Failed,
            TransferStatus::Cancelled,
        ] {
            state
                .transfers
                .lock()
                .await
                .get_mut("test")
                .unwrap()
                .transfer
                .status = status;
            let guard = lock_idle_transfers(&state).await.unwrap();
            // Enqueue uses this same mutex before inserting or spawning work.
            assert!(state.transfers.try_lock().is_err());
            drop(guard);
            assert!(state.transfers.try_lock().is_ok());
        }
    }

    // Public throwaway test vector from tauri-plugin-updater's offline tests
    // (Apache-2.0 OR MIT); it is not the project's signing key.
    const TEST_KEY: &str = "untrusted comment: minisign public key: A49EF4746FA6E6D7\nRWTX5qZvdPSepDGWg2bz8n6ArQnBMwgQK1geVosorcbLzcunD+KJ+cYe";
    const TEST_SIGNATURE: &str = "untrusted comment: signature from tauri secret key\nRUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\ntrusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\nDDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==";

    #[test]
    fn updater_verifies_signature_and_rejects_tampering_and_wrong_version() {
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            serde_json::json!({ "pubkey": STANDARD.encode(TEST_KEY), "requireSignedVersion": true }),
        );
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let updater = app
            .updater_builder()
            .target("test-target")
            .endpoints(vec!["https://example.invalid/update".parse().unwrap()])
            .unwrap()
            .build()
            .unwrap();
        let manifest = serde_json::json!({
            "version": "2.0.0",
            "platforms": { "test-target": {
                "url": "https://github.com/YerayAlonso/super-ftp/releases/download/v2.0.0/app.exe",
                "signature": STANDARD.encode(TEST_SIGNATURE)
            }}
        });
        let mut update = updater.restore_update(manifest.clone()).unwrap().unwrap();
        assert!(update.verify(b"test").is_ok());
        assert!(update.verify(b"tampered").is_err());
        update.signature = STANDARD.encode(TEST_SIGNATURE.replace("file:payload", "file:other"));
        assert!(update.verify(b"test").is_err());
        update.signature = "invalid signature".into();
        assert!(update.verify(b"test").is_err());
        let mut wrong_version = manifest;
        wrong_version["version"] = serde_json::json!("3.0.0");
        let update = updater.restore_update(wrong_version).unwrap().unwrap();
        assert!(update.verify(b"test").is_err());
    }

    #[tokio::test]
    async fn downloaded_bytes_must_match_the_signed_release() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            serde_json::json!({ "pubkey": STANDARD.encode(TEST_KEY), "requireSignedVersion": true }),
        );
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let updater = app
            .updater_builder()
            .target("test-target")
            .endpoints(vec!["https://example.invalid/update".parse().unwrap()])
            .unwrap()
            .build()
            .unwrap();
        for (body, expected) in [(b"test".as_slice(), true), (b"evil".as_slice(), false)] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let body = body.to_vec();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            });
            let manifest = serde_json::json!({ "version": "2.0.0", "platforms": { "test-target": {
                "url": format!("http://{address}/fixture"), "signature": STANDARD.encode(TEST_SIGNATURE)
            }}});
            let update = updater.restore_update(manifest).unwrap().unwrap();
            assert_eq!(
                download_verified(&reqwest::Client::new(), &update)
                    .await
                    .is_ok(),
                expected
            );
            server.join().unwrap();
        }
    }

    #[tokio::test]
    async fn manifest_fetch_handles_missing_invalid_and_oversized_responses() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let cases = [
            (
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                0,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                1,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nbad",
                2,
            ),
            (
                "HTTP/1.1 200 OK\r\nContent-Length: 1048577\r\nConnection: close\r\n\r\n",
                2,
            ),
            (
                "HTTP/1.1 302 Found\r\nLocation: https://example.invalid/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                2,
            ),
        ];
        for (response, expected) in cases {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request).unwrap();
                stream.write_all(response.as_bytes()).unwrap();
            });
            let result = fetch_manifest(&client, &format!("http://{address}/manifest")).await;
            match expected {
                0 => assert!(matches!(result, Ok(None))),
                1 => assert!(matches!(result, Ok(Some(_)))),
                _ => assert!(result.is_err()),
            }
            server.join().unwrap();
        }
    }

    // This launches a real installer and exits the test process. It must never
    // run on a developer machine; the dedicated Windows runner supplies an
    // ephemeral key, an isolated installation and a disposable installer.
    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "installs an update; isolated GitHub Actions runner only"]
    async fn isolated_windows_signed_upgrade() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        assert_eq!(std::env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
        assert_eq!(
            std::env::var("SUPERFTP_ALLOW_INSTALL_TEST").as_deref(),
            Ok("isolated-runner")
        );
        let runner = std::fs::canonicalize(std::env::var("RUNNER_TEMP").unwrap()).unwrap();
        let install =
            std::fs::canonicalize(std::env::var("SUPERFTP_TEST_INSTALL_DIR").unwrap()).unwrap();
        assert!(install.starts_with(&runner) && install != runner);
        let installer =
            std::fs::canonicalize(std::env::var("SUPERFTP_TEST_INSTALLER").unwrap()).unwrap();
        assert!(installer.starts_with(&runner));
        let bytes = std::fs::read(&installer).unwrap();
        let signature = std::fs::read_to_string(installer.with_extension("exe.sig")).unwrap();
        let public_key =
            std::fs::read_to_string(std::env::var("SUPERFTP_TEST_PUBLIC_KEY").unwrap()).unwrap();
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            serde_json::json!({
                "pubkey": public_key.trim(), "requireSignedVersion": true,
                "windows": { "installMode": "quiet" }
            }),
        );
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let updater = app
            .updater_builder()
            .target("test-target")
            .executable_path(install.join("super-ftp.exe"))
            .endpoints(vec!["https://example.invalid/update".parse().unwrap()])
            .unwrap()
            .build()
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0u8; 1024];
            let _ = stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                bytes.len()
            )
            .unwrap();
            stream.write_all(&bytes).unwrap();
        });
        let manifest = serde_json::json!({ "version": "0.1.3", "platforms": { "test-target": {
            "url": format!("http://{address}/fixture"), "signature": signature.trim()
        }}});
        let update = updater
            .restore_update(manifest)
            .unwrap()
            .unwrap()
            .restart_after_install(false);
        let bytes = download_verified(&reqwest::Client::new(), &update)
            .await
            .unwrap();
        server.join().unwrap();
        // Verifies the signature again, then starts NSIS and exits the process.
        update.install(bytes).unwrap();
        panic!("Windows updater unexpectedly returned without exiting");
    }

    #[tokio::test]
    #[ignore = "requires a locally signed installer fixture"]
    async fn project_key_verifies_signed_installer_fixture() {
        let dir = std::env::var("SUPERFTP_SIGNED_FIXTURE_DIR").expect("fixture directory required");
        let version =
            std::env::var("SUPERFTP_SIGNED_FIXTURE_VERSION").unwrap_or_else(|_| "0.1.2".into());
        let mut wrong_version = semver::Version::parse(&version).expect("valid fixture version");
        wrong_version.patch += 1;
        let filename = format!("Super.FTP_{version}_x64-setup.exe");
        let path = std::path::Path::new(&dir).join(&filename);
        let bytes = std::fs::read(&path).expect("fixture installer");
        let signature =
            std::fs::read_to_string(path.with_extension("exe.sig")).expect("fixture signature");
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let pubkey = config["plugins"]["updater"]["pubkey"].as_str().unwrap();
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.config_mut().plugins.0.insert(
            "updater".into(),
            serde_json::json!({ "pubkey": pubkey, "requireSignedVersion": true }),
        );
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        let updater = app
            .updater_builder()
            .target("test-target")
            .endpoints(vec!["https://example.invalid/update".parse().unwrap()])
            .unwrap()
            .build()
            .unwrap();
        let download_url = std::env::var("SUPERFTP_SIGNED_FIXTURE_URL").unwrap_or_else(|_| {
            format!(
                "https://github.com/YerayAlonso/super-ftp/releases/download/v{version}/{filename}"
            )
        });
        let manifest_url = std::env::var("SUPERFTP_SIGNED_FIXTURE_MANIFEST_URL").ok();
        let manifest = if let Some(url) = manifest_url {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap();
            fetch_manifest(&client, &url)
                .await
                .expect("HTTPS manifest retrieval")
                .expect("manifest exists")
        } else {
            serde_json::json!({ "version": version, "platforms": { "test-target": {
                "url": download_url,
                "signature": signature.trim()
            }}})
        };
        let update = updater
            .restore_update(manifest.clone())
            .unwrap()
            .expect("newer fixture");
        assert!(update.verify(&bytes).is_ok());
        // Explicit opt-in: exercise the real public HTTPS release download,
        // without installing anything or publishing a fixture manifest.
        if std::env::var("SUPERFTP_VERIFY_PUBLIC_DOWNLOAD").as_deref() == Ok("true") {
            assert!(allowed_installer_url(&update.download_url));
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(180))
                .redirect(reqwest::redirect::Policy::custom(|attempt| {
                    if attempt.previous().len() < 5 && allowed_download_redirect(attempt.url()) {
                        attempt.follow()
                    } else {
                        attempt.error("Untrusted update redirect")
                    }
                }))
                .build()
                .unwrap();
            let downloaded = download_verified(&client, &update)
                .await
                .expect("verified HTTPS download");
            assert_eq!(
                downloaded, bytes,
                "public artifact differs from local signed fixture"
            );
        }
        let mut modified = bytes.clone();
        modified[0] ^= 1;
        assert!(update.verify(&modified).is_err());
        let mut wrong = manifest;
        wrong["version"] = serde_json::json!(wrong_version.to_string());
        assert!(
            updater
                .restore_update(wrong)
                .unwrap()
                .unwrap()
                .verify(&bytes)
                .is_err()
        );
    }

    #[test]
    fn accepts_only_our_https_release_assets() {
        for (url, expected) in [
            (
                "https://github.com/YerayAlonso/super-ftp/releases/download/v1/app.exe",
                true,
            ),
            (
                "http://github.com/YerayAlonso/super-ftp/releases/download/v1/app.exe",
                false,
            ),
            (
                "https://github.com/other/repo/releases/download/v1/app.exe",
                false,
            ),
            (
                "https://github.com.evil.test/YerayAlonso/super-ftp/releases/download/v1/app.exe",
                false,
            ),
            (
                "https://user@github.com/YerayAlonso/super-ftp/releases/download/v1/app.exe",
                false,
            ),
            (
                "https://github.com:444/YerayAlonso/super-ftp/releases/download/v1/app.exe",
                false,
            ),
        ] {
            assert_eq!(
                allowed_installer_url(&reqwest::Url::parse(url).unwrap()),
                expected,
                "{url}"
            );
        }
    }
    #[test]
    fn redirects_are_restricted_to_https_github_asset_hosts() {
        for (url, expected) in [
            (
                "https://release-assets.githubusercontent.com/asset?token=example",
                true,
            ),
            ("https://objects.githubusercontent.com/asset", true),
            ("http://release-assets.githubusercontent.com/asset", false),
            (
                "https://release-assets.githubusercontent.com.evil.test/asset",
                false,
            ),
            ("https://user@github.com/path", false),
            ("https://github.com:444/path", false),
        ] {
            assert_eq!(
                allowed_download_redirect(&reqwest::Url::parse(url).unwrap()),
                expected,
                "{url}"
            );
        }
    }

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

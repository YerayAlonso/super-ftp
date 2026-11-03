use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use tauri::{AppHandle, Manager, State};
use zeroize::Zeroizing;

use crate::{
    error::{AppError, AppResult},
    models::*,
    paths::{
        is_local_hidden, is_local_link, join_remote, remote_parent, validate_absolute_local,
        validate_leaf, validate_remote_path, validate_remote_target,
    },
    state::AppState,
    storage::{self, validate_connection},
};

#[tauri::command]
pub async fn get_connections(state: State<'_, AppState>) -> AppResult<Vec<Connection>> {
    Ok(state.store.lock().await.config.connections.clone())
}

#[tauri::command]
pub async fn save_connection(
    state: State<'_, AppState>,
    mut connection: Connection,
    secret: Option<String>,
) -> AppResult<Connection> {
    let secret = secret.map(Zeroizing::new);
    validate_connection(&mut connection)?;
    let mut store = state.store.lock().await;
    let previous = store
        .config
        .connections
        .iter()
        .find(|old| old.id == connection.id)
        .cloned();
    if connection.id.is_empty() {
        connection.id = uuid::Uuid::new_v4().to_string();
    } else if previous.is_none() {
        return Err(AppError::new(
            "NOT_FOUND",
            "This connection no longer exists.",
        ));
    }
    if previous.is_none() && store.config.connections.len() >= 200 {
        return Err(AppError::new(
            "LIMIT",
            "You can save up to 200 connections.",
        ));
    }
    let identity_changed = previous.as_ref().is_some_and(|old| {
        old.host != connection.host
            || old.port != connection.port
            || old.username != connection.username
            || old.auth_method != connection.auth_method
            || old.key_path != connection.key_path
    });
    // A saved password must never silently move to a different server/user/key.
    let clear = previous.as_ref().is_some_and(|old| old.remember_secret)
        && (!connection.remember_secret || identity_changed);
    let id = storage::credential_id(&connection);
    let previous_id = previous.as_ref().map(storage::credential_id);
    let remember = connection.remember_secret && connection.auth_method != AuthMethod::Agent;
    connection.remember_secret = remember;
    tokio::task::spawn_blocking(move || -> AppResult<()> {
        if clear && let Some(old_id) = previous_id {
            storage::delete_secret(&old_id)?;
        }
        if remember && let Some(secret) = secret.filter(|secret| !secret.is_empty()) {
            storage::secret_entry(&id)?
                .set_password(secret.as_str())
                .map_err(|_| {
                    AppError::new(
                        "KEYRING",
                        "Could not save the secret in the system credential store.",
                    )
                })?;
        }
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("KEYRING", "The secure credential store did not respond."))??;
    let mut next = store.config.clone();
    if let Some(old) = next
        .connections
        .iter_mut()
        .find(|old| old.id == connection.id)
    {
        *old = connection.clone();
    } else {
        next.connections.push(connection.clone());
    }
    store.persist(next)?;
    Ok(connection)
}

#[tauri::command]
pub async fn delete_connection(state: State<'_, AppState>, connection_id: String) -> AppResult<()> {
    let mut store = state.store.lock().await;
    let connection = store
        .config
        .connections
        .iter()
        .find(|connection| connection.id == connection_id)
        .ok_or_else(|| AppError::new("NOT_FOUND", "The connection does not exist."))?;
    if connection.remember_secret {
        let vault_id = storage::credential_id(connection);
        tokio::task::spawn_blocking(move || storage::delete_secret(&vault_id))
            .await
            .map_err(|_| AppError::new("KEYRING", "Could not delete the secret."))??;
    }
    let mut next = store.config.clone();
    let id = connection.id.clone();
    next.connections.retain(|connection| connection.id != id);
    store.persist(next)
}

#[tauri::command]
pub async fn connect_session(
    state: State<'_, AppState>,
    connection_id: String,
    secret: Option<String>,
) -> AppResult<SessionInfo> {
    let connection = state
        .store
        .lock()
        .await
        .config
        .connections
        .iter()
        .find(|connection| connection.id == connection_id)
        .cloned()
        .ok_or_else(|| AppError::new("NOT_FOUND", "The connection does not exist."))?;
    let mut secret = secret.map(Zeroizing::new);
    if secret.is_none() && connection.remember_secret && connection.auth_method != AuthMethod::Agent
    {
        let id = storage::credential_id(&connection);
        secret = tokio::task::spawn_blocking(move || storage::read_secret(&id))
            .await
            .map_err(|_| {
                AppError::new("KEYRING", "The secure credential store did not respond.")
            })??;
    }
    let session = crate::ssh::connect(&state, connection, secret).await?;
    Ok(session.info.clone())
}

#[tauri::command]
pub async fn trust_host_key(state: State<'_, AppState>, challenge_id: String) -> AppResult<()> {
    let pending = state
        .pending_hosts
        .lock()
        .await
        .remove(&challenge_id)
        .ok_or_else(|| {
            AppError::new(
                "HOST_KEY_EXPIRED",
                "The verification has expired. Connect again.",
            )
        })?;
    if pending.created.elapsed() > Duration::from_secs(300) {
        return Err(AppError::new(
            "HOST_KEY_EXPIRED",
            "The verification has expired. Connect again.",
        ));
    }
    let mut store = state.store.lock().await;
    if let Some(known) = store
        .config
        .known_hosts
        .iter()
        .find(|known| known.host == pending.key.host && known.port == pending.key.port)
    {
        if known.public_key == pending.key.public_key {
            return Ok(());
        }
        return Err(AppError::new(
            "HOST_KEY_CHANGED",
            "A known server key cannot be replaced from this dialog.",
        ));
    }
    let mut next = store.config.clone();
    next.known_hosts.push(pending.key);
    store.persist(next)
}

#[tauri::command]
pub async fn disconnect_session(state: State<'_, AppState>, session_id: String) -> AppResult<()> {
    let session = state.sessions.lock().await.remove(&session_id);
    for task in state.transfers.lock().await.values() {
        if task.transfer.session_id == session_id {
            task.cancel.cancel();
        }
    }
    if let Some(session) = session {
        let _ = session.sftp.close().await;
        let _ = tokio::time::timeout(Duration::from_secs(3), async {
            session
                .ssh
                .lock()
                .await
                .disconnect(russh::Disconnect::ByApplication, "Session closed", "en")
                .await
        })
        .await;
    }
    Ok(())
}

#[tauri::command]
pub async fn list_remote(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> AppResult<DirectoryListing> {
    validate_remote_path(&path)?;
    let session = state.session(&session_id).await?;
    let path = session.sftp.canonicalize(path).await?;
    let mut entries = Vec::new();
    for entry in session.sftp.read_dir(path.clone()).await? {
        let name = entry.file_name();
        if name == "." || name == ".." {
            continue;
        }
        validate_leaf(&name)?;
        let attrs = entry.metadata();
        let kind = if attrs.is_symlink() {
            EntryKind::Symlink
        } else if attrs.is_dir() {
            EntryKind::Directory
        } else if attrs.is_regular() || attrs.permissions.is_none() {
            EntryKind::File
        } else {
            EntryKind::Other
        };
        entries.push(FileEntry {
            path: join_remote(&path, &name)?,
            hidden: name.starts_with('.'),
            name,
            kind,
            size: attrs.size.unwrap_or(0),
            modified: attrs.mtime.map(u64::from),
            permissions: attrs.permissions,
        });
    }
    Ok(DirectoryListing {
        parent: remote_parent(&path),
        path,
        entries,
    })
}

#[tauri::command]
pub async fn get_local_home(app: AppHandle) -> AppResult<String> {
    app.path()
        .home_dir()
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|_| AppError::new("HOME", "The home folder could not be found."))
}

#[tauri::command]
pub async fn list_local(path: String) -> AppResult<DirectoryListing> {
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(path);
        validate_absolute_local(&path)?;
        let path = dunce::canonicalize(path)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let Ok(metadata) = std::fs::symlink_metadata(entry.path()) else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let kind = if is_local_link(&metadata) {
                EntryKind::Symlink
            } else if metadata.is_dir() {
                EntryKind::Directory
            } else if metadata.is_file() {
                EntryKind::File
            } else {
                EntryKind::Other
            };
            #[cfg(unix)]
            let permissions = {
                use std::os::unix::fs::PermissionsExt;
                Some(metadata.permissions().mode())
            };
            #[cfg(not(unix))]
            let permissions = None;
            entries.push(FileEntry {
                path: entry.path().to_string_lossy().into_owned(),
                hidden: is_local_hidden(&name, &metadata),
                name,
                kind,
                size: metadata.len(),
                modified: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|time| time.as_secs()),
                permissions,
            });
        }
        Ok(DirectoryListing {
            parent: path
                .parent()
                .map(|path| path.to_string_lossy().into_owned()),
            path: path.to_string_lossy().into_owned(),
            entries,
        })
    })
    .await
    .map_err(|_| AppError::new("LOCAL", "Could not read the local folder."))?
}

#[tauri::command]
pub async fn create_local_directory(parent: String, name: String) -> AppResult<()> {
    validate_leaf(&name)?;
    let parent = PathBuf::from(parent);
    validate_absolute_local(&parent)?;
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir(parent.join(name))?;
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("LOCAL", "Could not read the local folder."))?
}

#[tauri::command]
pub async fn delete_local(paths: Vec<String>) -> AppResult<()> {
    if paths.len() > 1000 {
        return Err(AppError::new("LIMIT", "Select no more than 1,000 items."));
    }
    // Validate every target before deleting anything; never accept roots or
    // relative paths.
    for path in &paths {
        let path = std::path::Path::new(path);
        validate_absolute_local(path)?;
        if path.file_name().is_none() {
            return Err(AppError::new(
                "ROOT",
                "The root directory cannot be deleted.",
            ));
        }
    }
    tokio::task::spawn_blocking(move || {
        for path in paths {
            let metadata = std::fs::symlink_metadata(&path)?;
            #[cfg(windows)]
            let directory_link = {
                use std::os::windows::fs::MetadataExt;
                is_local_link(&metadata) && metadata.file_attributes() & 0x10 != 0
            };
            #[cfg(not(windows))]
            let directory_link = false;
            if directory_link {
                std::fs::remove_dir(path)?;
            } else if metadata.is_dir() && !is_local_link(&metadata) {
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
        }
        Ok(())
    })
    .await
    .map_err(|_| AppError::new("LOCAL", "Could not read the local folder."))?
}

#[tauri::command]
pub async fn create_remote_directory(
    state: State<'_, AppState>,
    session_id: String,
    parent: String,
    name: String,
) -> AppResult<()> {
    let path = join_remote(&parent, &name)?;
    state
        .session(&session_id)
        .await?
        .sftp
        .create_dir(path)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn rename_remote(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
    new_name: String,
) -> AppResult<()> {
    validate_remote_target(&path)?;
    let parent = remote_parent(&path)
        .ok_or_else(|| AppError::new("ROOT", "The root directory cannot be renamed."))?;
    let destination = join_remote(&parent, &new_name)?;
    let session = state.session(&session_id).await?;
    if session.sftp.try_exists(destination.clone()).await? {
        return Err(AppError::new(
            "EXISTS",
            "An item with this name already exists. It was not overwritten.",
        ));
    }
    session.sftp.rename(path, destination).await?;
    Ok(())
}

#[tauri::command]
pub async fn delete_remote(
    state: State<'_, AppState>,
    session_id: String,
    paths: Vec<String>,
) -> AppResult<()> {
    if paths.len() > 1000 {
        return Err(AppError::new("LIMIT", "Select no more than 1,000 items."));
    }
    for path in &paths {
        validate_remote_target(path)?;
        if remote_parent(path).is_none() {
            return Err(AppError::new(
                "ROOT",
                "The root directory cannot be deleted.",
            ));
        }
    }
    let session = state.session(&session_id).await?;
    delete_remote_paths(&session.sftp, paths).await
}

pub(crate) async fn delete_remote_paths(
    sftp: &russh_sftp::client::SftpSession,
    paths: Vec<String>,
) -> AppResult<()> {
    // Post-order traversal; inspect every entry without following symbolic links.
    let mut stack: Vec<(String, bool, usize)> = paths.into_iter().map(|p| (p, false, 0)).collect();
    let mut visited = 0usize;
    while let Some((path, expanded, depth)) = stack.pop() {
        if expanded {
            sftp.remove_dir(path).await?;
            continue;
        }
        visited += 1;
        if visited > 100_000 || depth > 64 || stack.len() > 100_000 {
            return Err(AppError::new(
                "SCAN_LIMIT",
                "The folder exceeds the limit of 100,000 items or 64 levels.",
            ));
        }
        let metadata = sftp.symlink_metadata(path.clone()).await?;
        if metadata.is_dir() && !metadata.is_symlink() {
            let entries = sftp.read_dir(path.clone()).await?;
            stack.push((path.clone(), true, depth));
            for entry in entries {
                let name = entry.file_name();
                if name == "." || name == ".." {
                    continue;
                }
                stack.push((join_remote(&path, &name)?, false, depth + 1));
                if stack.len() > 100_000 {
                    return Err(AppError::new(
                        "SCAN_LIMIT",
                        "The folder exceeds the limit of 100,000 items or 64 levels.",
                    ));
                }
            }
        } else {
            sftp.remove_file(path).await?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn enqueue_transfers(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    direction: Direction,
    sources: Vec<String>,
    destination: String,
) -> AppResult<Vec<Transfer>> {
    crate::transfer::enqueue(
        app,
        state.inner().clone(),
        session_id,
        direction,
        sources,
        destination,
    )
    .await
}

#[tauri::command]
pub async fn get_transfers(state: State<'_, AppState>) -> AppResult<Vec<Transfer>> {
    Ok(state
        .transfers
        .lock()
        .await
        .values()
        .map(|task| task.transfer.clone())
        .collect())
}

#[tauri::command]
pub async fn cancel_transfer(state: State<'_, AppState>, transfer_id: String) -> AppResult<()> {
    if let Some(task) = state.transfers.lock().await.get(&transfer_id) {
        task.cancel.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn resolve_transfer_conflict(
    state: State<'_, AppState>,
    transfer_id: String,
    conflict_id: String,
    decision: ConflictDecision,
) -> AppResult<()> {
    crate::transfer::resolve_conflict(&state, &transfer_id, &conflict_id, decision).await
}

#[tauri::command]
pub async fn clear_finished_transfers(state: State<'_, AppState>) -> AppResult<()> {
    state
        .transfers
        .lock()
        .await
        .retain(|_, task| task.transfer.status.is_active());
    Ok(())
}

#[cfg(test)]
mod local_operation_tests {
    use super::*;

    #[tokio::test]
    async fn creates_only_a_single_valid_new_folder() {
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().to_string_lossy().into_owned();
        create_local_directory(parent.clone(), "new".into())
            .await
            .unwrap();
        assert!(temp.path().join("new").is_dir());
        assert!(
            create_local_directory(parent.clone(), "new".into())
                .await
                .is_err()
        );
        assert!(
            create_local_directory(parent.clone(), "../escape".into())
                .await
                .is_err()
        );
        assert!(
            create_local_directory("relative".into(), "new".into())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn deletion_is_recursive_and_validates_all_paths_first() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("keep.txt");
        let populated = temp.path().join("populated");
        std::fs::create_dir(&populated).unwrap();
        std::fs::write(populated.join("child"), "safe").unwrap();
        std::fs::write(&file, "safe").unwrap();
        let paths = vec![file.to_string_lossy().into_owned(), "relative".into()];
        assert!(delete_local(paths).await.is_err());
        assert!(file.exists());
        delete_local(vec![populated.to_string_lossy().into_owned()])
            .await
            .unwrap();
        assert!(!populated.exists());
        delete_local(vec![file.to_string_lossy().into_owned()])
            .await
            .unwrap();
        assert!(!file.exists());
        let empty = temp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        delete_local(vec![empty.to_string_lossy().into_owned()])
            .await
            .unwrap();
        assert!(!empty.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn deleting_a_symlink_never_deletes_its_target() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("target");
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("child"), "safe").unwrap();
        let link = temp.path().join("link");
        std::os::unix::fs::symlink(&directory, &link).unwrap();
        delete_local(vec![link.to_string_lossy().into_owned()])
            .await
            .unwrap();
        assert!(!link.exists());
        assert!(directory.join("child").exists());
    }
}

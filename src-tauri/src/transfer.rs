use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{AppError, AppResult},
    models::{ConflictDecision, Direction, Transfer, TransferConflict, TransferStatus},
    paths::{
        is_local_link, join_remote, validate_absolute_local, validate_download_name, validate_leaf,
        validate_remote_path,
    },
    ssh::RemoteSession,
    state::{AppState, TransferTask},
};

const MAX_ENTRIES: usize = 100_000;
const MAX_DEPTH: usize = 64;

struct PlannedFile {
    local: PathBuf,
    remote: String,
    size: u64,
}
struct Plan {
    files: Vec<PlannedFile>,
    directories: Vec<(PathBuf, String)>,
    skipped: u64,
}

pub async fn enqueue(
    app: AppHandle,
    state: AppState,
    session_id: String,
    direction: Direction,
    sources: Vec<String>,
    destination: String,
) -> AppResult<Vec<Transfer>> {
    if sources.is_empty() || sources.len() > 1000 {
        return Err(AppError::new(
            "LIMIT",
            "Select between 1 and 1,000 items to transfer.",
        ));
    }
    let session = state.session(&session_id).await?;
    match direction {
        Direction::Upload => {
            validate_remote_path(&destination)?;
            for source in &sources {
                validate_absolute_local(Path::new(source))?;
            }
        }
        Direction::Download => {
            validate_absolute_local(Path::new(&destination))?;
            for source in &sources {
                validate_remote_path(source)?;
                let name = source
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("");
                validate_download_name(name)?;
            }
        }
    }
    let mut tasks = state.transfers.lock().await;
    if tasks.len() + sources.len() > 2000 {
        return Err(AppError::new(
            "QUEUE_FULL",
            "The queue is full. Clear finished transfers.",
        ));
    }
    let mut added = vec![];
    for source in sources {
        let name = match direction {
            Direction::Upload => Path::new(&source)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("fitxer")
                .to_owned(),
            Direction::Download => source
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("fitxer")
                .to_owned(),
        };
        let transfer = Transfer {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            name,
            direction: direction.clone(),
            source,
            destination: destination.clone(),
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
        let cancel = CancellationToken::new();
        tasks.insert(
            transfer.id.clone(),
            TransferTask {
                transfer: transfer.clone(),
                cancel: cancel.clone(),
                decision: None,
            },
        );
        added.push(transfer.clone());
        let state = state.clone();
        let app = app.clone();
        let session = session.clone();
        tauri::async_runtime::spawn(async move {
            execute(app, state, session, transfer, cancel).await;
        });
    }
    Ok(added)
}

async fn publish(app: Option<&AppHandle>, state: &AppState, transfer: &Transfer) {
    if let Some(task) = state.transfers.lock().await.get_mut(&transfer.id) {
        task.transfer = transfer.clone();
    }
    if let Some(app) = app {
        let _ = app.emit("transfer-update", transfer);
    }
}

async fn execute(
    app: AppHandle,
    state: AppState,
    session: Arc<RemoteSession>,
    mut transfer: Transfer,
    cancel: CancellationToken,
) {
    let result = async {
        // Acquire the per-session slot first, so a busy session cannot
        // occupy all global slots while merely waiting for its own limit.
        let _session_permit = tokio::select! {
            _ = cancel.cancelled() => return Err(AppError::cancelled()),
            permit = session.slots.acquire() => permit.map_err(|_| AppError::cancelled())?,
        };
        let _global_permit = tokio::select! {
            _ = cancel.cancelled() => return Err(AppError::cancelled()),
            permit = state.slots.acquire() => permit.map_err(|_| AppError::cancelled())?,
        };
        check_cancel(&cancel)?;
        run(Some(&app), &state, &session.sftp, &mut transfer, &cancel).await
    }
    .await;
    transfer.bytes_per_second = 0;
    transfer.conflict = None;
    match result {
        Ok(()) => transfer.status = TransferStatus::Completed,
        Err(error) if error.code == "CANCELLED" || cancel.is_cancelled() => {
            transfer.status = TransferStatus::Cancelled
        }
        Err(error) => {
            transfer.status = TransferStatus::Failed;
            transfer.error = Some(error.message);
            transfer.error_translation = error.translation;
        }
    }
    publish(Some(&app), &state, &transfer).await;
}

pub(crate) async fn run(
    app: Option<&AppHandle>,
    state: &AppState,
    sftp: &SftpSession,
    transfer: &mut Transfer,
    cancel: &CancellationToken,
) -> AppResult<()> {
    transfer.status = TransferStatus::Preparing;
    publish(app, state, transfer).await;
    let plan = match transfer.direction {
        Direction::Upload => {
            let source = PathBuf::from(&transfer.source);
            let destination = sftp.canonicalize(transfer.destination.clone()).await?;
            if !sftp.metadata(destination.clone()).await?.is_dir() {
                return Err(AppError::new(
                    "NOT_DIRECTORY",
                    "The remote destination is not a folder.",
                ));
            }
            let token = cancel.clone();
            tokio::task::spawn_blocking(move || plan_upload(&source, &destination, &token))
                .await
                .map_err(|_| AppError::new("SCAN", "Could not prepare the local files."))??
        }
        Direction::Download => {
            let destination = tokio::fs::canonicalize(&transfer.destination).await?;
            if !tokio::fs::metadata(&destination).await?.is_dir() {
                return Err(AppError::new(
                    "NOT_DIRECTORY",
                    "The local destination is not a folder.",
                ));
            }
            plan_download(sftp, &transfer.source, &destination, cancel).await?
        }
    };
    transfer.total_bytes = plan.files.iter().map(|file| file.size).sum();
    transfer.files_total = plan.files.len() as u64;
    transfer.skipped = plan.skipped;
    transfer.status = TransferStatus::Transferring;
    publish(app, state, transfer).await;
    for (local, remote) in &plan.directories {
        check_cancel(cancel)?;
        match transfer.direction {
            Direction::Upload => ensure_remote_dir(sftp, remote).await?,
            Direction::Download => ensure_local_dir(local).await?,
        }
    }
    let started = Instant::now();
    let mut last_update = Instant::now();
    for file in &plan.files {
        check_cancel(cancel)?;
        match transfer.direction {
            Direction::Upload => {
                if !upload_file(
                    app,
                    state,
                    sftp,
                    file,
                    transfer,
                    cancel,
                    started,
                    &mut last_update,
                )
                .await?
                {
                    transfer.skipped += 1;
                    transfer.total_bytes = transfer.total_bytes.saturating_sub(file.size);
                }
            }
            Direction::Download => {
                if !download_file(
                    app,
                    state,
                    sftp,
                    file,
                    transfer,
                    cancel,
                    started,
                    &mut last_update,
                )
                .await?
                {
                    transfer.skipped += 1;
                    transfer.total_bytes = transfer.total_bytes.saturating_sub(file.size);
                }
            }
        }
        transfer.files_done += 1;
        publish(app, state, transfer).await;
    }
    Ok(())
}

pub async fn resolve_conflict(
    state: &AppState,
    transfer_id: &str,
    conflict_id: &str,
    decision: ConflictDecision,
) -> AppResult<()> {
    if let ConflictDecision::Rename { name } = &decision {
        validate_download_name(name)?;
    }
    let mut tasks = state.transfers.lock().await;
    let task = tasks.get_mut(transfer_id).ok_or_else(|| {
        AppError::new("CONFLICT_EXPIRED", "This decision is no longer available.")
    })?;
    if task
        .decision
        .as_ref()
        .is_none_or(|(id, _)| id != conflict_id)
        || task.cancel.is_cancelled()
    {
        return Err(AppError::new(
            "CONFLICT_EXPIRED",
            "This decision is no longer available.",
        ));
    }
    let (_, sender) = task.decision.take().unwrap();
    sender
        .send(decision)
        .map_err(|_| AppError::new("CONFLICT_EXPIRED", "This decision is no longer available."))
}

#[allow(clippy::too_many_arguments)]
async fn ask_conflict(
    app: Option<&AppHandle>,
    state: &AppState,
    transfer: &mut Transfer,
    file: &PlannedFile,
    destination: &str,
    size: Option<u64>,
    modified: Option<u64>,
    cancel: &CancellationToken,
) -> AppResult<ConflictDecision> {
    let name = match transfer.direction {
        Direction::Upload => destination.rsplit('/').next().unwrap_or("").to_owned(),
        Direction::Download => Path::new(destination)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned(),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let conflict = TransferConflict {
        id: id.clone(),
        source: match transfer.direction {
            Direction::Upload => file.local.to_string_lossy().into_owned(),
            Direction::Download => file.remote.clone(),
        },
        destination: destination.into(),
        name,
        direction: transfer.direction.clone(),
        source_size: file.size,
        destination_size: size,
        destination_modified: modified,
    };
    transfer.status = TransferStatus::AwaitingDecision;
    transfer.bytes_per_second = 0;
    transfer.conflict = Some(conflict);
    {
        let mut tasks = state.transfers.lock().await;
        let task = tasks.get_mut(&transfer.id).ok_or_else(|| {
            AppError::parameterized(
                "EXISTS",
                "“{path}” already exists. It was not overwritten.",
                "path",
                destination,
            )
        })?;
        task.decision = Some((id, sender));
        task.transfer = transfer.clone();
    }
    if let Some(app) = app {
        let _ = app.emit("transfer-update", transfer.clone());
    }
    let decision = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(AppError::cancelled()),
        result = receiver => result.map_err(|_| AppError::cancelled()),
    };
    if let Some(task) = state.transfers.lock().await.get_mut(&transfer.id) {
        task.decision = None;
    }
    transfer.conflict = None;
    transfer.status = TransferStatus::Transferring;
    publish(app, state, transfer).await;
    match decision? {
        ConflictDecision::Cancel => Err(AppError::cancelled()),
        value => Ok(value),
    }
}

fn check_cancel(cancel: &CancellationToken) -> AppResult<()> {
    if cancel.is_cancelled() {
        Err(AppError::cancelled())
    } else {
        Ok(())
    }
}

fn plan_upload(source: &Path, destination: &str, cancel: &CancellationToken) -> AppResult<Plan> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AppError::new(
                "INVALID_NAME",
                "This root or a non-UTF-8 name cannot be transferred.",
            )
        })?;
    let mut stack = vec![(source.to_owned(), join_remote(destination, name)?, 0usize)];
    let mut plan = Plan {
        files: vec![],
        directories: vec![],
        skipped: 0,
    };
    while let Some((local, remote, depth)) = stack.pop() {
        check_cancel(cancel)?;
        if depth > MAX_DEPTH
            || plan.files.len() + plan.directories.len() + stack.len() >= MAX_ENTRIES
        {
            return Err(AppError::new(
                "SCAN_LIMIT",
                "The folder exceeds the limit of 100,000 items or 64 levels.",
            ));
        }
        let metadata = std::fs::symlink_metadata(&local)?;
        if is_local_link(&metadata) {
            plan.skipped += 1;
            continue;
        }
        if metadata.is_dir() {
            plan.directories.push((local.clone(), remote.clone()));
            for entry in std::fs::read_dir(local)? {
                let entry = entry?;
                let name = entry.file_name().into_string().map_err(|_| {
                    AppError::new("INVALID_NAME", "A filename that is not UTF-8 was found.")
                })?;
                stack.push((entry.path(), join_remote(&remote, &name)?, depth + 1));
                if stack.len() + plan.files.len() + plan.directories.len() > MAX_ENTRIES {
                    return Err(AppError::new(
                        "SCAN_LIMIT",
                        "The folder exceeds the limit of 100,000 items.",
                    ));
                }
            }
        } else if metadata.is_file() {
            plan.files.push(PlannedFile {
                local,
                remote,
                size: metadata.len(),
            });
        } else {
            plan.skipped += 1;
        }
    }
    Ok(plan)
}

async fn plan_download(
    sftp: &SftpSession,
    source: &str,
    destination: &Path,
    cancel: &CancellationToken,
) -> AppResult<Plan> {
    let name = source
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("");
    validate_download_name(name)?;
    let mut stack = vec![(destination.join(name), source.to_owned(), 0usize)];
    let mut plan = Plan {
        files: vec![],
        directories: vec![],
        skipped: 0,
    };
    while let Some((local, remote, depth)) = stack.pop() {
        check_cancel(cancel)?;
        if depth > MAX_DEPTH
            || plan.files.len() + plan.directories.len() + stack.len() >= MAX_ENTRIES
        {
            return Err(AppError::new(
                "SCAN_LIMIT",
                "The folder exceeds the limit of 100,000 items or 64 levels.",
            ));
        }
        let metadata = sftp.symlink_metadata(remote.clone()).await?;
        if metadata.is_symlink() {
            plan.skipped += 1;
            continue;
        }
        if metadata.is_dir() {
            plan.directories.push((local.clone(), remote.clone()));
            for entry in sftp.read_dir(remote.clone()).await? {
                let name = entry.file_name();
                if name == "." || name == ".." {
                    continue;
                }
                validate_download_name(&name)?;
                stack.push((local.join(&name), join_remote(&remote, &name)?, depth + 1));
                if stack.len() + plan.files.len() + plan.directories.len() > MAX_ENTRIES {
                    return Err(AppError::new(
                        "SCAN_LIMIT",
                        "The folder exceeds the limit of 100,000 items.",
                    ));
                }
            }
        } else if metadata.is_regular() || metadata.permissions.is_none() {
            plan.files.push(PlannedFile {
                local,
                remote,
                size: metadata.size.unwrap_or(0),
            });
        } else {
            plan.skipped += 1;
        }
    }
    Ok(plan)
}

async fn ensure_remote_dir(sftp: &SftpSession, path: &str) -> AppResult<()> {
    if sftp.try_exists(path).await? {
        let attrs = sftp.symlink_metadata(path).await?;
        if !attrs.is_dir() || attrs.is_symlink() {
            return Err(AppError::new(
                "EXISTS",
                "The destination folder is a file or symbolic link.",
            ));
        }
    } else {
        sftp.create_dir(path).await?;
    }
    Ok(())
}

async fn ensure_local_dir(path: &Path) -> AppResult<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(attrs) if attrs.is_dir() && !is_local_link(&attrs) => Ok(()),
        Ok(_) => Err(AppError::new(
            "UNSAFE_DESTINATION",
            "The destination is a file or symbolic link. It was not modified.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tokio::fs::create_dir(path).await?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn copy_progress<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    app: Option<&AppHandle>,
    state: &AppState,
    reader: &mut R,
    writer: &mut W,
    transfer: &mut Transfer,
    cancel: &CancellationToken,
    started: Instant,
    last_update: &mut Instant,
) -> AppResult<()> {
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let count = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(AppError::cancelled()),
            read = reader.read(&mut buffer) => read?,
        };
        if count == 0 {
            break;
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(AppError::cancelled()),
            result = writer.write_all(&buffer[..count]) => result?,
        }
        transfer.bytes_transferred += count as u64;
        transfer.bytes_per_second =
            (transfer.bytes_transferred as f64 / started.elapsed().as_secs_f64().max(0.001)) as u64;
        if last_update.elapsed() >= Duration::from_millis(120) {
            publish(app, state, transfer).await;
            *last_update = Instant::now();
        }
    }
    writer.flush().await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn upload_file(
    app: Option<&AppHandle>,
    state: &AppState,
    sftp: &SftpSession,
    file: &PlannedFile,
    transfer: &mut Transfer,
    cancel: &CancellationToken,
    started: Instant,
    last_update: &mut Instant,
) -> AppResult<bool> {
    let mut destination = file.remote.clone();
    let Some(mut expected) =
        choose_remote_destination(app, state, sftp, file, transfer, cancel, &mut destination)
            .await?
    else {
        return Ok(false);
    };
    let parent = file
        .remote
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("/");
    let temporary = format!("{parent}/.superftp-{}.part", uuid::Uuid::new_v4());
    let result: AppResult<bool> = async {
        let mut input = tokio::fs::File::open(&file.local).await?;
        let mut output = sftp
            .open_with_flags(
                temporary.clone(),
                OpenFlags::CREATE | OpenFlags::EXCLUDE | OpenFlags::WRITE,
            )
            .await?;
        let copied = copy_progress(
            app,
            state,
            &mut input,
            &mut output,
            transfer,
            cancel,
            started,
            last_update,
        )
        .await;
        let closed = output.close().await;
        copied?;
        closed?;
        check_cancel(cancel)?;
        loop {
            let current = remote_file_stamp(sftp, &destination).await?;
            if current == expected {
                break;
            }
            let Some(next) = choose_remote_destination(
                app,
                state,
                sftp,
                file,
                transfer,
                cancel,
                &mut destination,
            )
            .await?
            else {
                return Ok(false);
            };
            expected = next;
        }
        if expected.is_some() {
            // SFTP v3 has no portable atomic replace. Preserve the original in
            // a unique sibling first, and restore it if publication fails.
            let backup = format!("{parent}/.superftp-{}.backup", uuid::Uuid::new_v4());
            sftp.rename(destination.clone(), backup.clone()).await?;
            if let Err(error) = sftp.rename(temporary.clone(), destination.clone()).await {
                if sftp
                    .rename(backup.clone(), destination.clone())
                    .await
                    .is_err()
                {
                    return Err(AppError::parameterized(
                        "BACKUP",
                        "The original could not be restored. Backup: {path}",
                        "path",
                        backup,
                    ));
                }
                return Err(error.into());
            }
            // If cleanup fails, leave the backup rather than destroy user data.
            let _ = sftp.remove_file(backup).await;
        } else {
            sftp.rename(temporary.clone(), destination.clone()).await?;
        }
        Ok(true)
    }
    .await;
    if !matches!(&result, Ok(true)) {
        let _ = sftp.remove_file(temporary).await;
    }
    result
}

#[allow(clippy::too_many_arguments)]
async fn download_file(
    app: Option<&AppHandle>,
    state: &AppState,
    sftp: &SftpSession,
    file: &PlannedFile,
    transfer: &mut Transfer,
    cancel: &CancellationToken,
    started: Instant,
    last_update: &mut Instant,
) -> AppResult<bool> {
    let mut destination = file.local.clone();
    let Some(mut expected) =
        choose_local_destination(app, state, file, transfer, cancel, &mut destination).await?
    else {
        return Ok(false);
    };
    let parent = file
        .local
        .parent()
        .ok_or_else(|| AppError::new("PATH", "The destination has no parent folder."))?;
    // NamedTempFile is mode 0600 on Unix and deletes itself on failure.
    // Persisting with no-clobber also protects against a destination race.
    let temporary = tempfile::Builder::new()
        .prefix(".superftp-")
        .suffix(".part")
        .tempfile_in(parent)?;
    let mut output = tokio::fs::File::from_std(temporary.reopen()?);
    let mut input = sftp.open(file.remote.clone()).await?;
    let result = copy_progress(
        app,
        state,
        &mut input,
        &mut output,
        transfer,
        cancel,
        started,
        last_update,
    )
    .await;
    let closed = input.close().await;
    result?;
    closed?;
    output.sync_all().await?;
    drop(output);
    check_cancel(cancel)?;
    loop {
        let current = local_file_stamp(&destination).await?;
        if current == expected {
            break;
        }
        let Some(next) =
            choose_local_destination(app, state, file, transfer, cancel, &mut destination).await?
        else {
            return Ok(false);
        };
        expected = next;
    }
    if expected.is_some() {
        temporary
            .persist(&destination)
            .map_err(|error| AppError::from(error.error))?;
    } else {
        temporary
            .persist_noclobber(&destination)
            .map_err(|error| AppError::from(error.error))?;
    }
    Ok(true)
}

type Stamp = Option<(u64, Option<u64>)>;

async fn remote_file_stamp(sftp: &SftpSession, path: &str) -> AppResult<Stamp> {
    match sftp.symlink_metadata(path).await {
        Ok(attrs) if attrs.is_regular() && !attrs.is_symlink() => {
            Ok(Some((attrs.size.unwrap_or(0), attrs.mtime.map(u64::from))))
        }
        Ok(_) => Err(AppError::new(
            "UNSAFE_DESTINATION",
            "The destination is a file or symbolic link. It was not modified.",
        )),
        Err(russh_sftp::client::error::Error::Status(status))
            if status.status_code == russh_sftp::protocol::StatusCode::NoSuchFile =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

async fn local_file_stamp(path: &Path) -> AppResult<Stamp> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(attrs) if attrs.is_file() && !is_local_link(&attrs) => Ok(Some((
            attrs.len(),
            attrs
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos().min(u64::MAX as u128) as u64),
        ))),
        Ok(_) => Err(AppError::new(
            "UNSAFE_DESTINATION",
            "The destination is a file or symbolic link. It was not modified.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn choose_remote_destination(
    app: Option<&AppHandle>,
    state: &AppState,
    sftp: &SftpSession,
    file: &PlannedFile,
    transfer: &mut Transfer,
    cancel: &CancellationToken,
    destination: &mut String,
) -> AppResult<Option<Stamp>> {
    loop {
        check_cancel(cancel)?;
        let stamp = remote_file_stamp(sftp, destination).await?;
        let Some((size, modified)) = stamp else {
            return Ok(Some(None));
        };
        match ask_conflict(
            app,
            state,
            transfer,
            file,
            destination,
            Some(size),
            modified,
            cancel,
        )
        .await?
        {
            ConflictDecision::Overwrite => return Ok(Some(stamp)),
            ConflictDecision::Skip => return Ok(None),
            ConflictDecision::Rename { name } => {
                validate_leaf(&name)?;
                let parent = destination
                    .rsplit_once('/')
                    .map(|(parent, _)| if parent.is_empty() { "/" } else { parent })
                    .unwrap_or("/");
                *destination = join_remote(parent, &name)?;
            }
            ConflictDecision::Cancel => return Err(AppError::cancelled()),
        }
    }
}

async fn choose_local_destination(
    app: Option<&AppHandle>,
    state: &AppState,
    file: &PlannedFile,
    transfer: &mut Transfer,
    cancel: &CancellationToken,
    destination: &mut PathBuf,
) -> AppResult<Option<Stamp>> {
    loop {
        check_cancel(cancel)?;
        let stamp = local_file_stamp(destination).await?;
        let Some((size, modified)) = stamp else {
            return Ok(Some(None));
        };
        let modified_seconds = modified.map(|time| time / 1_000_000_000);
        match ask_conflict(
            app,
            state,
            transfer,
            file,
            &destination.to_string_lossy(),
            Some(size),
            modified_seconds,
            cancel,
        )
        .await?
        {
            ConflictDecision::Overwrite => return Ok(Some(stamp)),
            ConflictDecision::Skip => return Ok(None),
            ConflictDecision::Rename { name } => {
                validate_download_name(&name)?;
                *destination = destination
                    .parent()
                    .ok_or_else(|| AppError::new("PATH", "The destination has no parent folder."))?
                    .join(name);
            }
            ConflictDecision::Cancel => return Err(AppError::cancelled()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_plan_preserves_relative_tree_and_empty_directories() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("project");
        std::fs::create_dir_all(source.join("empty")).unwrap();
        std::fs::write(source.join("hello.txt"), "hello").unwrap();
        let plan = plan_upload(&source, "/uploads", &CancellationToken::new()).unwrap();
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].remote, "/uploads/project/hello.txt");
        assert_eq!(plan.files[0].size, 5);
        assert_eq!(plan.directories.len(), 2);
    }

    #[test]
    fn cancelled_scan_does_not_read_source() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = plan_upload(Path::new("missing.txt"), "/uploads", &cancel);
        assert_eq!(result.err().unwrap().code, "CANCELLED");
    }

    #[tokio::test]
    async fn refuses_to_merge_a_directory_into_a_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file");
        std::fs::write(&file, "keep me").unwrap();
        assert!(ensure_local_dir(&file).await.is_err());
        assert_eq!(std::fs::read_to_string(file).unwrap(), "keep me");
    }
}

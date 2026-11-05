//! Real loopback SSH + SFTP tests. No internet, GUI, user credentials, or
//! persistent keyring entries are involved; the server is an in-memory fixture.
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use russh::{
    Channel, ChannelId,
    keys::{Algorithm, HashAlg, PrivateKey, PublicKeyBase64},
    server::{self, Auth, Msg, Server as _, Session},
};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

use crate::{models::*, state::AppState, storage::Store};

#[derive(Default)]
struct MemoryFs {
    files: HashMap<String, Vec<u8>>,
    dirs: HashSet<String>,
    cancel_on_write: Option<CancellationToken>,
}

#[derive(Clone)]
struct Fixture {
    fs: Arc<Mutex<MemoryFs>>,
    auth_attempts: Arc<AtomicUsize>,
}

impl server::Server for Fixture {
    type Handler = SshFixture;
    fn new_client(&mut self, _: Option<SocketAddr>) -> Self::Handler {
        SshFixture {
            fixture: self.clone(),
            channels: HashMap::new(),
        }
    }
}

struct SshFixture {
    fixture: Fixture,
    channels: HashMap<ChannelId, Channel<Msg>>,
}

impl server::Handler for SshFixture {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        self.fixture.auth_attempts.fetch_add(1, Ordering::SeqCst);
        Ok(if user == "test" && password == "test-only" {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name != "sftp" {
            session.channel_failure(id)?;
            return Ok(());
        }
        let channel = self.channels.remove(&id).unwrap();
        session.channel_success(id)?;
        let sftp = SftpFixture {
            fs: self.fixture.fs.clone(),
            read_dirs: HashSet::new(),
        };
        tokio::spawn(async move {
            russh_sftp::server::run(channel.into_stream(), sftp).await;
        });
        Ok(())
    }
}

struct SftpFixture {
    fs: Arc<Mutex<MemoryFs>>,
    read_dirs: HashSet<String>,
}

fn status(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: String::new(),
        language_tag: "en".into(),
    }
}

impl SftpFixture {
    async fn attrs(&self, id: u32, path: &str) -> Result<Attrs, StatusCode> {
        let fs = self.fs.lock().await;
        let mut attrs = FileAttributes::empty();
        if fs.dirs.contains(path) {
            attrs.permissions = Some(0o040755);
        } else if let Some(data) = fs.files.get(path) {
            attrs.permissions = Some(0o100644);
            attrs.size = Some(data.len() as u64);
        } else {
            return Err(StatusCode::NoSuchFile);
        }
        Ok(Attrs { id, attrs })
    }
}

impl russh_sftp::server::Handler for SftpFixture {
    type Error = StatusCode;
    fn unimplemented(&self) -> StatusCode {
        StatusCode::OpUnsupported
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, StatusCode> {
        let path = if path == "." { "/data".into() } else { path };
        self.attrs(id, &path).await?;
        Ok(Name {
            id,
            files: vec![File::dummy(path)],
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
        self.attrs(id, &path).await
    }
    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
        self.attrs(id, &path).await
    }
    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, StatusCode> {
        self.attrs(id, &handle).await
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, StatusCode> {
        if !self.fs.lock().await.dirs.contains(&path) {
            return Err(StatusCode::NoSuchFile);
        }
        self.read_dirs.remove(&path);
        Ok(Handle { id, handle: path })
    }

    async fn readdir(&mut self, id: u32, path: String) -> Result<Name, StatusCode> {
        if !self.read_dirs.insert(path.clone()) {
            return Err(StatusCode::Eof);
        }
        let fs = self.fs.lock().await;
        let prefix = format!("{}/", path.trim_end_matches('/'));
        let mut files = vec![];
        for (name, data) in &fs.files {
            if let Some(leaf) = name
                .strip_prefix(&prefix)
                .filter(|leaf| !leaf.contains('/'))
            {
                let mut attrs = FileAttributes::empty();
                attrs.permissions = Some(0o100644);
                attrs.size = Some(data.len() as u64);
                files.push(File::new(leaf, attrs));
            }
        }
        for name in &fs.dirs {
            if let Some(leaf) = name
                .strip_prefix(&prefix)
                .filter(|leaf| !leaf.is_empty() && !leaf.contains('/'))
            {
                let mut attrs = FileAttributes::empty();
                attrs.permissions = Some(0o040755);
                files.push(File::new(leaf, attrs));
            }
        }
        Ok(Name { id, files })
    }

    async fn open(
        &mut self,
        id: u32,
        path: String,
        flags: OpenFlags,
        _: FileAttributes,
    ) -> Result<Handle, StatusCode> {
        let mut fs = self.fs.lock().await;
        if flags.contains(OpenFlags::EXCLUDE) && fs.files.contains_key(&path) {
            return Err(StatusCode::Failure);
        }
        if flags.contains(OpenFlags::CREATE) {
            fs.files.entry(path.clone()).or_default();
        }
        if !fs.files.contains_key(&path) {
            return Err(StatusCode::NoSuchFile);
        }
        Ok(Handle { id, handle: path })
    }

    async fn close(&mut self, id: u32, _: String) -> Result<Status, StatusCode> {
        Ok(status(id))
    }

    async fn read(
        &mut self,
        id: u32,
        path: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, StatusCode> {
        let fs = self.fs.lock().await;
        let bytes = fs.files.get(&path).ok_or(StatusCode::NoSuchFile)?;
        let start = offset as usize;
        if start >= bytes.len() {
            return Err(StatusCode::Eof);
        }
        Ok(Data {
            id,
            data: bytes[start..(start + len as usize).min(bytes.len())].to_vec(),
        })
    }

    async fn write(
        &mut self,
        id: u32,
        path: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, StatusCode> {
        let mut fs = self.fs.lock().await;
        let bytes = fs.files.get_mut(&path).ok_or(StatusCode::NoSuchFile)?;
        let end = offset as usize + data.len();
        if end > 8 * 1024 * 1024 {
            return Err(StatusCode::Failure);
        }
        if end > bytes.len() {
            bytes.resize(end, 0);
        }
        bytes[offset as usize..end].copy_from_slice(&data);
        if let Some(token) = &fs.cancel_on_write {
            token.cancel();
        }
        Ok(status(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _: FileAttributes,
    ) -> Result<Status, StatusCode> {
        if !self.fs.lock().await.dirs.insert(path) {
            return Err(StatusCode::Failure);
        }
        Ok(status(id))
    }

    async fn remove(&mut self, id: u32, path: String) -> Result<Status, StatusCode> {
        self.fs
            .lock()
            .await
            .files
            .remove(&path)
            .ok_or(StatusCode::NoSuchFile)?;
        Ok(status(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, StatusCode> {
        let mut fs = self.fs.lock().await;
        let prefix = format!("{path}/");
        if fs.files.keys().any(|p| p.starts_with(&prefix))
            || fs.dirs.iter().any(|p| p.starts_with(&prefix))
        {
            return Err(StatusCode::Failure);
        }
        if !fs.dirs.remove(&path) {
            return Err(StatusCode::NoSuchFile);
        }
        Ok(status(id))
    }

    async fn rename(&mut self, id: u32, old: String, new: String) -> Result<Status, StatusCode> {
        let mut fs = self.fs.lock().await;
        if fs.files.contains_key(&new) {
            return Err(StatusCode::Failure);
        }
        let bytes = fs.files.remove(&old).ok_or(StatusCode::NoSuchFile)?;
        fs.files.insert(new, bytes);
        Ok(status(id))
    }
}

struct TestServer {
    port: u16,
    key: KnownHost,
    fixture: Fixture,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn start_server() -> TestServer {
    let private = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).unwrap();
    let public = private.public_key();
    let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = socket.local_addr().unwrap().port();
    let key = KnownHost {
        host: "127.0.0.1".into(),
        port,
        algorithm: public.algorithm().to_string(),
        public_key: public.public_key_base64(),
        fingerprint: public.fingerprint(HashAlg::Sha256).to_string(),
    };
    let config = Arc::new(server::Config {
        keys: vec![private],
        auth_rejection_time: Duration::ZERO,
        auth_rejection_time_initial: Some(Duration::ZERO),
        ..Default::default()
    });
    let fs = MemoryFs {
        dirs: HashSet::from(["/".into(), "/data".into()]),
        ..Default::default()
    };
    let fixture = Fixture {
        fs: Arc::new(Mutex::new(fs)),
        auth_attempts: Arc::new(AtomicUsize::new(0)),
    };
    let mut runner = fixture.clone();
    let task = tokio::spawn(async move {
        let _ = runner.run_on_socket(config, &socket).await;
    });
    TestServer {
        port,
        key,
        fixture,
        task,
    }
}

fn profile(port: u16) -> Connection {
    Connection {
        id: "integration".into(),
        name: "Loopback fixture".into(),
        host: "127.0.0.1".into(),
        port,
        connect_timeout_seconds: 20,
        username: "test".into(),
        auth_method: AuthMethod::Password,
        key_path: None,
        remember_secret: false,
        remote_path: ".".into(),
        local_path: None,
        color: "#b5e878".into(),
    }
}

#[tokio::test]
async fn stalled_ssh_reports_the_phase_and_configured_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().join("config")).unwrap());
    let mut connection = profile(port);
    connection.connect_timeout_seconds = 5;
    // Accept TCP but never send an SSH banner; this needs no external network.
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        std::future::pending::<()>().await;
        drop(stream);
    });
    let error = crate::ssh::connect(&state, connection, None)
        .await
        .err()
        .unwrap();
    server.abort();
    assert_eq!(error.code, "TIMEOUT");
    assert!(
        error
            .message
            .starts_with("SSH negotiation timed out after 5 seconds.")
    );
    let translation = error.translation.unwrap();
    assert_eq!(translation.params["seconds"], "5");
    assert!(state.sessions.lock().await.is_empty());
}

#[tokio::test]
async fn refused_tcp_reports_the_phase() {
    // Reserve a port without listening so concurrent fixtures cannot reuse it.
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let port = socket.local_addr().unwrap().port();
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().join("config")).unwrap());
    let error = crate::ssh::connect(&state, profile(port), None)
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, "CONNECTION");
    assert!(error.message.starts_with("TCP connection failed:"));
    assert!(state.sessions.lock().await.is_empty());
}

fn transfer(direction: Direction, source: String, destination: String) -> Transfer {
    Transfer {
        id: "test-transfer".into(),
        session_id: "test".into(),
        name: "test".into(),
        direction,
        source,
        destination,
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
    }
}

#[tokio::test]
async fn unknown_and_changed_keys_are_rejected_before_password_authentication() {
    let server = start_server().await;
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().to_owned()).unwrap());
    let result = crate::ssh::connect(
        &state,
        profile(server.port),
        Some(Zeroizing::new("test-only".into())),
    )
    .await;
    let error = result.err().unwrap();
    assert_eq!(error.code, "HOST_KEY_UNKNOWN");
    assert!(!error.host_key.unwrap().changed);
    assert_eq!(server.fixture.auth_attempts.load(Ordering::SeqCst), 0);
    let mut wrong = server.key.clone();
    wrong.public_key = "different-key".into();
    state.store.lock().await.config.known_hosts.push(wrong);
    let result = crate::ssh::connect(
        &state,
        profile(server.port),
        Some(Zeroizing::new("test-only".into())),
    )
    .await;
    assert_eq!(result.err().unwrap().code, "HOST_KEY_CHANGED");
    assert_eq!(server.fixture.auth_attempts.load(Ordering::SeqCst), 0);
    assert!(state.sessions.lock().await.is_empty());
}

#[tokio::test]
async fn remote_deletion_removes_nested_contents() {
    let server = start_server().await;
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().join("config")).unwrap());
    state
        .store
        .lock()
        .await
        .config
        .known_hosts
        .push(server.key.clone());
    let session = crate::ssh::connect(
        &state,
        profile(server.port),
        Some(Zeroizing::new("test-only".into())),
    )
    .await
    .unwrap();
    {
        let mut fs = server.fixture.fs.lock().await;
        fs.dirs.insert("/data/tree".into());
        fs.dirs.insert("/data/tree/nested".into());
        fs.files
            .insert("/data/tree/nested/file".into(), b"data".to_vec());
        fs.files.insert("/data/keep".into(), b"safe".to_vec());
    }
    crate::commands::delete_remote_paths(&session.sftp, vec!["/data/tree".into()])
        .await
        .unwrap();
    let fs = server.fixture.fs.lock().await;
    assert!(!fs.dirs.contains("/data/tree"));
    assert!(!fs.dirs.contains("/data/tree/nested"));
    assert!(!fs.files.contains_key("/data/tree/nested/file"));
    assert_eq!(fs.files["/data/keep"], b"safe");
}

#[tokio::test]
async fn real_sftp_round_trip_preserves_bytes_and_refuses_overwrites() {
    let server = start_server().await;
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().join("config")).unwrap());
    state
        .store
        .lock()
        .await
        .config
        .known_hosts
        .push(server.key.clone());
    let session = crate::ssh::connect(
        &state,
        profile(server.port),
        Some(Zeroizing::new("test-only".into())),
    )
    .await
    .unwrap();
    assert_eq!(session.info.home_path, "/data");
    let source = temp.path().join("payload.bin");
    let bytes: Vec<u8> = (0..=255).cycle().take(700_001).collect();
    std::fs::write(&source, &bytes).unwrap();
    let mut upload = transfer(
        Direction::Upload,
        source.to_string_lossy().into_owned(),
        "/data".into(),
    );
    crate::transfer::run(
        None,
        &state,
        &session.sftp,
        &mut upload,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(upload.bytes_transferred, bytes.len() as u64);
    assert_eq!(
        server.fixture.fs.lock().await.files["/data/payload.bin"],
        bytes
    );
    let names: Vec<_> = session
        .sftp
        .read_dir("/data")
        .await
        .unwrap()
        .map(|file| file.file_name())
        .collect();
    assert_eq!(names, vec!["payload.bin"]);
    let downloaded = temp.path().join("download");
    std::fs::create_dir(&downloaded).unwrap();
    let mut download = transfer(
        Direction::Download,
        "/data/payload.bin".into(),
        downloaded.to_string_lossy().into_owned(),
    );
    crate::transfer::run(
        None,
        &state,
        &session.sftp,
        &mut download,
        &CancellationToken::new(),
    )
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(downloaded.join("payload.bin")).unwrap(),
        bytes
    );
    assert_eq!(download.files_done, 1);
    let error = crate::transfer::run(
        None,
        &state,
        &session.sftp,
        &mut download,
        &CancellationToken::new(),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.code, "EXISTS");
    let error = crate::transfer::run(
        None,
        &state,
        &session.sftp,
        &mut upload,
        &CancellationToken::new(),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.code, "EXISTS");
    assert_eq!(
        std::fs::read(downloaded.join("payload.bin")).unwrap(),
        bytes
    );
}

#[tokio::test]
async fn cancellation_removes_the_remote_partial_file() {
    let server = start_server().await;
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().join("config")).unwrap());
    state
        .store
        .lock()
        .await
        .config
        .known_hosts
        .push(server.key.clone());
    let session = crate::ssh::connect(
        &state,
        profile(server.port),
        Some(Zeroizing::new("test-only".into())),
    )
    .await
    .unwrap();
    let source = temp.path().join("cancel.bin");
    std::fs::write(&source, vec![42u8; 1_000_000]).unwrap();
    let cancel = CancellationToken::new();
    server.fixture.fs.lock().await.cancel_on_write = Some(cancel.clone());
    let mut upload = transfer(
        Direction::Upload,
        source.to_string_lossy().into_owned(),
        "/data".into(),
    );
    let error = crate::transfer::run(None, &state, &session.sftp, &mut upload, &cancel)
        .await
        .err()
        .unwrap();
    assert_eq!(error.code, "CANCELLED");
    assert!(server.fixture.fs.lock().await.files.is_empty());
}

async fn pending_conflict(state: &AppState) -> TransferConflict {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(conflict) = state
                .transfers
                .lock()
                .await
                .get("test-transfer")
                .and_then(|task| task.transfer.conflict.clone())
            {
                return conflict;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("conflict was not published")
}

async fn run_conflicting_transfer(
    state: &AppState,
    sftp: Arc<russh_sftp::client::SftpSession>,
    transfer: Transfer,
) -> (
    CancellationToken,
    tokio::task::JoinHandle<(crate::error::AppResult<()>, Transfer)>,
) {
    let cancel = CancellationToken::new();
    state.transfers.lock().await.insert(
        transfer.id.clone(),
        crate::state::TransferTask {
            transfer: transfer.clone(),
            cancel: cancel.clone(),
            decision: None,
        },
    );
    let state = state.clone();
    let token = cancel.clone();
    let task = tokio::spawn(async move {
        let mut transfer = transfer;
        let result = crate::transfer::run(None, &state, &sftp, &mut transfer, &token).await;
        (result, transfer)
    });
    (cancel, task)
}

#[tokio::test]
async fn conflict_decisions_work_for_uploads_and_downloads_without_clobbering_implicitly() {
    let server = start_server().await;
    let temp = tempfile::tempdir().unwrap();
    let state = AppState::new(Store::load(temp.path().join("config")).unwrap());
    state
        .store
        .lock()
        .await
        .config
        .known_hosts
        .push(server.key.clone());
    let session = crate::ssh::connect(
        &state,
        profile(server.port),
        Some(Zeroizing::new("test-only".into())),
    )
    .await
    .unwrap();
    let source = temp.path().join("payload.txt");
    std::fs::write(&source, b"new data").unwrap();
    server
        .fixture
        .fs
        .lock()
        .await
        .files
        .insert("/data/payload.txt".into(), b"original".to_vec());

    let make_upload = || {
        transfer(
            Direction::Upload,
            source.to_string_lossy().into_owned(),
            "/data".into(),
        )
    };
    let (_, task) = run_conflicting_transfer(&state, session.sftp.clone(), make_upload()).await;
    let conflict = pending_conflict(&state).await;
    assert_eq!(
        server.fixture.fs.lock().await.files["/data/payload.txt"],
        b"original"
    );
    assert!(
        crate::transfer::resolve_conflict(
            &state,
            "test-transfer",
            "wrong",
            ConflictDecision::Overwrite
        )
        .await
        .is_err()
    );
    assert!(
        crate::transfer::resolve_conflict(
            &state,
            "test-transfer",
            &conflict.id,
            ConflictDecision::Rename {
                name: "../escape".into()
            }
        )
        .await
        .is_err()
    );
    crate::transfer::resolve_conflict(
        &state,
        "test-transfer",
        &conflict.id,
        ConflictDecision::Rename {
            name: "copy.txt".into(),
        },
    )
    .await
    .unwrap();
    task.await.unwrap().0.unwrap();
    {
        let fs = server.fixture.fs.lock().await;
        assert_eq!(fs.files["/data/payload.txt"], b"original");
        assert_eq!(fs.files["/data/copy.txt"], b"new data");
    }

    let (_, task) = run_conflicting_transfer(&state, session.sftp.clone(), make_upload()).await;
    let conflict = pending_conflict(&state).await;
    crate::transfer::resolve_conflict(
        &state,
        "test-transfer",
        &conflict.id,
        ConflictDecision::Overwrite,
    )
    .await
    .unwrap();
    task.await.unwrap().0.unwrap();
    {
        let fs = server.fixture.fs.lock().await;
        assert_eq!(fs.files["/data/payload.txt"], b"new data");
        assert!(
            !fs.files
                .keys()
                .any(|name| name.ends_with(".backup") || name.ends_with(".part"))
        );
    }

    let download_dir = temp.path().join("download");
    std::fs::create_dir(&download_dir).unwrap();
    std::fs::write(download_dir.join("payload.txt"), b"local original").unwrap();
    let make_download = || {
        transfer(
            Direction::Download,
            "/data/payload.txt".into(),
            download_dir.to_string_lossy().into_owned(),
        )
    };
    let (_, task) = run_conflicting_transfer(&state, session.sftp.clone(), make_download()).await;
    let conflict = pending_conflict(&state).await;
    crate::transfer::resolve_conflict(
        &state,
        "test-transfer",
        &conflict.id,
        ConflictDecision::Skip,
    )
    .await
    .unwrap();
    let (result, skipped) = task.await.unwrap();
    result.unwrap();
    assert_eq!(skipped.skipped, 1);
    assert_eq!(
        std::fs::read(download_dir.join("payload.txt")).unwrap(),
        b"local original"
    );

    let (_, task) = run_conflicting_transfer(&state, session.sftp.clone(), make_download()).await;
    let conflict = pending_conflict(&state).await;
    crate::transfer::resolve_conflict(
        &state,
        "test-transfer",
        &conflict.id,
        ConflictDecision::Overwrite,
    )
    .await
    .unwrap();
    task.await.unwrap().0.unwrap();
    assert_eq!(
        std::fs::read(download_dir.join("payload.txt")).unwrap(),
        b"new data"
    );

    let (cancel, task) =
        run_conflicting_transfer(&state, session.sftp.clone(), make_download()).await;
    let conflict = pending_conflict(&state).await;
    cancel.cancel();
    assert_eq!(task.await.unwrap().0.err().unwrap().code, "CANCELLED");
    assert!(
        crate::transfer::resolve_conflict(
            &state,
            "test-transfer",
            &conflict.id,
            ConflictDecision::Overwrite
        )
        .await
        .is_err()
    );
    assert_eq!(
        std::fs::read(download_dir.join("payload.txt")).unwrap(),
        b"new data"
    );
}

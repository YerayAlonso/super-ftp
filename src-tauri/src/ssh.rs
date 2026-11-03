use std::{
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use russh::{
    client,
    keys::{
        HashAlg, PrivateKeyWithHashAlg, PublicKeyBase64, PublicKeyOrCertificate,
        agent::client::AgentClient,
    },
};
use russh_sftp::client::SftpSession;
use tokio::sync::{Mutex, Semaphore};
use zeroize::Zeroizing;

use crate::{
    error::{AppError, AppResult},
    models::{AuthMethod, Connection, HostKeyChallenge, KnownHost, SessionInfo},
    state::{AppState, PendingHost},
};

pub struct Verifier {
    host: String,
    port: u16,
    known: Option<KnownHost>,
    observed: Arc<StdMutex<Option<KnownHost>>>,
}

impl client::Handler for Verifier {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // Host CA/certificate trust is intentionally not implemented in this MVP.
        if server_key.certificate().is_some() {
            return Ok(false);
        }
        let public = server_key.public_key();
        let key = KnownHost {
            host: self.host.clone(),
            port: self.port,
            algorithm: public.algorithm().to_string(),
            public_key: public.public_key_base64(),
            fingerprint: public.fingerprint(HashAlg::Sha256).to_string(),
        };
        let accepted = self
            .known
            .as_ref()
            .is_some_and(|known| known.public_key == key.public_key);
        if let Ok(mut observed) = self.observed.lock() {
            *observed = Some(key);
        }
        Ok(accepted)
    }
}

pub struct RemoteSession {
    pub info: SessionInfo,
    pub sftp: Arc<SftpSession>,
    pub ssh: Mutex<client::Handle<Verifier>>,
    pub slots: Arc<Semaphore>,
}

pub async fn connect(
    state: &AppState,
    connection: Connection,
    secret: Option<Zeroizing<String>>,
) -> AppResult<Arc<RemoteSession>> {
    let _permit = state
        .connection_slots
        .acquire()
        .await
        .map_err(|_| AppError::new("BUSY", "No more connections can be opened right now."))?;
    if state.sessions.lock().await.len() >= 16 {
        return Err(AppError::new(
            "SESSION_LIMIT",
            "You can have up to 16 active sessions.",
        ));
    }
    let known = state
        .store
        .lock()
        .await
        .config
        .known_hosts
        .iter()
        .find(|known| known.host == connection.host && known.port == connection.port)
        .cloned();
    let observed = Arc::new(StdMutex::new(None));
    let verifier = Verifier {
        host: connection.host.clone(),
        port: connection.port,
        known: known.clone(),
        observed: observed.clone(),
    };
    let config = Arc::new(client::Config {
        keepalive_interval: Some(Duration::from_secs(30)),
        keepalive_max: 3,
        ..Default::default()
    });
    let result = tokio::time::timeout(
        Duration::from_secs(20),
        client::connect(
            config,
            (connection.host.as_str(), connection.port),
            verifier,
        ),
    )
    .await
    .map_err(|_| AppError::new("TIMEOUT", "The server did not respond within 20 seconds."))?;
    let key = observed.lock().ok().and_then(|guard| guard.clone());
    let mut ssh = match result {
        Ok(ssh) => ssh,
        Err(error) => {
            if let Some(key) = key {
                let changed = known
                    .as_ref()
                    .is_some_and(|known| known.public_key != key.public_key);
                if known.is_none() || changed {
                    let id = uuid::Uuid::new_v4().to_string();
                    if !changed {
                        let mut pending = state.pending_hosts.lock().await;
                        pending.retain(|_, pending| {
                            pending.created.elapsed() < Duration::from_secs(300)
                        });
                        if pending.len() >= 32 {
                            pending.clear();
                        }
                        pending.insert(
                            id.clone(),
                            PendingHost {
                                key: key.clone(),
                                created: std::time::Instant::now(),
                            },
                        );
                    }
                    return Err(AppError::host_key(HostKeyChallenge {
                        id,
                        host: key.host,
                        port: key.port,
                        algorithm: key.algorithm,
                        fingerprint: key.fingerprint,
                        previous_fingerprint: known.map(|known| known.fingerprint),
                        changed,
                    }));
                }
            }
            return Err(error.into());
        }
    };
    let authenticated = tokio::time::timeout(Duration::from_secs(45), async {
        match connection.auth_method {
            AuthMethod::Password => {
                let secret = secret.as_ref().ok_or_else(|| {
                    AppError::new("SECRET_REQUIRED", "Enter the connection password.")
                })?;
                Ok(ssh
                    .authenticate_password(&connection.username, secret.as_str())
                    .await?
                    .success())
            }
            AuthMethod::KeyFile => {
                let path = connection
                    .key_path
                    .clone()
                    .ok_or_else(|| AppError::new("KEY", "The SSH key path is missing."))?;
                let key = tokio::task::spawn_blocking(move || {
                    russh::keys::load_secret_key(
                        &path,
                        secret.as_ref().map(|secret| secret.as_str()),
                    )
                    .map_err(|_| {
                        AppError::new(
                            "KEY",
                            "Could not read the private key. Check the file and passphrase.",
                        )
                    })
                })
                .await
                .map_err(|_| AppError::new("KEY", "Could not load the private key."))??;
                let hash = ssh.best_supported_rsa_hash().await?.flatten();
                Ok(ssh
                    .authenticate_publickey(
                        &connection.username,
                        PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                    )
                    .await?
                    .success())
            }
            AuthMethod::Agent => authenticate_agent(&mut ssh, &connection.username).await,
        }
    })
    .await
    .map_err(|_| AppError::new("TIMEOUT", "Authentication timed out."))??;
    if !authenticated {
        return Err(AppError::new(
            "AUTH",
            "The server rejected authentication. Check your username and credentials.",
        ));
    }
    let sftp = tokio::time::timeout(Duration::from_secs(20), async {
        let channel = ssh.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;
        sftp.set_timeout(30);
        Ok::<_, AppError>(sftp)
    })
    .await
    .map_err(|_| AppError::new("TIMEOUT", "Could not start the SFTP subsystem."))??;
    // Open the account home first. A stale saved starting folder must not make
    // otherwise valid authentication fail or leak a connected session.
    let home = sftp.canonicalize(".").await?;
    let fingerprint = known.map(|known| known.fingerprint).unwrap_or_default();
    let info = SessionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        connection_id: connection.id,
        home_path: home,
        fingerprint,
    };
    let session = Arc::new(RemoteSession {
        info,
        sftp: Arc::new(sftp),
        ssh: Mutex::new(ssh),
        slots: Arc::new(Semaphore::new(2)),
    });
    let mut sessions = state.sessions.lock().await;
    if sessions.len() >= 16 {
        return Err(AppError::new(
            "SESSION_LIMIT",
            "You can have up to 16 active sessions.",
        ));
    }
    sessions.insert(session.info.id.clone(), session.clone());
    Ok(session)
}

async fn authenticate_agent(ssh: &mut client::Handle<Verifier>, user: &str) -> AppResult<bool> {
    #[cfg(unix)]
    let mut agent = AgentClient::connect_env().await.map_err(|_| {
        AppError::new(
            "AGENT",
            "Could not connect to the SSH agent. Check SSH_AUTH_SOCK.",
        )
    })?;
    #[cfg(windows)]
    let mut agent = AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await
        .map_err(|_| AppError::new("AGENT", "Could not connect to the Windows OpenSSH agent. Make sure the ssh-agent service is running."))?;
    let identities = agent
        .request_identities()
        .await
        .map_err(|_| AppError::new("AGENT", "The SSH agent could not list its keys."))?;
    let hash = ssh.best_supported_rsa_hash().await?.flatten();
    for identity in identities.into_iter().take(6) {
        if ssh
            .authenticate_publickey_with(user, identity.public_key().into_owned(), hash, &mut agent)
            .await
            .map_err(|_| {
                AppError::new(
                    "AGENT",
                    "The SSH agent could not sign the authentication request.",
                )
            })?
            .success()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

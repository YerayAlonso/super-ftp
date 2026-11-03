use std::{collections::HashMap, sync::Arc, time::Instant};

use tokio::sync::{Mutex, Semaphore};
use tokio_util::sync::CancellationToken;

use crate::{
    error::{AppError, AppResult},
    models::{ConflictDecision, KnownHost, Transfer},
    ssh::RemoteSession,
    storage::Store,
};

pub struct PendingHost {
    pub key: KnownHost,
    pub created: Instant,
}

pub struct TransferTask {
    pub transfer: Transfer,
    pub cancel: CancellationToken,
    pub decision: Option<(String, tokio::sync::oneshot::Sender<ConflictDecision>)>,
}

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub sessions: Arc<Mutex<HashMap<String, Arc<RemoteSession>>>>,
    pub pending_hosts: Arc<Mutex<HashMap<String, PendingHost>>>,
    pub transfers: Arc<Mutex<HashMap<String, TransferTask>>>,
    pub slots: Arc<Semaphore>,
    pub connection_slots: Arc<Semaphore>,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            pending_hosts: Arc::new(Mutex::new(HashMap::new())),
            transfers: Arc::new(Mutex::new(HashMap::new())),
            slots: Arc::new(Semaphore::new(3)),
            connection_slots: Arc::new(Semaphore::new(4)),
        }
    }

    pub async fn session(&self, id: &str) -> AppResult<Arc<RemoteSession>> {
        self.sessions
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::new("SESSION_CLOSED", "The session is closed. Reconnect."))
    }
}

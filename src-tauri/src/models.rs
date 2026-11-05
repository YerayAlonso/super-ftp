use serde::{Deserialize, Serialize};

fn default_connect_timeout_seconds() -> u64 {
    20
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    Password,
    KeyFile,
    Agent,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    #[serde(default = "default_connect_timeout_seconds")]
    pub connect_timeout_seconds: u64,
    pub username: String,
    pub auth_method: AuthMethod,
    pub key_path: Option<String>,
    pub remember_secret: bool,
    pub remote_path: String,
    pub local_path: Option<String>,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownHost {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub public_key: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub version: u8,
    pub connections: Vec<Connection>,
    pub known_hosts: Vec<KnownHost>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            connections: vec![],
            known_hosts: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyChallenge {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
    pub previous_fingerprint: Option<String>,
    pub changed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<u64>,
    pub permissions: Option<u32>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub connection_id: String,
    pub home_path: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Upload,
    Download,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransferStatus {
    Queued,
    Preparing,
    Transferring,
    AwaitingDecision,
    Completed,
    Failed,
    Cancelled,
}

impl TransferStatus {
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Preparing | Self::Transferring | Self::AwaitingDecision
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub direction: Direction,
    pub source: String,
    pub destination: String,
    pub status: TransferStatus,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub files_total: u64,
    pub files_done: u64,
    pub skipped: u64,
    pub bytes_per_second: u64,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_translation: Option<Box<MessageTranslation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<TransferConflict>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferConflict {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub name: String,
    pub direction: Direction,
    pub source_size: u64,
    pub destination_size: Option<u64>,
    pub destination_modified: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", tag = "action")]
pub enum ConflictDecision {
    Overwrite,
    Rename { name: String },
    Skip,
    Cancel,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageTranslation {
    pub key: String,
    pub params: std::collections::BTreeMap<String, String>,
}

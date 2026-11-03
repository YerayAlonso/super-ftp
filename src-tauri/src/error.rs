use serde::Serialize;

use crate::models::{HostKeyChallenge, MessageTranslation};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_key: Option<Box<HostKeyChallenge>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<Box<MessageTranslation>>,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        // Application-owned errors carry a stable translation key as well as
        // an English fallback. External OS/SSH/SFTP errors retain their details.
        let (message, translation) = if let Some(fallback) = english_fallback(code) {
            (
                fallback.to_owned(),
                Some(Box::new(MessageTranslation {
                    key: message,
                    params: std::collections::BTreeMap::new(),
                })),
            )
        } else {
            (message, None)
        };
        Self {
            code: code.into(),
            message,
            host_key: None,
            translation,
        }
    }

    pub fn parameterized(
        code: &str,
        template: &str,
        param: &str,
        value: impl Into<String>,
    ) -> Self {
        let value = value.into();
        let fallback = match code {
            "EXISTS" => "The destination already exists and was not overwritten: {path}",
            "UNSAFE_NAME" => "The name cannot be downloaded safely: {name}",
            "BACKUP" => "The original could not be restored. Backup: {path}",
            _ => template,
        };
        Self {
            code: code.into(),
            message: fallback.replace(&format!("{{{param}}}"), &value),
            host_key: None,
            translation: Some(Box::new(MessageTranslation {
                key: template.into(),
                params: std::collections::BTreeMap::from([(param.into(), value)]),
            })),
        }
    }

    pub fn host_key(challenge: HostKeyChallenge) -> Self {
        Self {
            code: if challenge.changed { "HOST_KEY_CHANGED" } else { "HOST_KEY_UNKNOWN" }.into(),
            message: if challenge.changed { "The server key has changed. Connection blocked; verify it with the administrator." } else { "Verify the server identity before connecting." }.into(),
            translation: Some(Box::new(MessageTranslation { key: if challenge.changed {
                "The server key has changed. Connection blocked: verify it with the administrator."
            } else { "Verify this server’s identity before connecting." }.into(), params: Default::default() })),
            host_key: Some(Box::new(challenge)),
        }
    }

    pub fn cancelled() -> Self {
        Self::new("CANCELLED", "Transfer cancelled.")
    }
}

fn english_fallback(code: &str) -> Option<&'static str> {
    Some(match code {
        "NOT_FOUND" => "The requested connection was not found.",
        "LIMIT" => "The permitted limit was exceeded.",
        "KEYRING" => "The system credential store could not complete the operation.",
        "HOST_KEY_EXPIRED" => "The server verification has expired. Connect again.",
        "HOST_KEY_CHANGED" => "The known server key cannot be replaced in this dialog.",
        "HOME" => "The home folder could not be found.",
        "LOCAL" => "The local filesystem operation failed.",
        "ROOT" => "This operation cannot be performed on the root directory.",
        "EXISTS" => "The destination already exists and was not overwritten.",
        "CANCELLED" => "Transfer cancelled.",
        "INVALID_PATH" => "The path is invalid.",
        "INVALID_NAME" => "The filename is invalid or unsupported.",
        "BUSY" => "No more connections can be opened right now.",
        "SESSION_LIMIT" => "The active session limit was reached.",
        "TIMEOUT" => "The operation timed out.",
        "SECRET_REQUIRED" => "A password is required for this connection.",
        "KEY" => "The SSH private key could not be used.",
        "AUTH" => "The server rejected authentication.",
        "AGENT" => "The SSH agent could not complete authentication.",
        "SESSION_CLOSED" => "The session is closed. Reconnect.",
        "CONFIG" => "The application configuration could not be processed.",
        "VALIDATION" => "The connection details are invalid.",
        "QUEUE_FULL" => "The transfer queue is full.",
        "NOT_DIRECTORY" => "The destination is not a directory.",
        "SCAN" => "The files could not be prepared for transfer.",
        "CONFLICT_EXPIRED" => "This decision is no longer available.",
        "SCAN_LIMIT" => "The folder exceeds the transfer scanning limits.",
        "UNSAFE_DESTINATION" => "The destination is not a safe regular file.",
        "PATH" => "The destination path is invalid.",
        _ => return None,
    })
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new("IO", error.to_string())
    }
}

impl From<russh::Error> for AppError {
    fn from(error: russh::Error) -> Self {
        Self::new("SSH", error.to_string())
    }
}

impl From<russh_sftp::client::error::Error> for AppError {
    fn from(error: russh_sftp::client::error::Error) -> Self {
        Self::new("SFTP", error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameterized_errors_preserve_a_readable_fallback_and_safe_translation_data() {
        let error = AppError::parameterized(
            "EXISTS",
            "“{path}” already exists. It was not overwritten.",
            "path",
            "/data/{path}<file>",
        );
        let value = serde_json::to_value(error).unwrap();
        assert_eq!(
            value["message"],
            "The destination already exists and was not overwritten: /data/{path}<file>"
        );
        assert_eq!(
            value["translation"]["key"],
            "“{path}” already exists. It was not overwritten."
        );
        assert_eq!(value["translation"]["params"]["path"], "/data/{path}<file>");
    }
}

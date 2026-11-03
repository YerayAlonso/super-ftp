use std::{io::Write, path::PathBuf};

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{
    error::{AppError, AppResult},
    models::{AuthMethod, Config, Connection},
    paths::validate_remote_path,
};

const SERVICE: &str = "dev.superftp.desktop";

pub struct Store {
    pub config: Config,
    directory: PathBuf,
}

impl Store {
    pub fn load(directory: PathBuf) -> AppResult<Self> {
        std::fs::create_dir_all(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
        }
        let path = directory.join("connections.json");
        let config = if path.exists() {
            let data = std::fs::read(&path)?;
            if data.len() > 4 * 1024 * 1024 {
                return Err(AppError::new(
                    "CONFIG",
                    "The configuration file is too large.",
                ));
            }
            let config: Config = serde_json::from_slice(&data).map_err(|_| {
                AppError::new(
                    "CONFIG",
                    "The configuration is invalid. It has not been overwritten.",
                )
            })?;
            if config.version != 1 {
                return Err(AppError::new(
                    "CONFIG",
                    "Unsupported configuration version.",
                ));
            }
            config
        } else {
            Config::default()
        };
        Ok(Self { config, directory })
    }

    pub fn persist(&mut self, next: Config) -> AppResult<()> {
        let mut file = tempfile::NamedTempFile::new_in(&self.directory)?;
        serde_json::to_writer_pretty(&mut file, &next)
            .map_err(|_| AppError::new("CONFIG", "Could not serialize the configuration."))?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(self.directory.join("connections.json"))
            .map_err(|error| AppError::from(error.error))?;
        self.config = next;
        Ok(())
    }
}

pub fn validate_connection(connection: &mut Connection) -> AppResult<()> {
    connection.name = connection.name.trim().into();
    connection.host = connection
        .host
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase();
    connection.username = connection.username.trim().into();
    if connection.name.is_empty()
        || connection.name.len() > 100
        || connection.host.is_empty()
        || connection.host.len() > 253
        || connection.host.contains(['/', '\\', '@'])
        || connection.host.chars().any(char::is_whitespace)
        || connection.host.chars().any(char::is_control)
        || connection.username.is_empty()
        || connection.username.len() > 255
        || connection.username.chars().any(char::is_control)
        || connection.port == 0
    {
        return Err(AppError::new(
            "VALIDATION",
            "Check the connection name, server, port and username.",
        ));
    }
    if connection.auth_method == AuthMethod::KeyFile
        && connection.key_path.as_deref().is_none_or(str::is_empty)
    {
        return Err(AppError::new("VALIDATION", "Select a private SSH key."));
    }
    if connection.remote_path.trim().is_empty() {
        connection.remote_path = ".".into();
    }
    validate_remote_path(&connection.remote_path)?;
    if ![
        "#b5e878", "#7cb7ff", "#c4a3ff", "#ffbc7c", "#ff8ca1", "#7cddd1",
    ]
    .contains(&connection.color.as_str())
    {
        connection.color = "#b5e878".into();
    }
    Ok(())
}

pub fn secret_entry(id: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, id)
        .map_err(|_| AppError::new("KEYRING", "The system credential store is unavailable. You can connect without saving the secret."))
}

/// Bind vault entries to the server, user, and authentication identity, not
/// just a mutable profile ID. Even a failed config write cannot move a secret
/// to the old server after the profile's host/user/key has been edited.
pub fn credential_id(connection: &Connection) -> String {
    let identity = serde_json::to_vec(&(
        &connection.host,
        connection.port,
        &connection.username,
        &connection.auth_method,
        &connection.key_path,
    ))
    .expect("serializing a tuple of strings cannot fail");
    let digest = Sha256::digest(identity);
    let suffix: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("{}:{suffix}", connection.id)
}

pub fn read_secret(id: &str) -> AppResult<Option<Zeroizing<String>>> {
    match secret_entry(id)?.get_password() {
        Ok(secret) => Ok(Some(Zeroizing::new(secret))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(AppError::new(
            "KEYRING",
            "Could not read the secret from the system credential store.",
        )),
    }
}

pub fn delete_secret(id: &str) -> AppResult<()> {
    match secret_entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(AppError::new(
            "KEYRING",
            "Could not delete the secret from the secure credential store.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Connection {
        Connection {
            id: "test".into(),
            name: " Work ".into(),
            host: " EXAMPLE.COM ".into(),
            port: 22,
            username: " deploy ".into(),
            auth_method: AuthMethod::Password,
            key_path: None,
            remember_secret: false,
            remote_path: ".".into(),
            local_path: None,
            color: "invalid".into(),
        }
    }

    #[test]
    fn validates_and_normalizes_profiles() {
        let mut value = connection();
        validate_connection(&mut value).unwrap();
        assert_eq!(value.host, "example.com");
        assert_eq!(value.name, "Work");
        value.host = "user@host/path".into();
        assert!(validate_connection(&mut value).is_err());
    }

    #[test]
    fn configuration_has_no_secret_field() {
        let json = serde_json::to_string(&connection()).unwrap();
        assert!(!json.contains("\"secret\""));
        assert!(!json.contains("\"password\":"));
    }

    #[test]
    fn vault_keys_are_bound_to_the_authentication_identity() {
        let original = connection();
        let mut renamed = original.clone();
        renamed.name = "Another label".into();
        assert_eq!(credential_id(&original), credential_id(&renamed));
        renamed.host = "different.example.com".into();
        assert_ne!(credential_id(&original), credential_id(&renamed));
        renamed = original.clone();
        renamed.username = "another-user".into();
        assert_ne!(credential_id(&original), credential_id(&renamed));
    }

    #[test]
    fn persists_atomically_and_preserves_invalid_configs() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::load(temp.path().to_owned()).unwrap();
        let mut next = Config::default();
        next.connections.push(connection());
        store.persist(next).unwrap();
        assert_eq!(
            Store::load(temp.path().to_owned())
                .unwrap()
                .config
                .connections
                .len(),
            1
        );
        std::fs::write(temp.path().join("connections.json"), "broken").unwrap();
        assert!(Store::load(temp.path().to_owned()).is_err());
        assert_eq!(
            std::fs::read_to_string(temp.path().join("connections.json")).unwrap(),
            "broken"
        );
    }
}

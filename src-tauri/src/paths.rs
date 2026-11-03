use std::path::{Component, Path};

use crate::error::{AppError, AppResult};

pub fn validate_remote_path(path: &str) -> AppResult<()> {
    if path.is_empty() || path.len() > 8192 || path.chars().any(char::is_control) {
        return Err(AppError::new("INVALID_PATH", "The remote path is invalid."));
    }
    Ok(())
}

pub fn validate_leaf(name: &str) -> AppResult<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.len() > 255
        || name.contains('/')
        || name.contains('\\')
        || name.chars().any(char::is_control)
    {
        return Err(AppError::new(
            "INVALID_NAME",
            "The name must be a single valid path component.",
        ));
    }
    Ok(())
}

pub fn validate_remote_target(path: &str) -> AppResult<()> {
    validate_remote_path(path)?;
    if !path.starts_with('/') {
        return Err(AppError::new(
            "INVALID_PATH",
            "The remote item must have an absolute path.",
        ));
    }
    validate_leaf(path.rsplit('/').next().unwrap_or(""))
}

pub fn is_local_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Includes directory junctions and other reparse points.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.is_symlink()
    }
}

pub fn is_local_hidden(name: &str, metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        name.starts_with('.') || metadata.file_attributes() & 0x2 != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        name.starts_with('.')
    }
}

/// The same conservative rules on every OS prevent path traversal, NTFS ADS,
/// device names, and cross-platform filename surprises in untrusted listings.
pub fn validate_download_name(name: &str) -> AppResult<()> {
    validate_leaf(name)?;
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$" | "CLOCK$"
    ) || stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
        .is_some_and(|number| {
            matches!(
                number,
                "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        });
    if reserved || name.ends_with(['.', ' ']) || name.contains(['<', '>', ':', '"', '|', '?', '*'])
    {
        return Err(AppError::parameterized(
            "UNSAFE_NAME",
            "The name “{name}” cannot be downloaded safely.",
            "name",
            name,
        ));
    }
    Ok(())
}

pub fn join_remote(parent: &str, name: &str) -> AppResult<String> {
    validate_remote_path(parent)?;
    validate_leaf(name)?;
    Ok(format!("{}/{name}", parent.trim_end_matches('/')))
}

pub fn remote_parent(path: &str) -> Option<String> {
    let path = path.trim_end_matches('/');
    if path.is_empty() || path == "." {
        return None;
    }
    path.rsplit_once('/').map(|(parent, _)| {
        if parent.is_empty() {
            "/".into()
        } else {
            parent.into()
        }
    })
}

pub fn validate_absolute_local(path: &Path) -> AppResult<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(AppError::new(
            "INVALID_PATH",
            "An absolute local path without “..” components is required.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_remote_traversal_names() {
        for name in [
            "",
            ".",
            "..",
            "../secret",
            "a/b",
            "a\\b",
            "bad\0name",
            "bad\nname",
        ] {
            assert!(validate_leaf(name).is_err(), "{name:?}");
        }
    }

    #[test]
    fn blocks_windows_device_and_stream_names_on_every_platform() {
        for name in [
            "CON",
            "con.txt",
            "NUL",
            "COM1.log",
            "LPT9",
            "file:secret",
            "file.",
            "file ",
            "COM¹.txt",
            "LPT²",
            "CONIN$",
        ] {
            assert!(validate_download_name(name).is_err(), "{name}");
        }
        for name in ["photo.png", "còpia final.zip", ".gitignore", "company.txt"] {
            assert!(validate_download_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn remote_paths_are_posix_even_on_windows() {
        assert_eq!(join_remote("/", "folder").unwrap(), "/folder");
        assert_eq!(
            join_remote("/home/user/", "file").unwrap(),
            "/home/user/file"
        );
        assert_eq!(remote_parent("/home"), Some("/".into()));
        assert_eq!(remote_parent("/"), None);
    }

    #[test]
    fn destructive_targets_cannot_name_root_or_dot_components() {
        for path in ["/", "/home/..", "/home/.", "/home/", "relative/file"] {
            assert!(validate_remote_target(path).is_err(), "{path}");
        }
        assert!(validate_remote_target("/home/file.txt").is_ok());
    }
}

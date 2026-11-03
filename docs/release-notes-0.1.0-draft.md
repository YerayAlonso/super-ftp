# Super FTP 0.1.0 — preview (draft)

These notes are a draft for maintainer review. No release or tag is created by this file.

Super FTP is a preview desktop SFTP client. Browse local and remote files, manage multiple connections and transfer files over SSH.

## Highlights

- Password, SSH key and agent authentication, with host-key verification and optional system credential storage.
- File and folder transfers, per-file conflict decisions, cancellation, and a transfer queue.
- Eight interface languages, light and dark themes, and adjustable interface size.
- Local and remote folder operations, including permanent recursive deletion after confirmation.

## Windows preview

The intended initial asset is the Windows x64 NSIS installer. It is **unsigned**, so Windows may display a security warning. Only download an asset from a trusted release source and verify its checksum against a trusted copy.

The Windows native smoke test covers startup, local listings, context menus, local folder operations, path validation and language persistence; it does not cover real SFTP connections or credential storage. Additional manual Windows testing has been reported by the maintainer. Confirm installer installation, upgrade and uninstall on a clean system before publishing.

macOS and Linux CI builds pass, but the application and installers have not been validated on real devices; no macOS or Linux release asset should be advertised until tested.

## Limitations

This preview has not undergone an independent security audit. It supports SFTP, not FTP or FTPS. Folder deletion is permanent and can be partially completed if an error occurs. Remote overwrites use a backup-and-restore procedure rather than portable atomic replacement; interruption or failure can leave backup or partial files. See the [README](https://github.com/YerayAlonso/super-ftp/blob/main/README.md#preview-limitations) and [security notes](https://github.com/YerayAlonso/super-ftp/blob/main/SECURITY.md) for details.

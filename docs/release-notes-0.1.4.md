# Super FTP 0.1.4 — Windows preview candidate

## Changes since 0.1.3

- Choose a connection timeout from 5 to 120 seconds for each saved connection. Existing profiles keep the default of 20 seconds. DNS lookup, TCP connection and SSH negotiation share this connection budget; authentication and SFTP startup retain their separate limits.
- Connection failures now identify DNS, TCP or SSH negotiation problems, with translated guidance to help diagnose them.
- The connection dialog has a wider, two-column layout, keeps settings visible and adapts to narrow windows with a scrollable body.
- Updated development formatting tools and third-party notices.

## Availability and updates

This preview is for **Windows x64 only**. Users on 0.1.3 can update in-app after opting into preview updates. Users on 0.1.2 and earlier must install manually. The stable update channel remains inactive.

Tauri updater signatures are separate from Windows Authenticode. The installer is not Authenticode-signed, so Windows may display an unknown-publisher warning. macOS and Linux have not been tested on real devices, and no installers for those platforms are provided. The application has not undergone an independent security audit.

The unresolved RSA timing advisory remains documented in [SECURITY.md](https://github.com/YerayAlonso/super-ftp/blob/main/SECURITY.md#rsa-timing-advisory). Prefer Ed25519 where supported. Transfer, deletion and other preview limitations remain documented in the [README](https://github.com/YerayAlonso/super-ftp/blob/main/README.md#preview-limitations).

Third-party notices, verified MPL source archives and checksums accompany the installer. See [THIRD_PARTY_REVIEW.md](https://github.com/YerayAlonso/super-ftp/blob/main/THIRD_PARTY_REVIEW.md) for the scope of the licensing review.

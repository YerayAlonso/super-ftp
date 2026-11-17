# Super FTP 0.1.5 — Windows preview

## Changes since 0.1.4

- Fixed the RSA dependency timing advisory by updating to official russh 0.65.2, rsa 0.10.0 and ssh-key 0.7.0-rc.12 releases. RSA authentication remains supported, including encrypted private keys and RSA server host keys. Published versions through 0.1.4 still contain the affected dependency; install 0.1.5 to receive the correction.
- Expanded recovery tests for failed writes, failed overwrite publication, failed restoration and retained recovery backups. Existing transfer and recovery limitations remain documented in the README.
- Verified interoperability with a real OpenSSH server, including RSA authentication, explicit overwrite and byte-for-byte upload/download. Added native Windows Credential Manager and OpenSSH agent integration checks.
- Expanded third-party notices with 49 crate-specific full-text supplements. Automated verification checks 649 license/NOTICE files, all six bundled frontend packages and the complete notice text displayed by the packaged application.
- Added dependency-change security audit checks. Full and production npm audits report no advisories; RustSec reports no vulnerability findings. Its five remaining warnings concern packages outside the Windows dependency graph.

## Availability and updates

This preview is for **Windows x64 only**. Users on 0.1.3 or 0.1.4 can update in-app after opting into preview updates. Users on 0.1.2 and earlier must install manually. The stable update channel remains inactive.

The installer carries the project's version-bound Tauri updater signature. It does not have a Windows Authenticode publisher signature, so Windows may display an unknown-publisher warning. The application has not undergone an independent security audit. macOS and Linux have no installers or real-device support validation.

Third-party notices, verified MPL source archives and checksums accompany the installer. See [SECURITY.md](https://github.com/YerayAlonso/super-ftp/blob/main/SECURITY.md), [preview limitations](https://github.com/YerayAlonso/super-ftp/blob/main/README.md#preview-limitations) and the [technical licensing review](https://github.com/YerayAlonso/super-ftp/blob/main/THIRD_PARTY_REVIEW.md) for scope and remaining limitations.

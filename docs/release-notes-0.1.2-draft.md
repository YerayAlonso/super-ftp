# Super FTP 0.1.2 — Windows preview candidate

Update checks are now integrated into Preferences, with the existing optional preview filter, release notes and GitHub releases link. Checks are manual, do not send credentials, and do not download or install software. Private GitHub releases cannot be checked without authentication.

This candidate is **Windows x64 only** and unsigned; Windows may display a warning. macOS and Linux have not been tested on real devices, and no installers for those platforms are provided. The application has not undergone an independent security audit.

An RSA dependency has an unresolved timing advisory; applicability to this application's SSH signing has not been established. Prefer Ed25519 for new connections where supported. See [SECURITY.md](https://github.com/YerayAlonso/super-ftp/blob/main/SECURITY.md) for the advisory and [README.md](https://github.com/YerayAlonso/super-ftp/blob/main/README.md#preview-limitations) for transfer and deletion limitations.

Third-party licensing and asset provenance review remain open; see [THIRD_PARTY_REVIEW.md](https://github.com/YerayAlonso/super-ftp/blob/main/THIRD_PARTY_REVIEW.md). Verify notices and MPL source assets against the exact build before distributing it.

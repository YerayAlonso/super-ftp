# Signed update release record

## Current production status (2026-11-17)

- Windows preview updates point to the signed [v0.1.5 prerelease](https://github.com/YerayAlonso/super-ftp/releases/tag/v0.1.5) through `updates/preview.json` on `main`. Stable updates remain disabled.
- The exact installer passed project-key signature verification, version binding, modified-byte rejection and wrong-version rejection. Installer SHA-256: `18c29039571d48233a9059873ddf27595cca6946e5e3ab750de68c311d7e7d87`.
- All ten public release assets were downloaded without authentication and matched the verified artifacts and checksums. The application's download helper also verified the real public HTTPS installer and signature before feed activation.
- [Isolated upgrade run 44073399422](https://github.com/YerayAlonso/super-ftp/actions/runs/44073399422) passed the exact signed upgrade from public 0.1.4, installed-version check, executable comparison and uninstall while the candidate was still a draft.
- [Desktop CI 44038109516](https://github.com/YerayAlonso/super-ftp/actions/runs/44038109516) passed Windows, macOS, Linux and UI jobs, including real OpenSSH interoperability and Windows credential-store/agent integration. [Security CI 44039028136](https://github.com/YerayAlonso/super-ftp/actions/runs/44039028136) passed full/production npm and RustSec audits.
- The pre-publication audit recheck found no vulnerabilities and six RustSec warnings, including the Windows chrono maintenance warning through russh-sftp. See SECURITY.md for the current assessment. The RSA advisory is corrected in 0.1.5; older installers remain affected.
- Users on 0.1.3 or 0.1.4 can update in-app after opting into previews. Users on 0.1.2 and earlier must install manually because those builds lack the updater public key. Installation remains blocked during active transfers or conflict decisions.
- Windows Authenticode signing, independent review and remaining stable-release work are documented in `docs/stable-readiness.md`. There are no macOS or Linux installers.

## Previous activation (2026-11-13)

The verified v0.1.4 preview used installer SHA-256 `4f524f94ba5873039e4c933120244a60cfd7c9864b3916ab80119322e24da626`. [Run 43391642757](https://github.com/YerayAlonso/super-ftp/actions/runs/43391642757) passed the isolated upgrade from 0.1.3. Public assets and the production HTTPS feed were verified after activation; stable updates stayed disabled.

## Previous activation (2026-11-05)

The initial signed public preview feed targeted v0.1.3, with installer SHA-256 `f91147190098f35413a973905a249be77c7471864af0bd7c752ca67abe1e7b2c`. Isolated upgrade run `42002463936` verified the upgrade from 0.1.2. The temporary `v0.1.3-test.1` release and `test-feed-013` branch were removed after verification.

## Security and publishing notes

The signing private key and password remain under the maintainer's control and must never be committed or uploaded. Tauri updater signatures are separate from Windows Authenticode; Windows may warn that the installer publisher is unknown. Super FTP remains preview software without an independent security audit. Startup checks contact GitHub and disclose ordinary network metadata such as the user's IP address.

The manifest generator (`scripts/create-update-manifest.mjs`) formats version-bound metadata but does not authenticate the signature or installer. Before publishing a feed, verify the exact signed artifact with the project's embedded updater public key, test rejection of modified bytes and wrong versions, and verify the public release asset hash. A GitHub release by itself does not activate an update channel; only the signed manifest does.

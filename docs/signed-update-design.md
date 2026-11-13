# Signed update release record

## Current production status (2026-11-13)

- Windows preview updates point to the signed [v0.1.4 prerelease](https://github.com/YerayAlonso/super-ftp/releases/tag/v0.1.4) through `updates/preview.json` on `main`. The stable feed remains absent; stable updates are disabled.
- The exact installer was verified with the existing embedded project updater public key, version binding, modified-byte rejection and wrong-version rejection. Installer SHA-256: `4f524f94ba5873039e4c933120244a60cfd7c9864b3916ab80119322e24da626`.
- Every published release asset was downloaded without authentication and matched the local verified artifact and published checksums. The application download helper also verified the real public HTTPS installer and signature.
- The production raw GitHub preview URL returned the exact verified 0.1.4 manifest. The HTTPS manifest/download fixture test then passed against that production URL, including signature verification and modified-byte and wrong-version rejection. The stable URL returned HTTP 404.
- [GitHub Actions run 43391642757](https://github.com/YerayAlonso/super-ftp/actions/runs/43391642757) passed the isolated Windows upgrade from public 0.1.3 using the exact project-signed 0.1.4 candidate while it was still a draft. It checked the installed version, matched the installed executable to the candidate and uninstalled it successfully.
- [Desktop CI run 43390804443](https://github.com/YerayAlonso/super-ftp/actions/runs/43390804443) passed checks, tests, Clippy and builds on Windows, macOS and Linux, as well as UI tests. There are still no macOS or Linux installers or real-device validation for those platforms.
- Users on 0.1.3 can update in-app after opting into previews. Users on 0.1.2 and earlier must install manually because those builds lack the updater public key. Installation is blocked while transfers are active or waiting on a conflict decision.
- The unresolved RSA advisory remains documented. RSA compatibility is retained and Ed25519 is recommended; see SECURITY.md and the 0.1.4 preparation record for current audit results.

## Previous activation (2026-11-05)

The initial signed public preview feed targeted v0.1.3, with installer SHA-256 `f91147190098f35413a973905a249be77c7471864af0bd7c752ca67abe1e7b2c`. Isolated upgrade run `42002463936` verified the upgrade from 0.1.2. The temporary `v0.1.3-test.1` release and `test-feed-013` branch were removed after verification.

## Security and publishing notes

The signing private key and password remain under the maintainer's control and must never be committed or uploaded. Tauri updater signatures are separate from Windows Authenticode; Windows may warn that the installer publisher is unknown. Super FTP remains preview software without an independent security audit. Startup checks contact GitHub and disclose ordinary network metadata such as the user's IP address.

The manifest generator (`scripts/create-update-manifest.mjs`) formats version-bound metadata but does not authenticate the signature or installer. Before publishing a feed, verify the exact signed artifact with the project's embedded updater public key, test rejection of modified bytes and wrong versions, and verify the public release asset hash. A GitHub release by itself does not activate an update channel; only the signed manifest does.

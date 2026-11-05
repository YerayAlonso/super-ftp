# Signed update release record

## Current production status (2026-11-05)

- Windows preview updates are active through `updates/preview.json` on `main` and point to the signed `v0.1.3` prerelease.
- The public raw GitHub preview URL returns HTTP 200 with the exact verified manifest. The stable feed `updates/stable.json` remains absent (HTTP 404).
- The exact installer was verified with the project updater public key, version binding, altered-byte rejection, and SHA-256 `f91147190098f35413a973905a249be77c7471864af0bd7c752ca67abe1e7b2c`.
- The published release assets were downloaded again and every listed SHA-256 checksum passed.
- The HTTPS manifest/download fixture test passed against the production preview URL, verifying the signature and downloaded installer bytes.
- GitHub Actions run `42002463936` passed the isolated Windows upgrade from public preview `0.1.2` using the exact project-key-signed `0.1.3` candidate and the temporary HTTPS test feed. It checked the installed version and executable.
- Users on `0.1.2` and earlier must install `0.1.3` manually; those builds do not contain the updater public key. Later Windows previews can update in-app after opting in. Installation is blocked while transfers are active or waiting on a conflict decision.
- `v0.1.3-test.1` and its temporary feed were test-only and have been removed. The branch and tag are deleted; the production `v0.1.3` preview remains published.
- Stable updates remain disabled. There are no macOS or Linux installers. The unresolved RSA advisory remains visible; RSA compatibility is retained and Ed25519 is recommended.

## Security and publishing notes

The signing private key and password remain under the maintainer's control and must never be committed or uploaded. Tauri updater signatures are separate from Windows Authenticode; Windows may warn that the installer publisher is unknown. Super FTP remains preview software without an independent security audit. Startup checks contact GitHub and disclose ordinary network metadata such as the user's IP address.

The manifest generator (`scripts/create-update-manifest.mjs`) formats version-bound metadata but does not authenticate the signature or installer. Before publishing a feed, verify the exact signed artifact with the project's embedded updater public key, test rejection of modified bytes and wrong versions, and verify the public release asset hash. A GitHub release by itself does not activate an update channel; only the signed manifest does.

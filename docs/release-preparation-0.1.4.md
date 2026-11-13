# 0.1.4 Windows preview preparation

Prepared on 2026-11-13 from `main` at `523dbd60cf2896e19c314d9ebe3168c5e1fee9a4`, with the local release-preparation changes. This is a local candidate, not a published release or an activated update.

## Contents

The candidate includes the configurable connection timeout, phase-specific connection errors and revised connection dialog added after public `v0.1.3` (`4a4ee53366dedb1103777bd2236c2fc9abecb08d`). Version 0.1.4 is synchronized across npm, Cargo and Tauri metadata and lockfiles. License notices and the Windows dependency inventory were regenerated against those lockfiles.

The signed-upgrade workflow now reads the candidate version from Tauri metadata and starts from public 0.1.3. It accepts a project-signed release tag, an independently verified installer SHA-256 and an optional HTTPS manifest, and can retrieve a draft candidate without publishing it. The Rust isolated upgrade test uses the package version by default and accepts an explicit fixture version.

## Validation

- Node.js 24.16.0, Rust 1.99.1 and Visual Studio 2022 Build Tools on Windows x64.
- `npm ci`: installed the locked dependency versions.
- `npm run verify` under Node 24: passed, including 17 unit tests, the manifest test and the production web build. Vite reports the existing large license-notice chunk warning.
- `npm run test:ui`: 26 passed in Microsoft Edge, including desktop and narrow/enlarged connection layouts.
- Cargo formatting, 30 Rust library tests and Clippy with warnings denied: passed. Two tests requiring signed fixtures or an isolated installer runner were skipped.
- Frontend notice verification: all six bundled packages covered.
- MPL sources: all five registry archives matched Cargo.lock and the local Cargo registry source files.
- All five PowerShell blocks in the modified signed-upgrade workflow parsed successfully. The workflow itself has not run on GitHub Actions for this candidate.
- `npm run desktop:build -- --bundles nsis` under Node 24: passed. Final Windows frontend assets were rebuilt before the final installer was bundled.
- The executable extracted from the final NSIS installer reports product version 0.1.4. Its only differences from the build executable are Tauri's three bundle-type bytes (`UNK` to `NSS`).
- `npm run test:native` against that extracted executable: passed local browsing, invalid-path rejection, language persistence, context menus and local operations, notices and update-check response, with zero renderer errors. No installer was run on the developer's Windows installation.
- `npm audit --omit=dev`: no advisories. Full `npm audit`: two low-severity findings for the existing development-only deepmerge advisory and its Svelte/Vite dependent, with no automated fix available.
- `cargo audit`: fails on the known RSA timing advisory in `rsa 0.10.0-rc.18`; it also reports five dependency warnings. RustSec now identifies RSA 0.10.0 as patched, but both russh 0.65.0 and 0.65.1 pin the affected release candidate. See SECURITY.md for the updated dependency constraint and limitations.

## Local artifacts

The ignored directory `src-tauri/target/release/candidate-0.1.4/` contains the versioned installer, release notes, third-party notices, verified MPL source ZIP, installer SHA-256, packaged-executable SHA-256, supplementary checksums and source-state record. The extracted executable used for the native smoke test remains in its `extracted/` subdirectory. Build artifacts are local and are not committed to Git.

The exact installer has been signed with the existing project updater key and version 0.1.4. The project-key fixture test passed signature verification, modified-byte rejection and wrong-version rejection. Installer SHA-256: `4f524f94ba5873039e4c933120244a60cfd7c9864b3916ab80119322e24da626`. A candidate preview manifest has been generated locally; it is not advertised by the production feed.

## Before publication

Keep `updates/preview.json` on the verified public 0.1.3 installer until the new exact artifact is signed and verified. Do not reuse the 0.1.3 signature for this installer.

1. Sign the exact 0.1.4 installer with the maintainer's existing updater key and version binding; retain the embedded public key.
2. Verify the project signature, modified-byte rejection and wrong-version rejection. Run the isolated upgrade test against the exact candidate, starting from public 0.1.3.
3. Review the outstanding dependency audit findings and notices before distribution. The candidate retains the documented preview limitations.
4. Publish the versioned installer, updater signature, checksums, notices and verified MPL sources together with the release notes. Download the public assets again and verify their hashes.
5. Generate and verify the 0.1.4 preview manifest against those exact public assets, then update the preview feed. Stable updates remain disabled.

Publication is proceeding through a draft release and an isolated upgrade check before making the release and preview feed available publicly.

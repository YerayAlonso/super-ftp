# 0.1.4 Windows preview preparation

Prepared on 2026-11-13 from `main` at `523dbd60cf2896e19c314d9ebe3168c5e1fee9a4`, with the local release-preparation changes. The verified Windows preview is now published as [v0.1.4](https://github.com/YerayAlonso/super-ftp/releases/tag/v0.1.4), with the preview update feed activated after public artifact verification. The release tag targets `795c63f`, whose only application-source difference from the build commit `9a04edb` is the isolated test correction.

## Contents

The candidate includes the configurable connection timeout, phase-specific connection errors and revised connection dialog added after public `v0.1.3` (`4a4ee53366dedb1103777bd2236c2fc9abecb08d`). Version 0.1.4 is synchronized across npm, Cargo and Tauri metadata and lockfiles. License notices and the Windows dependency inventory were regenerated against those lockfiles.

The signed-upgrade workflow now reads the candidate version from Tauri metadata and starts from public 0.1.3. It accepts a project-signed release tag, an independently verified installer SHA-256 and an optional HTTPS manifest, and can retrieve a draft candidate without publishing it. The Rust isolated upgrade test uses the package version by default and accepts an explicit fixture version.

## Validation

- Node.js 24.16.0, Rust 1.99.1 and Visual Studio 2022 Build Tools on Windows x64.
- `npm ci`: installed the locked dependency versions.
- `npm run verify` under Node 24: passed, including 17 unit tests, the manifest test and the production web build. Vite reports the existing large license-notice chunk warning.
- `npm run test:ui`: 26 passed in Microsoft Edge, including desktop and narrow/enlarged connection layouts.
- Cargo formatting, 30 Rust library tests and Clippy with warnings denied: passed. The normal suite skips two opt-in signed-fixture and isolated-installer tests; both were subsequently run explicitly and passed.
- Frontend notice verification: all six bundled packages covered.
- MPL sources: all five registry archives matched Cargo.lock and the local Cargo registry source files.
- All five PowerShell blocks in the modified signed-upgrade workflow parsed successfully. The project-candidate workflow subsequently passed in the isolated run linked below.
- `npm run desktop:build -- --bundles nsis` under Node 24: passed. Final Windows frontend assets were rebuilt before the final installer was bundled.
- The executable extracted from the final NSIS installer reports product version 0.1.4. Its only differences from the build executable are Tauri's three bundle-type bytes (`UNK` to `NSS`).
- `npm run test:native` against that extracted executable: passed local browsing, invalid-path rejection, language persistence, context menus and local operations, notices and update-check response, with zero renderer errors. No installer was run on the developer's Windows installation.
- The exact project-signed candidate passed the [isolated Windows upgrade run](https://github.com/YerayAlonso/super-ftp/actions/runs/43391642757) from public 0.1.3. The runner checked installed version 0.1.4, compared the installed executable with the candidate's extracted executable and successfully uninstalled it. The test-only loopback fixture is validated separately from the public HTTPS installer URL policy; production restrictions remain unchanged.
- `npm audit --omit=dev`: no advisories. Full `npm audit`: two low-severity findings for the existing development-only deepmerge advisory and its Svelte/Vite dependent, with no automated fix available.
- `cargo audit`: fails on the known RSA timing advisory in `rsa 0.10.0-rc.18`; it also reports five dependency warnings. RustSec now identifies RSA 0.10.0 as patched, but both russh 0.65.0 and 0.65.1 pin the affected release candidate. See SECURITY.md for the updated dependency constraint and limitations.

## Local artifacts

The ignored directory `src-tauri/target/release/candidate-0.1.4/` contains the versioned installer, release notes, third-party notices, verified MPL source ZIP, installer SHA-256, packaged-executable SHA-256, supplementary checksums and source-state record. The extracted executable used for the native smoke test remains in its `extracted/` subdirectory. Build artifacts are not committed to Git; the verified release files are also attached to the public release.

The exact installer has been signed with the existing project updater key and version 0.1.4. The project-key fixture test passed signature verification, modified-byte rejection and wrong-version rejection. Installer SHA-256: `4f524f94ba5873039e4c933120244a60cfd7c9864b3916ab80119322e24da626`. The verified preview manifest is published through the production preview feed; stable updates remain disabled.

## Publication verification

- [Desktop CI run 43390804443](https://github.com/YerayAlonso/super-ftp/actions/runs/43390804443) passed all four jobs: Windows, macOS, Linux and UI tests.
- The exact candidate passed the isolated upgrade test before publication.
- All ten public release assets were downloaded without authentication and compared byte-for-byte by SHA-256 with the local verified files. Both checksum manifests passed.
- The project-key fixture test also downloaded the real public HTTPS installer through the application's download helper and passed signature, modified-byte and wrong-version checks.
- The preview feed was updated only after these checks; no stable feed was created. See `docs/signed-update-design.md` for the current production record.
- After activation, the production preview URL returned the exact verified manifest and the HTTPS manifest/download fixture test passed against it. The stable URL returned HTTP 404. Tag `v0.1.4` was fetched and confirmed to target `795c63ffd4b596ceb8597d89adeb52ffd48d47ee`.

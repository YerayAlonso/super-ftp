# 0.1.5 Windows preview preparation

Prepared on 2026-11-17 from the verified development candidate. Application build sources are commit `30427391ddb7294893137cfd298463aabf7e1004`; subsequent commits before release preparation change only documentation and audit policy. The candidate installer bytes are unchanged from the native smoke-tested readiness artifact.

## Candidate identity and evidence

- Version: 0.1.5, Windows x64 NSIS installer.
- Installer SHA-256: `18c29039571d48233a9059873ddf27595cca6946e5e3ab750de68c311d7e7d87`.
- [Desktop CI 44038109516](https://github.com/YerayAlonso/super-ftp/actions/runs/44038109516): all Windows, macOS, Linux and UI jobs passed, including real OpenSSH interoperability and Windows credential-store/agent integration.
- [Security CI 44039028136](https://github.com/YerayAlonso/super-ftp/actions/runs/44039028136): RustSec, production npm and full npm audits passed. Five RustSec warnings concern packages absent from the Windows dependency graph.
- The pre-publication audit recheck still reports no vulnerabilities; its newer advisory database adds a sixth warning for unmaintained chrono, used on Windows through russh-sftp 3.0.1. There is no newer russh-sftp registry release available, so this is recorded in the release notes and SECURITY.md rather than suppressed. Full and production npm audits remain clear.
- The executable extracted from the candidate reports 0.1.5 and passed the native smoke test with zero renderer errors. Its complete displayed notice text matched the generated source hash. Its only differences from the build executable are Tauri's NSIS bundle-type marker bytes.
- License verification checked 649 files across 374 third-party Rust packages and all six bundled frontend packages. All five MPL archives matched Cargo.lock and the local Cargo registry files.

## Publication sequence

1. Sign this exact candidate with the existing project updater key and version binding. Do not upload the private key or password.
2. Verify the signature against the embedded public key and test modified-byte and wrong-version rejection.
3. Upload the candidate to a draft release and run the isolated upgrade from public 0.1.4. Check installed version, executable hash and uninstall.
4. Publish as a Windows preview, download every public asset without authentication and verify checksums and the actual signed HTTPS download.
5. Activate the preview feed only after those checks, and verify its production URL and signature. Keep stable updates disabled.

This updater signature is separate from Windows Authenticode. Stable-release decisions and remaining external work are recorded in `docs/stable-readiness.md`.

## Current status

The verified [0.1.5 Windows preview](https://github.com/YerayAlonso/super-ftp/releases/tag/v0.1.5) is published. The project-key fixture test passed signature verification, modified-byte rejection and wrong-version rejection. [Isolated upgrade run 44073399422](https://github.com/YerayAlonso/super-ftp/actions/runs/44073399422) passed the exact upgrade from public 0.1.4, installed-version check, executable comparison and uninstall. All ten public assets were downloaded without authentication and matched the verified local files and both checksum manifests. The application download helper also verified the actual public HTTPS installer and signature. The preview feed is activated only after those checks; stable remains disabled.

After activation, the production raw GitHub preview URL served the exact verified 0.1.5 manifest. The HTTPS manifest/download fixture passed against that production URL, including signature, modified-byte and wrong-version checks. The stable feed returned HTTP 404. Tag `v0.1.5` was fetched and confirmed to target `f93ad0749421a7cb8ef3a430433aa414be14f3a5`.

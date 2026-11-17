# Windows stable-release readiness

Updated 2026-11-17. These changes are published in the **0.1.5 Windows preview**, with its signed preview update feed activated after artifact and upgrade validation. This record separates engineering evidence from a decision to declare a supported Windows release stable.

## Changes completed in the source

- Replaced the affected RSA release candidate with official `russh 0.65.2`, `rsa 0.10.0` and `ssh-key 0.7.0-rc.12` registry releases. No fork, moving Git branch or disabled RSA feature is needed. Cargo.lock retains exact versions and checksums.
- Added real SSH/SFTP tests for encrypted RSA client keys and RSA hosts, incorrect passphrases and Ed25519 keys.
- Added fault injection for failed upload writes, failed publication, failed restoration and failed backup cleanup. Tests verify original bytes remain either at the original path or at the recovery path reported to the user, and temporary partial files are removed when the connection remains available.
- Added opt-in native Windows tests for Credential Manager and the system OpenSSH agent. They use temporary generated entries/keys, clean them up and verify existing agent identities remain unchanged. CI runs the same tests on disposable Windows runners.
- Added a real OpenSSH server interoperability fixture for isolated Linux CI: generated RSA client key, pinned generated host key, loopback-only listener, explicit overwrite and byte-for-byte round trip through the application's backend. The fixture refuses to run privileged server setup on developer machines.
- Expanded license generation with crate-specific license files and recursive NOTICE discovery; 49 additional texts are preserved. The verifier checks 649 files across 374 third-party Rust packages and checks exact source identities against metadata. The native smoke test compares the entire rendered notice snapshot with the source hash.
- Dependency changes now trigger RustSec, production npm and full npm audits on pushes and pull requests, as well as scheduled/manual runs. No advisory is ignored to make the new checks pass.

## Internal security review

Reviewed the source paths for SSH host trust and authentication (`ssh.rs`), credential identity and persistence (`storage.rs`, `commands.rs`), destructive path validation (`paths.rs`, `commands.rs`), transfer staging and rollback (`transfer.rs`), signed updates and transfer exclusion (`updates.rs`), and the Tauri capabilities/CSP. The concrete dependency problem was corrected and the recovery/authentication evidence was expanded. This is an internal engineering review, not an independent security audit or proof that undiscovered defects are absent.

Existing safeguards and limits remain: unknown/changed host keys block authentication, secrets are not stored in configuration or browser storage, remote names are rendered as text, generic filesystem/shell/HTTP plugins are not exposed, overwrites require explicit conflict decisions, and update artifacts require version-bound project signatures. Remote SFTP v3 replacement is not atomic. A process crash or lost connection can still leave temporary or recovery files; cleanup is best effort. Another local process with the same permissions is outside the path-validation threat boundary.

## Validation record

- Local checks on Windows 11 Pro build 10.0.26300 passed: Svelte/TypeScript, 17 unit tests, the manifest test, production web build and 26 UI tests in Edge.
- The Rust library suite passed 33 tests. Credential Manager and real Windows OpenSSH agent tests were then run explicitly and passed; existing agent identities were preserved. Formatting and Clippy with warnings denied passed.
- RustSec exited successfully with no vulnerability findings and five warnings. Both production and full npm audits found no advisories on 2026-11-17. GitHub withdrew the previous deepmerge advisory on 2026-11-16; package versions were not changed to hide it.
- A later pre-publication recheck on the same date still found no vulnerabilities but reported a sixth warning, RUSTSEC-2026-0543: chrono is unmaintained and is present on Windows through russh-sftp 3.0.1. The earlier CI results above reflect their advisory database snapshot; the release notes and SECURITY.md record this newer finding.
- Rust license verification checked 649 files across 374 third-party packages; frontend verification covered all six bundled packages. All five MPL archives matched Cargo.lock and the local Cargo registry source files.
- [Desktop CI run 44038109516](https://github.com/YerayAlonso/super-ftp/actions/runs/44038109516) passed all four jobs: Linux, macOS, Windows and UI. The Linux job passed the actual system OpenSSH server test, and the Windows job passed the real credential-store and agent tests, full Rust license checks and Clippy.
- [Security run 44039028136](https://github.com/YerayAlonso/super-ftp/actions/runs/44039028136) passed RustSec, production npm and full npm audits with the final audit workflow. Application sources were tested in commit `3042739`; the later audit-policy change did not change the application binary.
- The Windows NSIS development candidate reports version 0.1.5. Its extracted executable matches the build apart from Tauri's expected three-byte NSIS bundle marker. The native smoke test passed against that extracted executable with zero renderer errors; the entire displayed license text matched the generated source hash.
- The original local readiness snapshot remains in `src-tauri/target/release/readiness-0.1.5/`. Its exact installer bytes were subsequently copied to `candidate-0.1.5`, signed with the project updater key and [published as v0.1.5](https://github.com/YerayAlonso/super-ftp/releases/tag/v0.1.5). Installer SHA-256: `18c29039571d48233a9059873ddf27595cca6946e5e3ab750de68c311d7e7d87`. There is no Windows Authenticode signature.
- [Isolated upgrade run 44073399422](https://github.com/YerayAlonso/super-ftp/actions/runs/44073399422) passed the exact signed 0.1.4-to-0.1.5 upgrade, installed-version check, executable comparison and uninstall. Every public asset was then downloaded without authentication and matched its verified checksum; the application's HTTPS download helper also verified the public installer and signature.

The public release must not be relabeled stable or its update feed switched just because these source changes exist.

## Remaining work outside this patch

| Item | What remains |
| --- | --- |
| Windows publisher identity | No valid current-user Authenticode code-signing certificate was found. Obtaining/connecting a certificate requires the maintainer/provider; updater signatures do not replace it. |
| Real-device and sustained use | Native drag and drop, varied Windows setups and external server implementations still need a recorded usage matrix and a period of real use without serious regressions. The automated OpenSSH fixture covers one implementation, not every server. |
| Independent security review | Arrange an external review if that is part of the project's stable-release acceptance criteria. The internal source review does not claim independence. |
| Rights and licensing assessment | Technical collection, source checks and packaged notices can be verified automatically. Independent asset provenance, provider terms and a legal compliance determination cannot be established by these tests. See THIRD_PARTY_REVIEW.md. |
| Non-Windows dependency warnings | RustSec reports five warnings, including Linux GTK unsoundness and unmaintained packages. All five affected package names are absent from the resolved Windows graph. They remain visible and need assessment before distributing other platforms. |
| Windows dependency maintenance | The release-day recheck adds the chrono maintenance warning through russh-sftp 3.0.1. Chrono is unconditional there, and no newer russh-sftp registry release is available. Track an upstream migration or review a library change; retain the warning until resolved. |
| Stable release decision | The verified 0.1.5 preview is published. Stable remains a separate decision and channel after assessing the remaining work; 0.1.4 and earlier installers still contain the old RSA dependency. |

macOS/Linux installers, FTP/FTPS, queue persistence, synchronization and resumable transfers are not prerequisites for a stable **Windows SFTP** release with the existing documented scope.

# Security

Super FTP is a preview. It has not undergone an independent security audit and cannot protect against a compromised operating system or user account.

## Implemented protections

- SSH/SFTP runs in the Rust backend. The app does not execute remote commands or use unencrypted FTP.
- The server public key is verified before authentication. First trust requires an explicit decision after checking a SHA-256 fingerprint through an independent channel. A changed known key is blocked.
- Pending host-key verifications are single-use tokens that expire after five minutes. The frontend cannot submit an arbitrary key or replace a known key through the trust command.
- Persistent secrets use the system credential store, bound to profile, host, port, user, authentication method and key file. Changing the identity does not silently reuse the old password.
- Rust secrets use `Zeroizing` where practical. This does **not** guarantee erasure of copies in libraries, IPC, JavaScript, swap or memory dumps. A secret entered in the UI necessarily exists temporarily in the webview.
- Configuration writes use a synchronized temporary file and atomic replacement. The Unix configuration directory has restrictive permissions. Invalid configuration is not overwritten automatically.
- The webview loads bundled code under a CSP without remote origins, iframes, external forms or objects. Generic shell, HTTP, filesystem and window-creation plugins are not exposed. Update commands contact fixed GitHub release and update-manifest URLs; on explicit installation they download a bounded HTTPS installer and require an update signature bound to its version. A separate command opens the fixed repository releases page. Rust commands are nevertheless powerful: this is a file manager running with the user's permissions.
- Server-provided file names are rendered as text, never as HTML. The application does not use analytics or telemetry. At startup and when the user requests an update check it contacts GitHub without credentials; GitHub receives the request and network metadata such as the IP address. Operating-system and WebView2 updates are outside that statement.
- Transfers stream in bounded chunks with limited concurrency. Temporary names are exclusive; a conflict requires explicit overwrite, rename, skip or cancellation. Filenames are validated to reject traversal, path separators, controls, Windows device names and NTFS alternate streams.
- Recursive transfers are bounded and do not follow symbolic links. Existing destination directories that are symbolic links are rejected.

## Overwrite and recovery limitations

- Overwrite is performed only after the user explicitly chooses it for a specific conflict. A changed destination is checked again before publication, but metadata checks are not a cryptographic compare-and-swap and cannot defeat every concurrent modification.
- Local downloads use a temporary sibling file. SFTP v3 does not provide a portable atomic replace, so a remote overwrite first moves the original to a unique `.superftp-*.backup`, then publishes the completed temporary file. If publication fails, restoration is attempted. Process termination, a network failure, concurrent changes or failed cleanup can leave a backup or partial file; inspect and recover it manually. No cross-server atomicity guarantee is made.
- Existing directories are merged during recursive transfers. Conflicting files are handled individually. Directory/file type conflicts and symbolic-link destinations are rejected rather than overwritten.
- These path checks are not a capability-based sandbox against another local process with the same user permissions. Do not transfer into directories controlled by an untrusted local user. A verified malicious server can still lie about its content and metadata.
- The SFTP library materializes each remote directory in memory. There is no independently negotiated end-to-end file checksum; transport integrity is provided by SSH.
- Agent forwarding, interactive multi-factor authentication, host certificates/CA trust and advanced OpenSSH configuration are not implemented.
- Windows installers do not have an Authenticode publisher signature. The separate Tauri updater signature is version-bound and required for in-app installation. Dependency selection and automated tests are not a substitute for a security review.

## Dependencies and reporting

Run `npm audit`, `npm audit --omit=dev` and a RustSec audit (`cargo audit`) before release. As of 2026-11-17, both npm audits report no advisories. The previously reported development-only [deepmerge advisory GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4) was withdrawn upstream on 2026-11-16; no package change or advisory suppression was used to clear it. Recheck current results before distribution.

### RSA timing advisory

**Fixed in the 0.1.5 development sources; not yet released.**

Published versions through **0.1.4** include `rsa 0.10.0-rc.18`, affected by [RUSTSEC-2023-0071 / CVE-2023-49092](https://rustsec.org/advisories/RUSTSEC-2023-0071.html). Those installers remain affected; changing this repository does not update an installed application. Prefer Ed25519 where supported until using a release containing the fix. The advisory's practical exploitability in this client's SSH signing path has not been established.

The 0.1.5 development lockfile uses the official crates.io release **russh 0.65.2**, patched **rsa 0.10.0** and **ssh-key 0.7.0-rc.12**, retaining RSA compatibility. This release includes the merged [upstream RSA correction #821](https://github.com/warp-tech/russh/pull/821). The earlier russh releases 0.65.0 and 0.65.1 still require the affected RSA release candidate. All production dependencies use locked registry sources; no temporary Git dependency or private fork is required.

The pre-publication RustSec recheck on 2026-11-17 exits successfully with no vulnerability findings and six warnings. Five concern proc-macro-error, zbus_names, zvariant, zvariant_derive and glib (including an unsoundness warning); these packages do not occur in the resolved Windows graph. The additional [chrono maintenance advisory RUSTSEC-2026-0543](https://github.com/RustSec/advisory-db/blob/main/crates/chrono/RUSTSEC-2026-0543.md) does affect the Windows graph through russh-sftp 3.0.1. Chrono is an unconditional dependency of that library, all chrono versions are marked unmaintained, and no newer russh-sftp registry release is available. This warning requires upstream migration or a separately reviewed library change; it is not ignored. Successful audit exit status is not a claim that every dependency or platform has received a clean security review.

Regression tests authenticate with an encrypted RSA client key to an RSA host, reject an incorrect passphrase before public-key authentication, and retain Ed25519 authentication. Additional fault tests check interrupted writes, publication rollback, failed restoration and retained recovery backups. Native Windows opt-in tests check the credential store and OpenSSH agent using only uniquely generated test entries and keys.

SSH agent authentication delegates signing to the external agent; its implementation remains outside this application's control. Dependency changes now trigger RustSec, production npm and full npm audits on pushes and pull requests, as well as scheduled/manual runs. No advisory is ignored to pass these checks.

To report a vulnerability privately, use [GitHub's private vulnerability reporting form](https://github.com/YerayAlonso/super-ftp/security/advisories/new). Do not post vulnerability details, passwords, private keys, memory dumps or private server configuration in a public issue. The maintainer will review reports through GitHub; availability of the form requires a GitHub account.

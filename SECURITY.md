# Security

Super FTP is a preview. It has not undergone an independent security audit and cannot protect against a compromised operating system or user account.

## Implemented protections

- SSH/SFTP runs in the Rust backend. The app does not execute remote commands or use unencrypted FTP.
- The server public key is verified before authentication. First trust requires an explicit decision after checking a SHA-256 fingerprint through an independent channel. A changed known key is blocked.
- Pending host-key verifications are single-use tokens that expire after five minutes. The frontend cannot submit an arbitrary key or replace a known key through the trust command.
- Persistent secrets use the system credential store, bound to profile, host, port, user, authentication method and key file. Changing the identity does not silently reuse the old password.
- Rust secrets use `Zeroizing` where practical. This does **not** guarantee erasure of copies in libraries, IPC, JavaScript, swap or memory dumps. A secret entered in the UI necessarily exists temporarily in the webview.
- Configuration writes use a synchronized temporary file and atomic replacement. The Unix configuration directory has restrictive permissions. Invalid configuration is not overwritten automatically.
- The webview loads bundled code under a CSP without remote origins, iframes, external forms or objects. Shell, HTTP, filesystem and window-creation plugins are not exposed. Rust commands are nevertheless powerful: this is a file manager running with the user's permissions.
- Server-provided file names are rendered as text, never as HTML. The production UI does not use analytics, telemetry or cloud services. Operating-system and WebView2 updates are outside that statement.
- Transfers stream in bounded chunks with limited concurrency. Temporary names are exclusive; a conflict requires explicit overwrite, rename, skip or cancellation. Filenames are validated to reject traversal, path separators, controls, Windows device names and NTFS alternate streams.
- Recursive transfers are bounded and do not follow symbolic links. Existing destination directories that are symbolic links are rejected.

## Overwrite and recovery limitations

- Overwrite is performed only after the user explicitly chooses it for a specific conflict. A changed destination is checked again before publication, but metadata checks are not a cryptographic compare-and-swap and cannot defeat every concurrent modification.
- Local downloads use a temporary sibling file. SFTP v3 does not provide a portable atomic replace, so a remote overwrite first moves the original to a unique `.superftp-*.backup`, then publishes the completed temporary file. If publication fails, restoration is attempted. Process termination, a network failure, concurrent changes or failed cleanup can leave a backup or partial file; inspect and recover it manually. No cross-server atomicity guarantee is made.
- Existing directories are merged during recursive transfers. Conflicting files are handled individually. Directory/file type conflicts and symbolic-link destinations are rejected rather than overwritten.
- These path checks are not a capability-based sandbox against another local process with the same user permissions. Do not transfer into directories controlled by an untrusted local user. A verified malicious server can still lie about its content and metadata.
- The SFTP library materializes each remote directory in memory. There is no independently negotiated end-to-end file checksum; transport integrity is provided by SSH.
- Agent forwarding, interactive multi-factor authentication, host certificates/CA trust and advanced OpenSSH configuration are not implemented.
- The preview installers are unsigned and not notarized. Dependency selection and automated tests are not a substitute for a security review.

## Dependencies and reporting

Run `npm audit`, `npm audit --omit=dev` and a RustSec audit (`cargo audit`) before release. At the time this preview was prepared, the full npm audit reported a low-severity `deepmerge` advisory through the development-only Svelte/Vite plugin (GHSA-gcqx-w48m-w2h4), with no upstream fix available; the production-only npm audit reported no advisories. Recheck current results before distribution.

Do not publish passwords, private keys, memory dumps or private server configuration in an issue. The repository maintainer must enable GitHub private vulnerability reporting or provide another private contact before public launch. Until a private channel is established, do not post vulnerability details publicly; contact the maintainer privately if you have a secure way to do so.

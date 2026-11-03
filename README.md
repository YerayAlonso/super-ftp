# Super FTP

**Your files, your servers, one workspace.**

Super FTP is a desktop **SFTP** client built with Rust, Tauri 2 and Svelte 5. Browse local and remote files side by side, manage multiple sessions, and transfer files over SSH. It uses the operating system's webview rather than bundling a browser engine.

This is a **functional preview**, not an independently audited or signed production release. macOS and Linux still require real-device validation.

[Features](#features) · [Get started](#getting-started) · [Build from source](#development) · [Security](SECURITY.md) · [Contribute](CONTRIBUTING.md) · [Release checklist](RELEASING.md)

> **Looking for a download?** No public release is assumed by this repository. When a release is published, its assets and platform support will be described on [GitHub Releases](https://github.com/YerayAlonso/super-ftp/releases). Until then, build from source below.

## Features

- Save connection profiles with server, port, username, colour and starting folders.
- Authenticate with a password, SSH private key and optional passphrase, or the system SSH agent. Secrets can optionally be stored in the operating system's credential store; they are never saved in the configuration JSON or browser storage.
- Verify the SHA-256 host-key fingerprint before first connecting. Changed known keys block authentication until investigated.
- Open independent sessions in tabs, with separate navigation history and remote paths.
- Browse local and remote files in virtualized, searchable and sortable panes. Upload via drag and drop from the local pane or the system file manager.
- Transfer files and folders, including empty folders. The queue shows progress and speed, allows cancellation and manual retries, and runs at most three transfers globally and two per session.
- When a destination file exists, choose **Overwrite**, **Rename the new file**, **Skip this file** or **Cancel the entire transfer**. Conflicts in recursive transfers are handled individually. Renaming only changes the destination filename. Existing folders are merged; files within them are checked individually.
- Use context menus to create folders, view properties, transfer or delete files. Remote entries can also be renamed. Deletion is permanent, requires confirmation and removes folders with their contents. Remote deletion is limited to 100,000 items and 64 levels; failures can leave a partially deleted tree.
- Change languages instantly without interrupting sessions. The choice is stored locally; English is the default when no valid choice is saved. Application-owned backend errors have English fallbacks. OS, SSH and SFTP library details may use their original language.
- Adjust the interface text size from 80% to 130% in the top bar; the choice is stored locally. The whole interface scales together to keep controls and file lists aligned.

## Development

### Requirements

- Node.js 24 LTS and npm.
- Rust stable 1.90 or later.
- **Windows:** Microsoft C++ Build Tools with “Desktop development with C++”, a Windows SDK and WebView2. If `link.exe` is unavailable after installation, open a new terminal or Developer PowerShell for VS.
- **macOS:** Xcode Command Line Tools (`xcode-select --install`).
- **Ubuntu/Debian:** Tauri/WebKitGTK dependencies, for example:

  ```sh
  sudo apt-get install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev patchelf
  ```

```sh
npm ci
npm run desktop
```

`npm run dev` opens a **web-only interface preview**. It cannot access local files or connect to SFTP servers and does not simulate connections or save credentials.

### Build

```sh
npm run desktop:build
```

Build on each target operating system. Tauri uses WebView2 on Windows, WKWebView on macOS and WebKitGTK on Linux. Installer sizes do not necessarily include the system webview dependency.

For the Windows NSIS installer only:

```sh
npm run desktop:build -- --bundles nsis
```

Bundles are written to `src-tauri/target/release/bundle/`; the Windows binary is `src-tauri/target/release/super-ftp.exe`. Code signing and notarization are not configured.

### Tests

```sh
npm run verify                 # Svelte/TypeScript checks, unit tests and web build
npm run test:ui                # UI flows with a controlled IPC fixture
npm run test:native            # Windows smoke test of the existing release binary
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

UI tests use Edge on Windows. On Linux/macOS, first run `npx playwright install chromium`. Test fixtures in `e2e/` are not included in production builds. The Windows native smoke test temporarily enables WebView2 debugging on local port 9223 and uses an isolated WebView2 cache and temporary files. The app still reads the current user's configuration, but the test does not create or delete connection profiles or credentials.

Rust integration tests run a real SSH/SFTP server on loopback with an in-memory filesystem. They cover byte-for-byte transfers, host-key checks, cancellation, conflict decisions and overwrite protection. They do not require external servers or user credentials.

The CI workflow targets Windows, macOS and Linux; a workflow file alone does not mean those jobs have run successfully.

## Getting started

1. Select **New connection** and enter your SFTP details.
2. Choose password, SSH key or agent authentication. Optionally save the secret in the system credential store.
3. Select **Save and connect**. Before trusting a new server, verify its fingerprint with the administrator over an independent channel.
4. Open a remote folder and drag in local files. To download, select remote items and use the arrow toward the local pane, or double-click a file.
5. **New session** opens another tab, including for the same profile. Clicking a saved profile activates its first existing session; its menu can open another.

The language selector is in the top bar. The SSH agent uses `SSH_AUTH_SOCK` on macOS/Linux and the Windows OpenSSH `ssh-agent` named pipe on Windows. The app does not start an agent or enable agent forwarding.

## Data and security

`connections.json` contains profiles and public server keys only. Typical locations:

| Platform | Configuration |
| --- | --- |
| Windows | `%APPDATA%\dev.superftp.desktop\connections.json` |
| macOS | `~/Library/Application Support/dev.superftp.desktop/connections.json` |
| Linux | `$XDG_CONFIG_HOME/dev.superftp.desktop/connections.json` or `~/.config/dev.superftp.desktop/connections.json` |

Configuration writes use a temporary file and atomic replacement. Invalid configuration is not silently reset. Private SSH keys are read from the selected file, not copied into configuration.

Secrets use **Windows Credential Manager**, **macOS Keychain** or Linux **Secret Service**. If the store is unavailable or locked, the app reports an error and never falls back to plaintext. Disable secret storage to connect without persisting a secret.

See [SECURITY.md](SECURITY.md) for the security model and limitations.

## Preview limitations

- SFTP only: no FTP, FTPS, SCP, remote terminal or remote command execution.
- Up to 16 sessions, 200 profiles and 2,000 in-memory transfer tasks. The queue is not persisted; pause/resume and synchronization are not implemented.
- Recursive transfers are limited to 100,000 items and 64 levels. Symbolic links and special files are skipped rather than followed. Properties show metadata from the most recent listing.
- Names unsafe on Windows or containing path components are rejected when downloading on all platforms. Directories and symbolic links at file destinations are not overwritten.
- Overwrite is offered only after an explicit decision. Downloads are written to a temporary file and published locally; SFTP v3 does not provide portable atomic replacement, so uploads temporarily rename the old destination to a unique sibling backup before publishing the new file. If publication fails, the app attempts to restore the original. A failed restoration or cleanup can leave a `.superftp-*.backup` file; an interrupted upload can leave a `.superftp-*.part` file. Do not remove these without checking their contents.
- Closing a session cancels its transfers. A process crash or lost connection can leave a remote temporary file; cleanup is best effort. The app does not automatically delete files it did not create.
- Host certificate/CA trust, OpenSSH `known_hosts` import and replacing known host keys from the UI are not implemented. Legitimate key rotation requires independent verification and manual maintenance while the app is closed.
- Listings are virtualized in the interface, but the current SFTP library materializes the complete remote directory before displaying it.
- Native drag and drop, the SSH agent and system credential stores still require real-device validation on all three platforms. Mocked UI tests do not replace this work.

## Structure

```text
src/
  App.svelte                 Sessions and UI coordination
  components/                File panes, dialogs and transfer queue
  lib/                       IPC contracts, localization, formatting and tests
src-tauri/src/
  ssh.rs                     SSH, host-key verification and authentication
  transfer.rs                Streaming, conflicts, concurrency and cancellation
  storage.rs                 Atomic configuration and credential store
  commands.rs                Tauri commands
  paths.rs                   Path and filename validation
  integration_tests.rs       Loopback SSH/SFTP test fixture
e2e/                         UI tests, not shipped with the application
```

Licensed under the [MIT License](LICENSE). Contributions are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md). See [CHANGELOG.md](CHANGELOG.md) for the unreleased changes.

# Contributing to Super FTP

Thank you for helping improve Super FTP! This is a preview; small fixes, documentation, translations, accessibility feedback and carefully scoped features are welcome.

## Before you start

- Search existing issues and pull requests. For substantial changes, open an issue to discuss the approach before investing in an implementation.
- Never include credentials, private keys, real server addresses, personal files or unredacted configuration in issues, screenshots, tests or pull requests.
- Report security vulnerabilities privately; see [SECURITY.md](SECURITY.md). Do not file a public issue with exploit details.
- Please contribute only work you have the right to submit. By submitting a contribution, you agree it may be distributed under the project's [MIT license](LICENSE).

## Set up and run

Install Node.js 24 LTS, Rust stable (at least 1.90), and the platform-specific Tauri prerequisites in [README.md](README.md#requirements). From the project root:

```sh
npm ci
npm run desktop
```

`npm run dev` is a browser-only UI preview, not an SFTP client. For Windows native builds, use a terminal with the Microsoft C++ build environment available. To build a release executable and bundles, run `npm run desktop:build` on the target operating system.

## Before opening a pull request

```sh
npm run verify
npm run test:ui
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

On Linux and macOS, install Playwright Chromium first with `npx playwright install chromium`; on Windows the UI tests use installed Microsoft Edge. Linux also needs the native libraries listed in the README. `npm run test:native` is an **opt-in Windows test** of a previously built release binary; it starts the app and temporarily modifies the current user's application configuration by adding and removing a test profile. Back up your configuration first and do not run it against data you cannot afford to change. The existing native test needs review before being used as a release gate.

Keep changes focused, add tests where practical, and describe what you tested and any platform limitations in the pull request. Include screenshots for visual changes only after checking them for sensitive information. Document changes to transfer, deletion, host-key or credential handling and avoid weakening existing safeguards.

## Translations

English strings are the catalogue keys in `src/lib/i18n/`; each entry provides Catalan, Spanish, French, German, Italian, Portuguese and Dutch translations. Preserve placeholders such as `{name}` exactly. Tests check catalogue coverage and placeholder consistency. Avoid translating paths, filenames and raw library errors as though they were UI messages.

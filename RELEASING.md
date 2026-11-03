# Publishing and release checklist

This is a manual checklist, not an automated publishing workflow. The GitHub repository is [YerayAlonso/super-ftp](https://github.com/YerayAlonso/super-ftp), initially private. No public release, domain or signing identity is assumed to exist. A maintainer should complete these steps before distributing a build.

## Before making the repository public

- [ ] Confirm the maintainers and permission to publish all code, assets and the MIT copyright notice.
- [ ] Review every file to be committed, including tests, screenshots and history, for credentials, private keys, real server details, personal data and third-party material. `.gitignore` is not a secret scanner. Never rely solely on the limited checks performed during preparation.
- [ ] Enable private vulnerability reporting in GitHub Security settings or establish another private reporting channel, and update [SECURITY.md](SECURITY.md) with an actionable contact before announcing the project.
- [ ] Choose moderation and maintenance contacts and add a code of conduct if desired. Enable repository security features and branch protection as appropriate.
- [x] Initialize the local Git repository, configure the remote and push the initial commit.
- [x] Run the desktop CI workflow on the GitHub repository; Windows, macOS, Linux and UI jobs have passed. Continue checking CI for subsequent changes.

## Preparing a release

- [ ] Review the preview limitations, security model, open issues and target operating systems. Test native file operations, transfers, cancellation, conflict handling, SSH agent and credential store on each platform advertised. The Windows native smoke test covers only basic local operations; mocked UI tests do not replace real-device testing.
- [ ] Run `npm run verify`, `npm run test:ui`, Rust formatting, tests and Clippy (see [CONTRIBUTING.md](CONTRIBUTING.md)). Run `npm audit`, `npm audit --omit=dev` and `cargo audit`; investigate and record findings. Review third-party license obligations before distributing binaries.
- [ ] Agree on the version and update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` and the changelog consistently. Verify the lockfiles and build with locked dependencies.
- [ ] Build on each intended target OS. Test installation, launch, upgrade and uninstall in a clean environment. Publish only artifacts that were actually built and tested; do not imply untested platforms are supported.
- [ ] Decide on code signing and macOS notarization. Current builds are unsigned and may trigger OS warnings. Clearly state this if distributing a preview.
- [ ] Generate SHA-256 checksums for the final artifacts, verify them independently, and attach the checksums with the binaries. Checksums provide integrity, not publisher authentication unless obtained through a trusted channel.
- [ ] Create a tag from the reviewed commit and draft a GitHub release. Include changes, tested platforms, known limitations, installation instructions and unsigned-build warnings. Mark early versions as pre-releases if appropriate. Review the draft and assets before publishing.

## Optional GitHub Pages

`docs/index.html` is a static project page that works under a repository subpath. When ready, configure **Settings → Pages → Deploy from a branch** and select the chosen branch and `/docs` folder. Pages availability and visibility for private repositories depend on the GitHub plan and settings; check those before enabling it. Verify the published page and release link before announcing it. No workflow here publishes a website automatically.

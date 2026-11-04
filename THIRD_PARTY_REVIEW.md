# Third-party licensing review — preliminary

This is an engineering inventory, not legal advice or a complete compliance determination. Review the exact dependencies and assets of each release before distribution. The project MIT license does not replace the licenses of third-party components.

## Findings for the current Windows preview

- The application bundles the **Inter** font from `@fontsource-variable/inter` 5.3.0. Its license is **SIL OFL 1.1**; the font package's `LICENSE` includes the Inter copyright notice and the full license. The OFL requires the copyright notice and license to accompany redistributed font software. Verify that every distributed installer and executable includes them in an accessible form.
- The UI uses **Lucide** icons from `@lucide/svelte` 1.67.0. The package declares the **ISC** license and its `LICENSE` also contains the MIT notice for the specified Feather-derived icons. Review and preserve the applicable notices in distributed copies.
- Rust `option-ext` 0.2.0 declares **MPL-2.0**. `cargo tree --target x86_64-pc-windows-msvc -e normal -i option-ext` shows a path through `dirs-sys`, `dirs`, and Tauri to this application. Review the MPL's source availability and notice requirements for the exact binaries and any modifications. This does not automatically mean that the whole application must be relicensed as MPL.
- Other Rust packages in the Windows-targeted metadata report include MIT, Apache-2.0, BSD, ISC, Unicode-3.0, Zlib and MPL-2.0 expressions. Some are build tools or procedural macros rather than code included in the executable. The metadata inventory is not a binary bill of materials and does not establish which packages are shipped.
- The npm lockfile contains MPL-2.0 `lightningcss` packages marked as development dependencies. Development-only status in the lockfile alone does not prove a particular output is free of third-party content; inspect the actual build and packaging.
- `app-icon.svg` and the generated platform icons are project assets. Their provenance and any external inputs still need human verification; this inventory does not independently establish ownership or rights in AI-generated material.

## Method and limits

On the current working tree, review used `package-lock.json`, the installed font and icon package license files, `cargo metadata --locked --filter-platform x86_64-pc-windows-msvc`, and selected `cargo tree -e normal` reverse-dependency paths. Cargo metadata reported 364 packages, including build and development dependencies; npm lockfile entries also include optional packages for other platforms. No full license-text collection, source-offer review, executable attribution check or automated license-policy audit has yet been completed. macOS and Linux dependency graphs were not audited.

`third-party/rust-windows-inventory.json` is a preliminary Windows-targeted `cargo-about` 0.9.3 inventory generated with `--locked --fail`; it reports 342 third-party packages in that tool's resolved graph. Its scope and package count differ from `cargo metadata`. It is not a list of code proven to be in the binary, and the collected license texts still need review. `third-party/inter-OFL-1.1.txt` and `third-party/lucide-ISC-and-Feather-MIT.txt` reproduce the license files from the installed npm packages.

The current development version embeds `src/lib/third-party-notices.txt` and exposes it in the Help dialog; it combines Windows-targeted cargo-about output with license and notice files from installed npm packages. This snapshot is not proof of complete attribution: five installed npm packages lack top-level notice files, `NOTICE` and source availability obligations require separate review, and other OS dependency graphs differ. The published **0.1.0 prerelease predates this change** and does not contain this notice view. Do not treat the snapshot as complete distribution compliance.

The notice generator lists version-specific crates.io source archive URLs for MPL-2.0 entries in the selected graph. The five downloaded archives matched their `Cargo.lock` SHA-256 checksums during this review; run `node scripts/verify-mpl-sources.mjs` to repeat the network check. This does not guarantee ongoing availability, establish that local source files were unmodified, or satisfy every source-distribution obligation. The five npm packages without top-level license files are `@rolldown/binding-win32-x64-msvc`, `@tauri-apps/cli`, `@tauri-apps/cli-win32-x64-msvc`, `is-reference`, and `locate-character`. `npm ls` associates them with the build tools Vite/Rolldown, Tauri CLI and the Svelte compiler. A local Vite build's module list did not include `is-reference`, `locate-character`, `@tauri-apps/cli` or the Rolldown native binding; this is useful evidence, not a complete binary or distribution audit. The Tauri native CLI binding is a build-tool dependency as well. Keep the five entries flagged until their license notices and actual release inclusion have been independently reviewed.

For a local copy of all five verified source archives and checksums, run `node scripts/verify-mpl-sources.mjs --output-dir NEW_DIRECTORY`. The directory must not already exist. `python scripts/verify-mpl-cache.py NEW_DIRECTORY` compares those verified archives with the current Cargo registry files, rejecting altered or extra files; this comparison passed locally for all five packages. It does not establish the provenance of an already-built executable or prevent subsequent cache modifications. These archives are upstream crates.io artifacts, not a complete source distribution. Package and publish any source bundle alongside a future release only after review. The existing 0.1.0 prerelease does not have such a bundle attached.

To regenerate the Windows snapshot, install cargo-about 0.9.3 with its `cli` feature, run `npm ci`, then run:

```sh
node scripts/generate-notices.mjs
```

The script invokes cargo-about directly, so its inventory cannot silently come from an older intermediate JSON file. Raw cargo-about JSON can contain local filesystem paths; do not commit it. The generated notice text excludes metadata paths but still requires human review for completeness. A unit test checks lockfile hashes to flag dependency changes; matching hashes do not prove the notices are complete or that installed npm packages match the lockfile. Generate platform-specific inventories and notices before releasing for other targets.

`npm audit --omit=dev` found no known vulnerabilities at the time of this review. `cargo audit` was not installed and RustSec was not checked; license and vulnerability reviews are separate tasks.

## Before wider distribution

1. Generate a complete, versioned dependency and license inventory for each released platform and distinguish build-only components from distributed components.
2. Collect the applicable full license texts, copyright notices and any required attribution, including Inter and Lucide, and make them accessible with each distributed package. Verify the resulting installer and any standalone executable, not only the source repository.
3. Review MPL-2.0 dependencies, changes to their covered files, and source availability and notice obligations. Check all other license expressions and any `NOTICE` files for additional requirements.
4. Verify the provenance and rights of project-owned assets and AI-assisted content, including applicable provider terms; obtain legal advice if needed.
5. Repeat the inventory and security audits whenever dependency versions or distribution targets change. Update existing release assets if review finds missing required notices.

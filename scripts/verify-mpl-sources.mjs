// Verify registry archives against Cargo.lock; this is not a license audit.
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';

const lock = readFileSync('src-tauri/Cargo.lock', 'utf8');
const blocks = lock.split('[[package]]');
const { packages, cargoLockSha256 } = JSON.parse(readFileSync('third-party/rust-windows-inventory.json', 'utf8'));
const actualLockHash = createHash('sha256').update(lock.replace(/\r\n/g, '\n')).digest('hex');
if (actualLockHash !== cargoLockSha256) throw new Error('Windows inventory is stale; regenerate the notices first.');
const mpl = packages.filter((pkg) => pkg.license?.includes('MPL-2.0'));
if (!mpl.length) throw new Error('No MPL packages found in the Windows inventory.');
const args = process.argv.slice(2);
if (args.length && (args.length !== 2 || args[0] !== '--output-dir')) throw new Error('Usage: node scripts/verify-mpl-sources.mjs [--output-dir NEW_DIRECTORY]');
const output = args[1];
// Refuse an existing directory rather than overwrite any user files.
if (output) mkdirSync(output);
const results = [];
for (const pkg of mpl) {
  const block = blocks.find((item) => {
    const name = item.match(/^name = "([^"]+)"$/m)?.[1];
    const version = item.match(/^version = "([^"]+)"$/m)?.[1];
    return name === pkg.name && version === pkg.version;
  });
  const checksum = block?.match(/^checksum = "([a-f0-9]{64})"$/m)?.[1];
  if (!checksum) throw new Error(`Missing registry checksum: ${pkg.name} ${pkg.version}`);
  const url = `https://crates.io/api/v1/crates/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}/download`;
  const response = await fetch(url, { signal: AbortSignal.timeout(30_000) });
  if (!response.ok) throw new Error(`Could not download ${pkg.name} ${pkg.version}: ${response.status}`);
  const archive = Buffer.from(await response.arrayBuffer());
  const actual = createHash('sha256').update(archive).digest('hex');
  if (actual !== checksum) throw new Error(`Checksum mismatch: ${pkg.name} ${pkg.version}`);
  const filename = `${pkg.name}-${pkg.version}.crate`;
  if (output) writeFileSync(join(output, filename), archive, { flag: 'wx' });
  results.push(`${checksum}  ${filename}`);
  console.log(`${pkg.name} ${pkg.version}: matches Cargo.lock`);
}
if (output) {
  writeFileSync(join(output, 'SHA256SUMS.txt'), `${results.join('\n')}\n`, { flag: 'wx' });
  writeFileSync(join(output, 'README.txt'), 'Unmodified crates.io source archives for MPL-2.0 packages in the Windows-targeted cargo-about dependency graph. Each archive was checked against Cargo.lock. This includes build-only packages and is not a complete source distribution or legal compliance determination.\n', { flag: 'wx' });
  console.log(`Verified source archives written to ${output}`);
}

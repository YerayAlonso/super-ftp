// Generate a reviewable Windows notice snapshot from cargo-about JSON and
// installed npm packages. This is not a substitute for a distribution audit.
import { readFileSync, readdirSync, existsSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';

const command = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const version = spawnSync(command, ['about', '--version'], { encoding: 'utf8' });
if (version.error) throw version.error;
if (version.status !== 0 || version.stdout.trim() !== 'cargo-about 0.9.3') {
  throw new Error('Install cargo-about 0.9.3 with the cli feature before generating notices.');
}
const directory = mkdtempSync(join(tmpdir(), 'superftp-notices-'));
let rust;
try {
  const json = join(directory, 'rust-licenses.json');
  const result = spawnSync(command, [
    'about', 'generate', '--manifest-path', 'src-tauri/Cargo.toml',
    '--target', 'x86_64-pc-windows-msvc', '--locked', '--fail', '--format', 'json', '--output-file', json,
  ], { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`cargo-about failed:\n${result.stderr}`);
  rust = JSON.parse(readFileSync(json, 'utf8'));
} finally {
  rmSync(directory, { recursive: true, force: true });
}
const lock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
const sha256 = (file) => createHash('sha256').update(readFileSync(file, 'utf8').replace(/\r\n/g, '\n')).digest('hex');
const rustPackages = rust.crates.map(({ package: pkg }) => pkg).filter((pkg) => pkg.source)
  .map((pkg) => ({ name: pkg.name, version: pkg.version, license: pkg.license, repository: pkg.repository }))
  .sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
const output = [
  'THIRD-PARTY LICENSE NOTICES — WINDOWS SNAPSHOT',
  `Cargo.lock SHA-256: ${sha256('src-tauri/Cargo.lock')}`,
  `package-lock.json SHA-256: ${sha256('package-lock.json')}`,
  'Generated from the Windows-targeted Cargo dependency graph and installed npm packages.',
  'Includes build and development dependencies; inclusion does not imply code is shipped.',
  'Review notices, source availability and exact distributed dependencies before release.',
];
for (const license of rust.licenses) {
  const packages = license.used_by.map(({ crate }) => `${crate.name} ${crate.version}`).sort();
  const sources = license.id === 'MPL-2.0'
    ? license.used_by.map(({ crate }) => {
      if (!crate.source?.startsWith('registry+https://github.com/rust-lang/crates.io-index')) {
        throw new Error(`Review source availability for ${crate.name} ${crate.version}`);
      }
      return `${crate.name} ${crate.version}: https://crates.io/api/v1/crates/${crate.name}/${crate.version}/download`;
    }).sort().join('\n')
    : '';
  output.push(`\n${'='.repeat(72)}\nRust: ${license.name} (${license.id})\nPackages: ${packages.join(', ')}${sources ? `\nPublished source archives:\n${sources}` : ''}\n\n${license.text}`);
}
const missing = [];
for (const [directory, pkg] of Object.entries(lock.packages)) {
  if (!directory || !existsSync(directory)) continue;
  const notices = readdirSync(directory).filter((name) => /^(licen[cs]e|copying|notice)(\.|$)/i.test(name));
  const label = `${directory.replace(/^node_modules\//, '')} ${pkg.version} (${pkg.license ?? 'unknown license'})`;
  if (!notices.length) { missing.push(label); continue; }
  for (const name of notices) {
    const file = join(directory, name);
    try {
      output.push(`\n${'='.repeat(72)}\nnpm: ${label} — ${name}\n\n${readFileSync(file, 'utf8')}`);
    } catch (error) {
      throw new Error(`Could not read notice for ${label}: ${error.message}`);
    }
  }
}
output.push(`\n${'='.repeat(72)}\nInstalled npm packages without a top-level notice file (review separately):\n${missing.join('\n')}\n`);
writeFileSync('src/lib/third-party-notices.txt', output.join('\n'), 'utf8');
writeFileSync('third-party/rust-windows-inventory.json', `${JSON.stringify({
  target: 'x86_64-pc-windows-msvc', tool: 'cargo-about 0.9.3',
  cargoLockSha256: sha256('src-tauri/Cargo.lock'),
  scope: 'resolved dependency graph; includes build and development dependencies, not a binary bill of materials',
  packages: rustPackages,
}, null, 2)}\n`, 'utf8');
console.log(`Generated notices for ${rust.crates.length} Cargo packages; ${missing.length} npm packages need separate notice review.`);

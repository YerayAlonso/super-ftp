import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { rustNoticeFiles } from './rust-notice-files.mjs';

const normalize = (text) => text.replace(/\r\n/g, '\n');
const inventory = JSON.parse(readFileSync('third-party/rust-windows-inventory.json', 'utf8'));
const lock = normalize(readFileSync('src-tauri/Cargo.lock', 'utf8'));
if (createHash('sha256').update(lock).digest('hex') !== inventory.cargoLockSha256) {
  throw new Error('Windows inventory is stale; regenerate notices.');
}
const result = spawnSync(
  process.platform === 'win32' ? 'cargo.exe' : 'cargo',
  [
    'metadata',
    '--manifest-path',
    'src-tauri/Cargo.toml',
    '--locked',
    '--format-version',
    '1',
    '--filter-platform',
    'x86_64-pc-windows-msvc',
  ],
  { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 },
);
if (result.error) throw result.error;
if (result.status !== 0) throw new Error(result.stderr);
const metadata = JSON.parse(result.stdout);
const notices = normalize(readFileSync('src/lib/third-party-notices.txt', 'utf8'));
let checked = 0;
for (const item of inventory.packages) {
  const pkg = metadata.packages.find(
    (pkg) => pkg.name === item.name && pkg.version === item.version && pkg.source === item.source,
  );
  if (!pkg) throw new Error(`Inventory source mismatch: ${item.name} ${item.version}`);
  if (item.source.startsWith('git+') && !/#[a-f0-9]{40}$/.test(item.source)) {
    throw new Error(`Git dependency lacks an immutable source revision: ${item.name}`);
  }
  if (!notices.includes(`${item.name} ${item.version}`))
    throw new Error(`Missing Rust package: ${item.name} ${item.version}`);
  for (const file of rustNoticeFiles(pkg)) {
    if (!notices.includes(file.text))
      throw new Error(`Missing complete ${file.name}: ${item.name} ${item.version}`);
    checked++;
  }
}
console.log(
  `Checked ${checked} license/NOTICE files and immutable sources for ${inventory.packages.length} Rust packages.`,
);

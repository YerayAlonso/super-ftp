import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

// Public throwaway signature from tauri-plugin-updater's offline tests (MIT OR Apache-2.0).
const signature = 'untrusted comment: signature from tauri secret key\nRUTX5qZvdPSepPF2zsjvP/L/TQtBLRnWRvaq+4omK1UEugtQp/USrGiI1v762sDy3id3aor7bRC9ETyeot6lMHHrWoZ5hjUM1Qc=\ntrusted comment: timestamp:1791649051\tfile:payload\tversion:2.0.0\nDDq2CLOEl5cQmC2eJRHlnZPVViOZsu1CjpPKQoei4A/XBfkDqGO0B+QMrcrHgEQ5qV3mne75tdFWhJpqUXh9AA==';

test('manifest generation checks version and refuses to overwrite', (t) => {
  const preferred = process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, 'Temp', 'opencode');
  const dir = mkdtempSync(join(preferred && existsSync(preferred) ? preferred : tmpdir(), 'superftp-manifest-test-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const installer = join(dir, 'Super.FTP_2.0.0_x64-setup.exe');
  const sig = join(dir, 'installer.sig');
  const output = join(dir, 'manifest.json');
  writeFileSync(installer, 'test');
  writeFileSync(sig, Buffer.from(signature).toString('base64'));
  const run = (...args) => spawnSync(process.execPath, ['scripts/create-update-manifest.mjs', ...args], { encoding: 'utf8' });
  assert.equal(run('2.0.0', installer, sig, output).status, 0);
  const manifest = JSON.parse(readFileSync(output, 'utf8'));
  assert.equal(manifest.platforms['windows-x86_64'].url, 'https://github.com/YerayAlonso/super-ftp/releases/download/v2.0.0/Super.FTP_2.0.0_x64-setup.exe');
  assert.equal(run('2.0.0', installer, sig, output).status, 1);
  const wrong = join(dir, 'Super.FTP_3.0.0_x64-setup.exe');
  writeFileSync(wrong, 'test');
  assert.equal(run('3.0.0', wrong, sig, join(dir, 'wrong.json')).status, 1);
});

// Generate a manifest for a signed Windows release; this does not verify its signature.
import { readFileSync, writeFileSync } from 'node:fs';
import { basename } from 'node:path';

const [version, installer, signatureFile, output] = process.argv.slice(2);
if (process.argv.length !== 6 || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version ?? '')) {
  throw new Error('Usage: node scripts/create-update-manifest.mjs VERSION INSTALLER SIGNATURE OUTPUT');
}
const name = basename(installer);
if (!/^Super\.FTP_[0-9A-Za-z.-]+_x64-setup\.exe$/.test(name)) {
  throw new Error('Unexpected installer filename.');
}
if (name !== `Super.FTP_${version}_x64-setup.exe`) throw new Error('Installer filename does not match version.');
// Ensure the installer exists; the publisher must also verify its signature independently.
readFileSync(installer);
const signature = readFileSync(signatureFile, 'utf8').trim();
if (!/^[A-Za-z0-9+/]+={0,2}$/.test(signature)) throw new Error('Invalid base64 signature format.');
const decoded = Buffer.from(signature, 'base64').toString('utf8');
const trusted = decoded.split(/\r?\n/).find((line) => line.startsWith('trusted comment: '));
if (!trusted?.split('\t').includes(`version:${version}`)) {
  throw new Error('Signature trusted comment does not contain the exact version.');
}
const url = `https://github.com/YerayAlonso/super-ftp/releases/download/v${version}/${name}`;
const manifest = { version, platforms: { 'windows-x86_64': { url, signature } } };
writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`, { flag: 'wx' });
console.log(`Created ${output}. Verify signature and download before publishing; this script does not authenticate them.`);

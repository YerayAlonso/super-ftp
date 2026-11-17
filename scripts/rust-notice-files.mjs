import { readFileSync, readdirSync, lstatSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';

// Keep crate-specific copyright and NOTICE text, in addition to SPDX license
// grouping. Paths stay local; only crate-relative filenames enter the snapshot.
export function rustNoticeFiles(pkg) {
  const root = dirname(pkg.manifest_path);
  const files = new Set();
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (entry.isFile() && /^(?:licen[cs]e|copying|notice)(?:[._-]|$)/i.test(entry.name)) {
      files.add(resolve(root, entry.name));
    }
  }
  if (pkg.license_file) files.add(resolve(root, pkg.license_file));
  function notices(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.name === '.git' || entry.name === 'target' || entry.name === 'node_modules')
        continue;
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) notices(path);
      else if (entry.isFile() && /^notice(?:[._-]|$)/i.test(entry.name)) files.add(path);
    }
  }
  notices(root);
  return [...files].sort().map((path) => {
    if (!lstatSync(path).isFile())
      throw new Error(`Invalid license file for ${pkg.name} ${pkg.version}`);
    const text = readFileSync(path, 'utf8').replace(/\r\n/g, '\n');
    if (!text.trim()) throw new Error(`Empty license file for ${pkg.name} ${pkg.version}`);
    return { name: relative(root, path).replaceAll('\\', '/'), text };
  });
}

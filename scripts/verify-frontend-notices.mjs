// Check Vite's actual JS module graph against the generated notice snapshot.
// This does not cover every Rust binary or separately emitted asset.
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { build } from 'vite';

const notices = readFileSync('src/lib/third-party-notices.txt', 'utf8').replace(/\r\n/g, '\n');
const lock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
const result = await build({ configFile: 'vite.config.ts', logLevel: 'silent', build: { write: false } });
const outputs = Array.isArray(result) ? result : [result];
const modules = outputs.flatMap((entry) => entry.output ?? []).flatMap((entry) => Object.keys(entry.modules ?? {}));
const bundled = new Set();
for (const id of modules) {
  const path = id.replaceAll('\\', '/').split('?')[0];
  const index = path.lastIndexOf('/node_modules/');
  if (index < 0) continue;
  const segments = path.slice(index + '/node_modules/'.length).split('/');
  bundled.add(segments[0].startsWith('@') ? `${segments[0]}/${segments[1]}` : segments[0]);
}
// The font is emitted as a CSS asset rather than a JavaScript module.
bundled.add('@fontsource-variable/inter');
for (const name of [...bundled].sort()) {
  const pkg = lock.packages[`node_modules/${name}`];
  if (!pkg) throw new Error(`No top-level lockfile entry for bundled package ${name}; review nested package mapping.`);
  const label = `npm: ${name} ${pkg.version} (${pkg.license ?? 'unknown license'})`;
  if (!notices.includes(label)) throw new Error(`No generated notice for bundled package ${name}.`);
  const directory = `node_modules/${name}`;
  const files = readdirSync(directory).filter((file) => /^(?:(?:third[-_ ]party[-_ ])?(?:licen[cs]e|copying|notice))([._-]|$)/i.test(file));
  if (!files.length) throw new Error(`No license file found for bundled package ${name}.`);
  for (const file of files) {
    const text = readFileSync(join(directory, file), 'utf8').replace(/\r\n/g, '\n');
    if (!notices.includes(`${label} — ${file}\n\n${text}`)) throw new Error(`The complete ${file} text is missing for ${name}.`);
  }
  console.log(`${name} ${pkg.version}: notice present`);
}
console.log(`Checked ${bundled.size} frontend packages. Review source, attribution, and assets separately.`);

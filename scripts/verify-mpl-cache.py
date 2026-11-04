"""Compare downloaded MPL source archives with Cargo's local registry copies.

Run after verify-mpl-sources.mjs --output-dir NEW_DIRECTORY. This verifies
files in the archives, not the provenance of an already-built executable.
"""
import hashlib
import json
import pathlib
import subprocess
import sys
import tarfile
import tomllib

if len(sys.argv) != 2:
    raise SystemExit('Usage: python scripts/verify-mpl-cache.py SOURCE_DIRECTORY')
source_dir = pathlib.Path(sys.argv[1])
with open('third-party/rust-windows-inventory.json', encoding='utf-8') as handle:
    inventory = json.load(handle)
lock_bytes = pathlib.Path('src-tauri/Cargo.lock').read_bytes().replace(b'\r\n', b'\n')
if hashlib.sha256(lock_bytes).hexdigest() != inventory['cargoLockSha256']:
    raise SystemExit('Windows inventory is stale; regenerate notices first.')
locked = tomllib.loads(lock_bytes.decode('utf-8'))['package']
metadata = json.loads(subprocess.check_output([
    'cargo', 'metadata', '--manifest-path', 'src-tauri/Cargo.toml', '--locked',
    '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc',
], text=True, encoding='utf-8'))
packages = {(pkg['name'], pkg['version']): pkg for pkg in metadata['packages']}
mpl = [pkg for pkg in inventory['packages'] if 'MPL-2.0' in (pkg['license'] or '')]
if not mpl:
    raise SystemExit('No MPL packages found in the Windows inventory.')

for pkg in mpl:
    name, version = pkg['name'], pkg['version']
    lock_entry = next((p for p in locked if p['name'] == name and p['version'] == version), None)
    checksum = lock_entry.get('checksum') if lock_entry else None
    archive_path = source_dir / f'{name}-{version}.crate'
    if not checksum or hashlib.sha256(archive_path.read_bytes()).hexdigest() != checksum:
        raise SystemExit(f'Archive checksum mismatch: {name} {version}')
    metadata_entry = packages.get((name, version))
    if not metadata_entry or not (metadata_entry.get('source') or '').startswith('registry+'):
        raise SystemExit(f'Not a registry package: {name} {version}')
    cache_dir = pathlib.Path(metadata_entry['manifest_path']).parent
    archive_files = set()
    with tarfile.open(archive_path, 'r:gz') as archive:
        for member in archive:
            if not member.isfile():
                if member.isdir():
                    continue
                raise SystemExit(f'Unexpected archive entry: {member.name}')
            parts = pathlib.PurePosixPath(member.name).parts
            if not parts or parts[0] != f'{name}-{version}' or any(part in ('.', '..') for part in parts):
                raise SystemExit(f'Unsafe archive entry: {member.name}')
            relative = pathlib.Path(*parts[1:])
            target = cache_dir / relative
            content = archive.extractfile(member)
            if not content or not target.is_file() or target.is_symlink():
                raise SystemExit(f'Missing or unsafe cache file: {name}/{relative}')
            if hashlib.sha256(content.read()).digest() != hashlib.sha256(target.read_bytes()).digest():
                raise SystemExit(f'Cache file differs from archive: {name}/{relative}')
            archive_files.add(relative.as_posix())
    # Unexpected source files could be imported by a build even if the archive
    # files are intact. Cargo's own cache marker is expected to be extra.
    for file in cache_dir.rglob('*'):
        if file.is_symlink():
            raise SystemExit(f'Unexpected cache symlink: {name}/{file.relative_to(cache_dir)}')
        if file.is_file():
            relative = file.relative_to(cache_dir).as_posix()
            if relative not in archive_files and relative not in ('.cargo-ok', '.cargo-checksum.json'):
                raise SystemExit(f'Unexpected cache file: {name}/{relative}')
    print(f'{name} {version}: local registry files match verified archive')

#!/usr/bin/env python3
"""Assemble a pinned, offline Niva build kit. Run after native CI builds."""
import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
TARGETS = {
    'windows-x86_64': 'niva-windows-x86_64.exe',
    'macos-aarch64': 'niva-macos-aarch64',
    'macos-x86_64': 'niva-macos-x86_64',
}
RUNTIME_LIMIT_BYTES = 3_300_000

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime-dir', type=pathlib.Path, required=True)
    parser.add_argument('--packager', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise SystemExit('Output already exists')
    version = json.loads((ROOT / 'packages/devtools/niva.json').read_text())['version']
    # Ensure the copied host tool and all runtime artifacts belong to this kit.
    tool_version = subprocess.check_output([str(args.packager.resolve()), '--version'], text=True).strip()
    if tool_version != f'niva-packager {version}':
        raise SystemExit(f'Packager version mismatch: {tool_version}')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.niva-kit-', dir=args.output.parent) as tmp:
        kit = pathlib.Path(tmp) / 'niva-build-kit'
        (kit / 'runtimes').mkdir(parents=True)
        runtimes = {}
        runtime_sizes = {}
        for target, filename in TARGETS.items():
            source = args.runtime_dir / filename
            size_bytes = source.stat().st_size
            if size_bytes >= RUNTIME_LIMIT_BYTES:
                raise SystemExit(
                    f'{target} runtime is {size_bytes} bytes; limit is strictly below {RUNTIME_LIMIT_BYTES}'
                )
            dest = kit / 'runtimes' / filename
            shutil.copyfile(source, dest)
            dest.chmod(0o755)
            runtimes[target] = {'path': f'runtimes/{filename}', 'version': version,
                                'sha256': hashlib.sha256(dest.read_bytes()).hexdigest()}
            runtime_sizes[target] = {
                'path': f'runtimes/{filename}',
                'sizeBytes': dest.stat().st_size,
                'limitBytesExclusive': RUNTIME_LIMIT_BYTES,
                'sha256': hashlib.sha256(dest.read_bytes()).hexdigest(),
            }
        tool = kit / ('niva-packager.exe' if args.packager.suffix == '.exe' else 'niva-packager')
        shutil.copyfile(args.packager, tool)
        tool.chmod(0o755)
        (kit / 'manifest.json').write_text(json.dumps({'schemaVersion': 1, 'version': version, 'runtimes': runtimes}, indent=2)+'\n')
        (kit / 'runtime-sizes.json').write_text(json.dumps({
            'schemaVersion': 1,
            'method': 'exact precompiled runtime file sizes copied into this kit',
            'runtimes': runtime_sizes,
        }, indent=2)+'\n')
        shutil.copyfile(ROOT / 'LICENSE', kit / 'LICENSE')
        shutil.copyfile(ROOT / 'docs/packager-usage.md', kit / 'README.md')
        # Preserve the dependency notices, and supply exact source download links.
        metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version=1'], cwd=ROOT))
        nodes = {node['id']:node for node in metadata['resolve']['nodes']}
        roots = {p['id'] for p in metadata['packages'] if p['name'] in ('niva', 'niva-packager')}
        if len(roots) != 2:
            raise SystemExit('Cargo metadata must include the niva runtime and niva-packager')
        selected = set()
        def visit(id):
            if id in selected: return
            selected.add(id)
            for child in nodes[id]['dependencies']: visit(child)
        for root in roots:
            visit(root)
        notices = []
        for package in metadata['packages']:
            if package['id'] not in selected: continue
            name, ver = package['name'], package['version']
            source = f'https://crates.io/api/v1/crates/{name}/{ver}/download' if package['source'] else 'https://github.com/bramblex/niva'
            notices.append(f'{name} {ver}\nLicense: {package["license"]}\nCorresponding source: {source}\n')
            pkgdir = pathlib.Path(package['manifest_path']).parent
            for file in pkgdir.iterdir():
                if file.is_file() and file.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
                    directory = kit / 'licenses' / f'{name}-{ver}'
                    directory.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(file, directory / file.name)
        node_notice = ROOT / 'packages/runtime/src/vendor/THIRD_PARTY_NOTICES.txt'
        if not node_notice.is_file():
            raise SystemExit(f'Missing embedded runtime JavaScript notices: {node_notice}')
        node_notice_dest = kit / 'licenses/runtime/THIRD_PARTY_NOTICES.txt'
        node_notice_dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(node_notice, node_notice_dest)
        notices.append('Niva runtime JavaScript dependencies\nSee licenses/runtime/THIRD_PARTY_NOTICES.txt\n')
        (kit / 'THIRD_PARTY.txt').write_text('\n'.join(notices), encoding='utf-8')
        (kit / 'SHA256SUMS').write_text('\n'.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(kit).as_posix()}' for p in sorted(kit.rglob('*')) if p.is_file())+'\n')
        archive = pathlib.Path(tmp) / 'kit.zip'
        with zipfile.ZipFile(archive, 'w', compression=zipfile.ZIP_DEFLATED) as z:
            for file in sorted(kit.rglob('*')):
                if not file.is_file(): continue
                info = zipfile.ZipInfo(file.relative_to(kit.parent).as_posix())
                info.create_system = 3
                executable = file == tool or file.parent.name == 'runtimes'
                info.external_attr = (0o100755 if executable else 0o100644) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                z.writestr(info, file.read_bytes())
        # Exclusive destination creation prevents an accidental overwrite.
        os.link(archive, args.output)
    print(args.output)

if __name__ == '__main__': main()

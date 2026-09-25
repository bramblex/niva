#!/usr/bin/env python3
"""Check an offline build kit by packaging all targets from its extracted files."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
RUNTIME_LIMIT_BYTES = 3_300_000

def sha256_file(path):
    digest=hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):
            digest.update(chunk)
    return digest.hexdigest()

def package_all(tool, kit, fixture, output, targets, layout):
    output.mkdir(parents=True)
    command=[str(tool),'build','--manifest',str(kit/'manifest.json'),'--config',str(fixture/'niva.json'),
             '--resource-dir',str(fixture),'--output-dir',str(output),'--resource-layout',layout]
    for target in targets: command+=['--target',target]
    result=subprocess.run(command,text=True,capture_output=True,check=True)
    report=json.loads(result.stdout.splitlines()[-1])
    assert [r['target'] for r in report['results']]==targets
    assert all(r['resourceLayout']==layout and r['status']=='complete' for r in report['results']),report
    return report

def check_artifact(item, layout):
    path=Path(item['path'])
    assert sha256_file(path)==item['sha256']
    if item['target']=='windows-x86_64' and layout=='embedded':
        assert path.suffix=='.exe'
        return
    assert path.suffix=='.zip'
    with zipfile.ZipFile(path) as archive:
        names=set(archive.namelist())
        if item['target']=='windows-x86_64':
            exe='NivaPackagerSmoke.exe'
            assert exe in names
            info=archive.getinfo(exe)
            assert (info.external_attr>>16)&0o777==0o755
            if layout=='external':
                assert 'resources/niva.json' in names and 'resources/message.txt' in names
                assert 'resources/META-INF/niva/THIRD_PARTY.txt' in names
                assert all(archive.getinfo(name).compress_type==zipfile.ZIP_STORED
                           for name in names if name.startswith('resources/'))
        else:
            app='NivaPackagerSmoke.app'
            exe=f'{app}/Contents/MacOS/NivaPackagerSmoke'
            assert exe in names
            assert (archive.getinfo(exe).external_attr>>16)&0o777==0o755
            if layout=='external':
                marker=f'{app}/Contents/Resources/RESOURCE_MODE'
                prefix=f'{app}/Contents/Resources/app/'
                assert archive.read(marker)==b'external'
                assert f'{prefix}niva.json' in names and f'{prefix}message.txt' in names
                assert f'{prefix}META-INF/niva/THIRD_PARTY.txt' in names
                assert all(archive.getinfo(name).compress_type==zipfile.ZIP_STORED
                           for name in names if name.startswith(prefix))
            if sys.platform=='darwin':
                extracted=path.parent/f'{item["target"]}-{layout}'
                subprocess.run(['ditto','-x','-k',str(path),str(extracted)],check=True)
                subprocess.run(['codesign','--verify','--deep','--strict',str(extracted/app)],check=True)
                import shutil
                shutil.rmtree(extracted)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--kit',type=Path,required=True)
    args=parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='niva-kit-check-') as tmp:
        root=Path(tmp).resolve()
        with zipfile.ZipFile(args.kit) as archive:
            for item in archive.infolist():
                if not (root/item.filename).resolve().is_relative_to(root): raise RuntimeError('Unsafe kit archive path')
            archive.extractall(root)
        kit=root/'niva-build-kit'
        for line in (kit/'SHA256SUMS').read_text().splitlines():
            expected,name=line.split('  ',1)
            if hashlib.sha256((kit/name).read_bytes()).hexdigest()!=expected: raise RuntimeError(f'Kit hash mismatch: {name}')
        third_party=(kit/'THIRD_PARTY.txt').read_text(encoding='utf-8')
        for package in ('wry ', 'apple-codesign '):
            if package not in third_party: raise RuntimeError(f'Missing native dependency notice for {package.strip()}')
        node_notice=kit/'licenses/runtime/THIRD_PARTY_NOTICES.txt'
        if not node_notice.is_file() or not node_notice.stat().st_size:
            raise RuntimeError('Missing embedded runtime JavaScript notices')
        if not (kit/'LICENSE').is_file(): raise RuntimeError('Missing Niva license')
        manifest=json.loads((kit/'manifest.json').read_text(encoding='utf-8'))
        size_report=json.loads((kit/'runtime-sizes.json').read_text(encoding='utf-8'))
        assert size_report['schemaVersion']==1
        assert set(size_report['runtimes'])==set(manifest['runtimes'])
        for target, runtime in manifest['runtimes'].items():
            row=size_report['runtimes'][target]
            runtime_path=kit/runtime['path']
            size=runtime_path.stat().st_size
            digest=hashlib.sha256(runtime_path.read_bytes()).hexdigest()
            assert row['path']==runtime['path'] and row['sizeBytes']==size
            assert row['limitBytesExclusive']==RUNTIME_LIMIT_BYTES and size<RUNTIME_LIMIT_BYTES
            assert row['sha256']==runtime['sha256']==digest
        tool=kit/('niva-packager.exe' if os.name=='nt' else 'niva-packager')
        tool.chmod(0o755)
        fixture=root/'project'
        subprocess.run([sys.executable,str(ROOT/'scripts/packager-smoke.py'),'--fixture',str(fixture)],check=True)
        targets=['windows-x86_64','macos-aarch64','macos-x86_64']
        embedded=package_all(tool,kit,fixture,root/'out-embedded',targets,'embedded')
        external=package_all(tool,kit,fixture,root/'out-external',targets,'external')
        for item in embedded['results']: check_artifact(item,'embedded')
        for item in external['results']: check_artifact(item,'external')
        if sys.platform=='win32':
            embedded_exe=next(Path(item['path']) for item in embedded['results'] if item['target']=='windows-x86_64')
            green_zip=next(Path(item['path']) for item in external['results'] if item['target']=='windows-x86_64')
            subprocess.run([sys.executable,str(ROOT/'scripts/packager-smoke.py'),'--artifact',str(embedded_exe)],check=True)
            subprocess.run([sys.executable,str(ROOT/'scripts/packager-smoke.py'),'--artifact',str(green_zip)],check=True)
        elif sys.platform=='darwin':
            machine=platform.machine().lower()
            native='macos-aarch64' if machine in ('arm64','aarch64') else 'macos-x86_64'
            for report in (embedded,external):
                artifact=next(Path(item['path']) for item in report['results'] if item['target']==native)
                subprocess.run([sys.executable,str(ROOT/'scripts/packager-smoke.py'),'--artifact',str(artifact)],check=True)
        print(json.dumps({'kit':str(args.kit),'targets':targets,'layouts':['embedded','external'],'status':'passed'}))

if __name__=='__main__': main()

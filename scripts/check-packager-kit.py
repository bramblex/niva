#!/usr/bin/env python3
"""Check an offline build kit by packaging all targets from its extracted files."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]

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
        tool=kit/('niva-packager.exe' if os.name=='nt' else 'niva-packager')
        tool.chmod(0o755)
        fixture=root/'project'
        subprocess.run([sys.executable,str(ROOT/'scripts/packager-smoke.py'),'--fixture',str(fixture)],check=True)
        targets=['windows-x86_64','macos-aarch64','macos-x86_64']
        command=[str(tool),'build','--manifest',str(kit/'manifest.json'),'--config',str(fixture/'niva.json'),'--resource-dir',str(fixture),'--output-dir',str(root/'out')]
        for target in targets: command+=['--target',target]
        result=subprocess.run(command,text=True,capture_output=True,check=True)
        report=json.loads(result.stdout.splitlines()[-1])
        assert [r['target'] for r in report['results']]==targets
        for item in report['results']:
            assert item['status']=='complete',item
            path=Path(item['path'])
            assert hashlib.sha256(path.read_bytes()).hexdigest()==item['sha256']
            if path.suffix=='.zip':
                with zipfile.ZipFile(path) as archive:
                    executable=archive.getinfo('NivaPackagerSmoke.app/Contents/MacOS/NivaPackagerSmoke')
                    assert (executable.external_attr>>16)&0o777==0o755
                if sys.platform=='darwin':
                    extracted=root/item['target']
                    subprocess.run(['ditto','-x','-k',str(path),str(extracted)],check=True)
                    subprocess.run(['codesign','--verify','--deep','--strict',str(extracted/'NivaPackagerSmoke.app')],check=True)
        print(json.dumps({'kit':str(args.kit),'targets':targets,'status':'passed'}))

if __name__=='__main__': main()

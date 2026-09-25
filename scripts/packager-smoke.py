#!/usr/bin/env python3
"""Build and run a packaged application on the current native host.

Pass --artifact to test a package produced by another host. Runtime success is
reported over Niva stdio, including resource/bridge/window/filesystem checks.
"""
import argparse
import json
import pathlib
import queue
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import uuid
import zipfile
from pathlib import PurePosixPath

PAGE = '''<!doctype html><html><body><h1>Niva packaged application</h1><script>
window.addEventListener('load', () => setTimeout(async () => {
  let step = "os.info";
  try {
    const info = Niva.os.info;
    step = "window.title";
    const title = await Niva.window.title();
    await Niva.window.setTitle(title + ' verified');
    step = "resource.read";
    const text = await Niva.resource.read('message.txt');
    if (text !== 'packager-resource-ok') throw new Error('resource mismatch');
    step = "os.dirs";
    const temp = (await Niva.os.dirs()).temp;
    await Niva.fs.promises.mkdir(temp, {recursive: true});
    const file = temp.replace(/[\\\\/]$/, '') + '/niva-packager-' + Date.now() + '.txt';
    step = "fs.write";
    await Niva.fs.promises.writeFile(file, text, 'utf8');
    if (await Niva.fs.promises.readFile(file, 'utf8') !== text) throw new Error('filesystem mismatch');
    await Niva.fs.promises.unlink(file);
    step = "runtime ESM";
    const path = await import('node:path');
    if (path.basename('/a/b.txt') !== 'b.txt') throw new Error('runtime ESM mismatch');
    Niva.process.stdout.write(JSON.stringify({name:'packager-smoke',ok:true,os:info,title}) + '\\n');
  } catch(error) { Niva.process.stdout.write(JSON.stringify({name:'packager-smoke',ok:false,step,message:String(error)}) + '\\n'); }
}, 100));
</script></body></html>'''

def generate_fixture(root):
    root.mkdir(parents=True, exist_ok=True)
    config = {'name':'NivaPackagerSmoke','uuid':'46342609-d4c3-41e8-a4a5-8d68439a3461','version':'1.2.3',
              'injectEsm':True,'window':{'entry':'index.html','title':'Packager smoke','size':{'width':420,'height':260}}}
    (root/'niva.json').write_text(json.dumps(config),encoding='utf-8')
    (root/'index.html').write_text(PAGE,encoding='utf-8')
    (root/'message.txt').write_text('packager-resource-ok',encoding='utf-8')

def run(binary):
    process = subprocess.Popen([str(binary.resolve())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    messages=queue.Queue()
    def read_stdout():
        for line in process.stdout: messages.put(line)
    threading.Thread(target=read_stdout,daemon=True).start()
    stderr=[]
    def read_stderr():
        for line in process.stderr: stderr.append(line)
    threading.Thread(target=read_stderr,daemon=True).start()
    try:
        deadline=time.monotonic()+45
        while time.monotonic()<deadline:
            try: line=messages.get(timeout=0.5)
            except queue.Empty:
                if process.poll() is not None: break
                continue
            try: message=json.loads(line)
            except json.JSONDecodeError: continue
            if 'packager-smoke' in line:
                print(json.dumps(message,ensure_ascii=False))
                if message.get('name') != 'packager-smoke': continue
                if message.get('ok') is not True: raise RuntimeError('application smoke failed')
                return
        raise RuntimeError('No successful smoke event received. stderr: '+''.join(stderr)[-4000:])
    finally:
        if process.poll() is None:
            process.terminate()
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: process.kill();process.wait()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifact',type=pathlib.Path)
    parser.add_argument('--fixture',type=pathlib.Path)
    args=parser.parse_args()
    if args.fixture: generate_fixture(args.fixture)
    if not args.artifact: return
    with tempfile.TemporaryDirectory(prefix='niva-packager-smoke-') as tmp:
        if args.artifact.suffix=='.zip':
            with zipfile.ZipFile(args.artifact) as archive:
                names=[]
                for item in archive.infolist():
                    name=item.filename
                    relative=PurePosixPath(name)
                    mode=(item.external_attr>>16)&0o170000
                    if (not name or name.startswith('/') or '\\' in name or ':' in name
                            or any(part in ('','..') for part in relative.parts)
                            or mode==0o120000):
                        raise RuntimeError('Unsafe package archive entry: '+name)
                    names.append(name)
                windows_exe=next((name for name in names if len(PurePosixPath(name).parts)==1 and name.lower().endswith('.exe')),None)
                if windows_exe and 'resources/niva.json' in names:
                    if sys.platform!='win32': raise SystemExit('Windows green ZIP runtime testing requires Windows')
                    archive.extractall(tmp)
                    binary=pathlib.Path(tmp)/windows_exe
                    if not (pathlib.Path(tmp)/'resources/message.txt').is_file():
                        raise RuntimeError('External resource directory is incomplete')
                else:
                    if sys.platform!='darwin': raise SystemExit('macOS app ZIP runtime testing requires macOS')
                    subprocess.run(['ditto','-x','-k',str(args.artifact.resolve()),tmp],check=True)
                    app=next(pathlib.Path(tmp).glob('*.app'))
                    subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
                    binary=next((app/'Contents/MacOS').iterdir())
                    if binary.stat().st_mode & 0o111 != 0o111: raise RuntimeError('executable permissions lost')
        else:
            if sys.platform!='win32': raise SystemExit('EXE runtime testing requires Windows')
            binary=args.artifact
        run(binary)

if __name__=='__main__': main()

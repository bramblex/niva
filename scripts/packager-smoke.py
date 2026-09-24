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

PAGE = '''<!doctype html><html><body><h1>Niva packaged application</h1><script>
window.addEventListener('load', () => setTimeout(async () => {
  let step = "os.info";
  try {
    const info = await Niva.api.os.info();
    step = "window.title";
    const title = await Niva.api.window.title();
    await Niva.api.window.setTitle(title + ' verified');
    step = "resource.read";
    const text = await Niva.api.resource.read('message.txt');
    if (text !== 'packager-resource-ok') throw new Error('resource mismatch');
    step = "os.dirs";
    const temp = (await Niva.api.os.dirs()).temp;
    await Niva.api.fs.createDirAll(temp);
    const file = temp.replace(/[\\\\/]$/, '') + '/niva-packager-' + Date.now() + '.txt';
    step = "fs.write";
    await Niva.api.fs.write(file, text);
    if (await Niva.api.fs.read(file) !== text) throw new Error('filesystem mismatch');
    await Niva.api.fs.remove(file);
    step = "NodeCompat";
    const path = await Niva.import('node:path');
    if (path.basename('/a/b.txt') !== 'b.txt') throw new Error('NodeCompat mismatch');
    await Niva.api.host.send('packager-smoke', {ok:true, os:info, title});
  } catch(error) { await Niva.api.host.send('packager-smoke', {ok:false,step,error:JSON.stringify(error),message:String(error)}); }
}, 100));
</script></body></html>'''

def generate_fixture(root):
    root.mkdir(parents=True, exist_ok=True)
    config = {'name':'NivaPackagerSmoke','uuid':'46342609-d4c3-41e8-a4a5-8d68439a3461','version':'1.2.3',
              'nodeCompat':{'modules':['path']},'window':{'entry':'index.html','title':'Packager smoke','size':{'width':420,'height':260}}}
    (root/'niva.json').write_text(json.dumps(config),encoding='utf-8')
    (root/'index.html').write_text(PAGE,encoding='utf-8')
    (root/'message.txt').write_text('packager-resource-ok',encoding='utf-8')

def run(binary):
    process = subprocess.Popen([str(binary.resolve()),'--stdio'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
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
                # The stdio event wraps the payload under params.data.
                def find_ok(value):
                    if isinstance(value,dict):
                        if 'ok' in value: return value['ok'] is True
                        return any(find_ok(v) for v in value.values())
                    return False
                if not find_ok(message): raise RuntimeError('application smoke failed')
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
            if sys.platform!='darwin': raise SystemExit('Mac ZIP runtime testing requires macOS')
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

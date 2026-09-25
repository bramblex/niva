"""Build and run both macOS resource layouts using a local, single-target test kit."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import threading
import time
import uuid

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
SIZE = 33 * 1024 * 1024 + 19


def kit_for_host(binary, packager, root):
    """This is an isolated host fixture, not a distributable three-platform kit."""
    target = 'macos-aarch64' if platform.machine() == 'arm64' else 'macos-x86_64'
    version = subprocess.check_output([str(packager), '--version'], text=True).strip().split()[-1]
    if binary.stat().st_size >= 3300000:
        raise ValueError('Use a complete release runtime below the size gate')
    root.mkdir(parents=True)
    runtime = root / 'runtimes' / target
    runtime.parent.mkdir()
    shutil.copy2(binary, runtime)
    (root / 'manifest.json').write_text(json.dumps({'schemaVersion': 1, 'version': version,
        'runtimes': {target: {'path': f'runtimes/{target}', 'version': version,
                             'sha256': hashlib.sha256(binary.read_bytes()).hexdigest()}}}))
    shutil.copy2(REPO / 'LICENSE', root / 'LICENSE')
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version=1'], cwd=REPO))
    graph = {node['id']: node for node in metadata['resolve']['nodes']}
    selected = set()
    def visit(identity):
        if identity in selected: return
        selected.add(identity)
        for dependency in graph[identity]['dependencies']: visit(dependency)
    for package in metadata['packages']:
        if package['name'] in ('niva', 'niva-packager'): visit(package['id'])
    notices = []
    for package in metadata['packages']:
        if package['id'] not in selected: continue
        notices.append(f"{package['name']} {package['version']} — {package['license']}")
        for source in Path(package['manifest_path']).parent.iterdir():
            if source.is_file() and source.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
                destination = root / 'licenses' / f"{package['name']}-{package['version']}" / source.name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)
    (root / 'licenses' / 'runtime').mkdir(parents=True)
    shutil.copy2(REPO / 'packages/runtime/src/vendor/THIRD_PARTY_NOTICES.txt', root / 'licenses/runtime/THIRD_PARTY_NOTICES.txt')
    (root / 'THIRD_PARTY.txt').write_text('\n'.join(notices) + '\nSee licenses/runtime for embedded JavaScript notices.\n')
    return target


def run_layout(binary, packager, output, kit, target, layout):
    root = output / layout
    root.mkdir()
    resources = root / 'resources'
    resources.mkdir()
    result = {'ok': False, 'layout': layout}
    done = threading.Event()
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def do_POST(self):
            length = int(self.headers.get('Content-Length', '0'))
            if length > 65536: self.send_response(413); self.end_headers(); return
            result.update(json.loads(self.rfile.read(length)))
            self.send_response(200)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            done.set()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    endpoint = f'http://127.0.0.1:{server.server_port}/'
    (resources / 'probe.js').write_text('const REPORT_URL=' + json.dumps(endpoint) + ';\n' + (HERE / 'probe.js').read_text())
    (resources / 'fixture.json').write_text(json.dumps({'size': SIZE}))
    (resources / 'module.cjs').write_text('exports.text=require("fs").readFileSync(require("path").join(__dirname,"message.txt"),"utf8");exports.fsIdentity=require("fs")===Niva.fs;')
    (resources / 'message.txt').write_text('packaged-module-data')
    (resources / 'index.html').write_text('<!doctype html><head><meta charset="utf-8">'
        f'<meta http-equiv="Content-Security-Policy" content="default-src \'self\'; script-src \'self\'; connect-src \'self\' {endpoint}">'
        '<title>Packaged resource layout</title></head><body><pre id="status">Testing</pre><script src="probe.js"></script></body>')
    pattern = bytes(range(251)) * 4096
    with (resources / 'large.bin').open('wb') as large:
        remaining = SIZE
        while remaining:
            chunk = pattern[:min(remaining, len(pattern))]
            large.write(chunk)
            remaining -= len(chunk)
    config = root / 'niva.json'
    config.write_text(json.dumps({'name': 'NivaResourceLayoutSmoke', 'version': '1.0.0', 'uuid': str(uuid.uuid4()),
        'injectCommonJs': True, 'injectEsm': True, 'window': {'entry': 'index.html', 'visible': True}}))
    command = [str(packager), 'build', '--manifest', str(kit / 'manifest.json'), '--config', str(config),
        '--resource-dir', str(resources), '--output-dir', str(root / 'packages'), '--resource-layout', layout, '--target', target]
    process = None
    try:
        built = subprocess.run(command, capture_output=True, text=True, timeout=120)
        (root / 'packager.stdout.log').write_text(built.stdout)
        (root / 'packager.stderr.log').write_text(built.stderr)
        if built.returncode: raise RuntimeError(f'Packager failed: {built.stderr or built.stdout}')
        report = json.loads(built.stdout.strip().splitlines()[-1])
        artifact = Path(report['results'][0]['path'])
        result['archiveBytes'] = artifact.stat().st_size
        result['archiveSha256'] = hashlib.sha256(artifact.read_bytes()).hexdigest()
        unpacked = root / 'unpacked'
        subprocess.run(['ditto', '-x', '-k', str(artifact), str(unpacked)], check=True)
        app = next(unpacked.glob('*.app'))
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
        executable = next((app / 'Contents/MacOS').iterdir())
        started = time.monotonic()
        with (root / 'stdio.log').open('wb') as log:
            process = subprocess.Popen([str(executable)], stdout=log, stderr=subprocess.STDOUT)
            peak = 0
            while not done.wait(.1) and time.monotonic() - started < 45:
                if process.poll() is not None: break
                rss = subprocess.run(['ps', '-o', 'rss=', '-p', str(process.pid)], capture_output=True, text=True)
                if rss.stdout.strip(): peak = max(peak, int(rss.stdout.strip()) * 1024)
            result['nativePeakRssSampledBytes'] = peak
            result['launchToReportMs'] = round((time.monotonic() - started) * 1000)
            if not done.is_set(): raise RuntimeError('No browser result within deadline')
    except Exception as error:
        result['ok'] = False
        result['error'] = str(error)
    finally:
        if process and process.poll() is None:
            process.terminate()
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: process.kill(); process.wait()
        server.shutdown(); server.server_close()
    (root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--packager', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    if sys.platform != 'darwin': raise SystemExit('This driver currently covers macOS app packaging')
    binary, packager, output = map(lambda value: Path(value).resolve(), (args.binary, args.packager, args.output))
    output.mkdir(parents=True, exist_ok=True)
    kit = output / 'single-host-test-kit'
    target = kit_for_host(binary, packager, kit)
    cases = []
    for layout in ['embedded', 'external']:
        result = run_layout(binary, packager, output, kit, target, layout)
        cases.append(result)
        print(json.dumps(result), flush=True)
    report = {'ok': all(case['ok'] for case in cases), 'cases': cases, 'target': target,
        'nativeBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'runtimeBytes': binary.stat().st_size, 'resourceBytes': SIZE,
        'limitations': ['Single host test kit; not a release kit', 'RSS samples Native process only, not WebContent', 'Patterned binary data, not video playback']}
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__': main()

"""Real WebView four-mode initialization, interface identity, and strict-CSP CJS."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import subprocess
import threading
import uuid

HERE = Path(__file__).resolve().parent


def run_case(binary, root, commonjs, esm, override=False):
    root.mkdir(parents=True, exist_ok=True)
    result = {'ok': False, 'commonjs': commonjs, 'esm': esm, 'authorOverride': override}
    done = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def do_OPTIONS(self):
            self.send_response(204)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type')
            self.end_headers()
        def do_POST(self):
            length = int(self.headers.get('Content-Length', '0'))
            if length > 65536:
                self.send_response(413); self.end_headers(); return
            result.update(json.loads(self.rfile.read(length)))
            self.send_response(200)
            self.send_header('Access-Control-Allow-Origin', '*')
            self.end_headers()
            done.set()

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    endpoint = f'http://127.0.0.1:{server.server_port}/'
    threading.Thread(target=server.serve_forever, daemon=True).start()
    (root / 'fixture.json').write_text(json.dumps({'commonjs': commonjs, 'esm': esm, 'override': override}))
    (root / 'probe.js').write_text('const REPORT_URL = ' + json.dumps(endpoint) + ';\n' + (HERE / 'probe.js').read_text())
    (root / 'module.cjs').write_text('exports.value=42; exports.thisIsExports=this===exports; exports.moduleInstance=module instanceof require("module");')
    (root / 'author-path.mjs').write_text('export default {marker: "author-override"};')
    author_map = '<script type="importmap">{"imports":{"node:path":"./author-path.mjs"}}</script>' if override else ''
    (root / 'index.html').write_text(
        '<!doctype html><html><head><meta charset="utf-8">'
        f'<meta http-equiv="Content-Security-Policy" content="default-src \'self\'; script-src \'self\'; connect-src \'self\' {endpoint}">'
        f'<title>Runtime bootstrap matrix</title>{author_map}</head><body><pre id="status">Testing...</pre>'
        '<script src="probe.js"></script></body></html>')
    (root / 'niva.json').write_text(json.dumps({'name': 'Runtime bootstrap matrix', 'uuid': str(uuid.uuid4()),
        'injectCommonJs': commonjs, 'injectEsm': esm,
        'window': {'entry': 'index.html', 'visible': True, 'size': {'width': 720, 'height': 480}}}))
    with (root / 'stdio.log').open('wb') as log:
        process = subprocess.Popen([str(binary), f'--resource={root}'], stdout=log, stderr=subprocess.STDOUT)
        try:
            if not done.wait(30):
                result['error'] = f'No browser report within 30 seconds; native status {process.poll()}'
        finally:
            if process.poll() is None: process.terminate()
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill(); process.wait(timeout=5)
            server.shutdown(); server.server_close()
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    if not binary.is_file(): raise FileNotFoundError(binary)
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    cases = []
    for commonjs, esm in ((False, False), (True, False), (False, True), (True, True)):
        result = run_case(binary, output / f'cjs-{int(commonjs)}-esm-{int(esm)}', commonjs, esm)
        cases.append(result)
        print(json.dumps(result), flush=True)
    result = run_case(binary, output / 'esm-author-override', True, True, True)
    cases.append(result)
    print(json.dumps(result), flush=True)
    report = {'ok': all(case['ok'] for case in cases), 'cases': cases,
              'nativeBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__': main()

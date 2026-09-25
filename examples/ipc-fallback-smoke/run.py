"""Run a real remote WebView with CSP blocking the local WS/sync-XHR bridge."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import threading
import time
import uuid

HERE = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--https-url', default='https://example.com/')
    parser.add_argument('--trusted-debug', action='store_true', help='Exercise explicit debug-origin token authorization instead of remote grants')
    parser.add_argument('--startup-timeout', type=float, default=90.0)
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    if not binary.is_file():
        raise FileNotFoundError(binary)
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    report = {}
    done = threading.Event()
    fixture = {'file': str(output / 'ipc-text.txt'), 'python': sys.executable,
               'directory': str(output / ('directory-' + uuid.uuid4().hex)),
               'httpsUrl': args.https_url,
               'progress': str(output / 'progress.json'),
               'started': str(output / 'lease-child-started.txt'),
               'orphan': str(output / 'lease-orphan.txt'),
               'command': 'echo niva-ipc' if os.name == 'nt' else 'printf niva-ipc'}
    Path(fixture['started']).unlink(missing_ok=True)
    Path(fixture['orphan']).unlink(missing_ok=True)
    Path(fixture['progress']).unlink(missing_ok=True)

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def respond(self, status, data, content_type):
            self.send_response(status)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(data)))
            self.send_header('Content-Security-Policy', "default-src 'self'; script-src 'self'; connect-src 'self'")
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            if self.path == '/':
                self.respond(200, b'<!doctype html><meta charset="utf-8"><title>IPC fallback smoke</title><pre id="status">Running...</pre><script src="/cases.js"></script>', 'text/html')
            elif self.path == '/cases.js':
                self.respond(200, (HERE / 'cases.js').read_bytes(), 'application/javascript')
            elif self.path == '/fixture':
                self.respond(200, json.dumps(fixture).encode(), 'application/json')
            elif self.path == '/text':
                self.respond(200, 'HTTP 中文'.encode(), 'text/plain; charset=utf-8')
            elif self.path == '/child-started':
                self.respond(200, json.dumps({'started': Path(fixture['started']).exists()}).encode(), 'application/json')
            else:
                self.respond(404, b'missing', 'text/plain')

        def do_POST(self):
            if self.path == '/progress':
                length = int(self.headers.get('Content-Length', '0'))
                data = self.rfile.read(length)
                temporary = output / 'progress.json.tmp'
                temporary.write_bytes(data)
                temporary.replace(output / 'progress.json')
                self.respond(200, b'ok', 'text/plain')
                return
            if self.path not in ('/result', '/echo'):
                self.respond(404, b'missing', 'text/plain')
                return
            length = int(self.headers.get('Content-Length', '0'))
            if length > 1024 * 1024:
                self.respond(413, b'too large', 'text/plain')
                return
            body = self.rfile.read(length)
            if self.path == '/echo':
                data = {'body': body.decode('utf8'), 'header': self.headers.get('X-Niva-Test')}
                self.respond(200, json.dumps(data).encode(), 'application/json')
                return
            report.update(json.loads(body))
            self.respond(200, b'ok', 'text/plain')
            done.set()

    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    origin = f'http://127.0.0.1:{server.server_port}'
    fixture['origin'] = origin
    config = output / 'niva.json'
    config.write_text(json.dumps({
        'name': 'Niva IPC fallback smoke', 'uuid': str(uuid.uuid4()),
        'injectCommonJs': False, 'injectEsm': False,
        'window': {'entry': origin + '/', 'visible': True,
                   'permissions': {} if args.trusted_debug else {origin: ['fs.readText', 'fs.writeText', 'fs.appendText', 'fs.node',
                                           'http.requestText', 'process.execText', 'process.execFileText']}},
    }, indent=2))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    log = (output / 'stdio.log').open('wb')
    process = None
    try:
        command = [str(binary), f'--config={config}', f'--resource={output}']
        if args.trusted_debug:
            command.append(f'--debug-entry={origin}/')
        process = subprocess.Popen(command,
                                   stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.PIPE)
        deadline = time.monotonic() + args.startup_timeout
        while not done.wait(.2):
            if process.poll() is not None:
                raise RuntimeError(f'Niva exited before reporting: {process.returncode}')
            if time.monotonic() > deadline:
                progress_path = Path(fixture['progress'])
                progress = json.loads(progress_path.read_text()) if progress_path.exists() else None
                raise TimeoutError(f'remote IPC page did not report within {args.startup_timeout:g} seconds; progress={progress!r}')
        # Keep the app alive past the child's scheduled side effect: terminating
        # Niva first could hide a failure to kill the expired IPC child.
        if Path(fixture['started']).exists():
            time.sleep(2.5)
            released = not Path(fixture['orphan']).exists()
            report.setdefault('checks', []).append({'name': 'expired IPC child cannot perform its delayed write', 'ok': released})
            report['ok'] = bool(report.get('ok')) and released
    except Exception as error:
        report.update(ok=False, error=str(error))
    finally:
        if process is not None:
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            if process.stdin:
                process.stdin.close()
        log.close()
        server.shutdown()
        server.server_close()
    report.update(engine='real-webview-ipc', authorization='trusted-debug-token' if args.trusted_debug else 'explicit-remote-grants',
                  nativeBinarySha256=hashlib.sha256(binary.read_bytes()).hexdigest())
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report.get('ok') else 1)


if __name__ == '__main__':
    main()

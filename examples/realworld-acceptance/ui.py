"""Serve the original Vite build and drive it in a Niva child WebView."""
import argparse
import hashlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import socket
import threading
from niva import start
from reference import stop

HERE = Path(__file__).resolve().parent


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('compiled')
    parser.add_argument('--frontend', required=True)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--backend-port', type=int, default=3001,
                        help='Must match VITE_BACKEND_PORT used by the original frontend build')
    args = parser.parse_args()
    root, frontend, binary, output = map(lambda value: Path(value).resolve(),
        (args.compiled, args.frontend, args.binary, args.output))
    output.mkdir(parents=True, exist_ok=True)
    if not (frontend / 'index.html').is_file(): raise FileNotFoundError(frontend / 'index.html')
    # Never replace or kill an existing service on the frontend's configured port.
    with socket.socket() as availability:
        availability.bind(('127.0.0.1', args.backend_port))
    done = threading.Event()
    report = {'ok': False, 'checks': [], 'nativeBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    class Handler(SimpleHTTPRequestHandler):
        def __init__(self, *values, **kwargs): super().__init__(*values, directory=str(frontend), **kwargs)
        def log_message(self, *values): pass
        def do_GET(self):
            if self.path == '/__niva_ui_probe.js':
                body = (HERE / 'ui_probe.js').read_bytes()
                self.send_response(200)
                self.send_header('Content-Type', 'application/javascript')
            elif self.path in ('/', '/signin'):
                body = (frontend / 'index.html').read_bytes().replace(b'</body>', b'<script src="/__niva_ui_probe.js"></script></body>')
                self.send_response(200)
                self.send_header('Content-Type', 'text/html; charset=utf-8')
            else:
                return super().do_GET()
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        def do_POST(self):
            if self.path != '/__niva_ui_result': self.send_error(404); return
            length = int(self.headers.get('Content-Length', '0'))
            if length > 65536: self.send_error(413); return
            report.update(json.loads(self.rfile.read(length)))
            self.send_response(200)
            self.end_headers()
            done.set()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    native, log = None, None
    try:
        native, log, _ = start(root, output, binary, 'ui', 60,
            backend_port=args.backend_port, frontend_port=server.server_port,
            ui_url=f'http://localhost:{server.server_port}/')
        if not done.wait(100): raise TimeoutError('Original frontend did not report within 100 seconds')
    except Exception as error:
        report['ok'] = False
        report['error'] = str(error)
    finally:
        if native: stop(native, log)
        server.shutdown(); server.server_close()
    report['frontendIndexSha256'] = hashlib.sha256((frontend / 'index.html').read_bytes()).hexdigest()
    report['limitations'] = ['DOM interaction inside real WebView; not visual/manual desktop acceptance', 'No host Node backend']
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__': main()

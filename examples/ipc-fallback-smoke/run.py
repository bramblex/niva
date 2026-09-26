"""Run a real remote WebView with CSP blocking the local WS/sync-XHR bridge."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys
import threading
import time
import uuid

HERE = Path(__file__).resolve().parent


def pids_for_executable(executable):
    result = subprocess.run(
        ['/bin/ps', '-axo', 'pid=,command='],
        check=True,
        capture_output=True,
        text=True,
    )
    expected = str(executable)
    pids = set()
    for line in result.stdout.splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) != 2:
            continue
        try:
            arguments = shlex.split(fields[1])
            pid = int(fields[0])
        except ValueError:
            continue
        if arguments and arguments[0] == expected:
            pids.add(pid)
    return pids


def pids_for_invocation(executable, config):
    config_arg = f'--config={config}'
    result = subprocess.run(
        ['/bin/ps', '-axo', 'pid=,command='],
        check=True,
        capture_output=True,
        text=True,
    )
    expected = str(executable)
    pids = set()
    for line in result.stdout.splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) != 2:
            continue
        try:
            arguments = shlex.split(fields[1])
            pid = int(fields[0])
        except ValueError:
            continue
        if arguments and arguments[0] == expected and config_arg in arguments[1:]:
            pids.add(pid)
    return pids


def terminate_invocation(executable, config, grace_seconds=4):
    pids = pids_for_invocation(executable, config)
    for pid in pids:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass

    deadline = time.monotonic() + grace_seconds
    while time.monotonic() < deadline:
        remaining = pids_for_invocation(executable, config)
        if not remaining:
            return
        time.sleep(0.1)

    for pid in pids_for_invocation(executable, config):
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--https-url', default='https://example.com/')
    parser.add_argument('--trusted-debug', action='store_true', help='Exercise explicit debug-origin token authorization instead of remote grants')
    parser.add_argument('--lease-only', action='store_true', help='Run only the IPC lease and child-cleanup regression')
    parser.add_argument('--skip-lease-check', action='store_true', help='Skip the independent lease-expiry case while validating bridge calls and streams')
    parser.add_argument('--skip-session-liveness', action='store_true', help='Skip long-running heartbeat cases when the desktop WebView is background-suspended')
    parser.add_argument('--launch-services-bundle', help='Foreground-launch this macOS app bundle through LaunchServices')
    parser.add_argument('--startup-timeout', type=float, default=90.0)
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    if not binary.is_file():
        raise FileNotFoundError(binary)
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    report = {}
    page_loads = {'entry': 0, 'casesScript': 0, 'childStatusRequests': 0, 'childStatusResponses': 0}
    progress_posts = {'received': 0, 'written': 0, 'responses': 0}
    child_status_results = []
    progress_lock = threading.Lock()
    done = threading.Event()
    fixture = {'file': str(output / 'ipc-text.txt'), 'python': sys.executable,
               'directory': str(output / ('directory-' + uuid.uuid4().hex)),
               'httpsUrl': args.https_url,
               'progress': str(output / 'progress.json'),
               'started': str(output / 'lease-child-started.txt'),
               'orphan': str(output / 'lease-orphan.txt'),
               'command': 'echo niva-ipc' if os.name == 'nt' else 'printf niva-ipc',
               'trustedDebug': args.trusted_debug}
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
            self.send_header('Cache-Control', 'no-store')
            self.send_header('Content-Security-Policy', "default-src 'self'; script-src 'self'; connect-src 'self'")
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self):
            path = self.path.split('?', 1)[0]
            if path == '/':
                page_loads['entry'] += 1
                self.respond(200, b'<!doctype html><meta charset="utf-8"><title>IPC fallback smoke</title><pre id="status">Running...</pre><script src="/cases.js"></script>', 'text/html')
            elif path == '/cases.js':
                page_loads['casesScript'] += 1
                self.respond(200, (HERE / 'cases.js').read_bytes(), 'application/javascript')
            elif self.path == '/fixture':
                self.respond(200, json.dumps(fixture).encode(), 'application/json')
            elif self.path == '/text':
                self.respond(200, 'HTTP 中文'.encode(), 'text/plain; charset=utf-8')
            elif self.path == '/child-started':
                with progress_lock:
                    page_loads['childStatusRequests'] += 1
                    started = Path(fixture['started']).exists()
                    child_status_results.append(started)
                self.respond(200, json.dumps({'started': started}).encode(), 'application/json')
                with progress_lock:
                    page_loads['childStatusResponses'] += 1
            else:
                self.respond(404, b'missing', 'text/plain')

        def do_POST(self):
            if self.path == '/progress':
                length = int(self.headers.get('Content-Length', '0'))
                data = self.rfile.read(length)
                with progress_lock:
                    progress_posts['received'] += 1
                    temporary = output / 'progress.json.tmp'
                    temporary.write_bytes(data)
                    temporary.replace(output / 'progress.json')
                    progress_posts['written'] += 1
                self.respond(200, b'ok', 'text/plain')
                with progress_lock:
                    progress_posts['responses'] += 1
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
    query = []
    if args.lease_only:
        query.append('leaseOnly=1')
    if args.skip_lease_check:
        query.append('skipLease=1')
    if args.skip_session_liveness:
        query.append('skipSessionLiveness=1')
    entry = origin + '/' + (('?' + '&'.join(query)) if query else '')
    config.write_text(json.dumps({
        'name': 'Niva IPC fallback smoke', 'uuid': str(uuid.uuid4()),
        'injectCommonJs': False, 'injectEsm': False,
        'window': {'entry': entry, 'visible': True,
                   'permissions': {} if args.trusted_debug else {origin: ['fs.readText', 'fs.writeText', 'fs.appendText', 'fs.node',
                                           'http.requestText', 'process.execText', 'process.execFileText']}},
    }, indent=2))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    log = (output / 'stdio.log').open('wb')
    process = None
    launcher = None
    launch_services_bundle = None
    launch_services_executable = None
    launch_services_pid = None
    try:
        command = [str(binary), f'--config={config}', f'--resource={output}']
        if args.trusted_debug:
            command.append(f'--debug-entry={entry}')
        if args.launch_services_bundle:
            if sys.platform != 'darwin':
                raise RuntimeError('--launch-services-bundle is supported only on macOS')
            launch_services_bundle = Path(args.launch_services_bundle).resolve()
            launch_services_executable = launch_services_bundle / 'Contents' / 'MacOS' / 'niva'
            if not launch_services_executable.is_file():
                raise FileNotFoundError(launch_services_executable)
            expected_sha = hashlib.sha256(binary.read_bytes()).hexdigest()
            bundle_sha = hashlib.sha256(launch_services_executable.read_bytes()).hexdigest()
            if bundle_sha != expected_sha:
                raise RuntimeError(f'LaunchServices bundle binary SHA mismatch: expected {expected_sha}, found {bundle_sha}')
            existing = pids_for_executable(launch_services_executable)
            if existing:
                raise RuntimeError(f'LaunchServices bundle is already running; refusing to reuse PIDs {sorted(existing)}')
            launcher = subprocess.Popen(
                ['/usr/bin/open', '-a', str(launch_services_bundle), '--args', *command[1:]],
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            try:
                launcher.wait(timeout=10)
            except subprocess.TimeoutExpired as error:
                raise TimeoutError('LaunchServices open command did not return within 10 seconds') from error
            if launcher.returncode != 0:
                raise RuntimeError(f'LaunchServices open exited with {launcher.returncode}')
            launch_deadline = time.monotonic() + 10
            while time.monotonic() < launch_deadline:
                started_pids = pids_for_invocation(launch_services_executable, config)
                if started_pids:
                    launch_services_pid = min(started_pids)
                    break
                time.sleep(0.1)
            if launch_services_pid is None:
                raise TimeoutError('LaunchServices did not start the Niva process with this config')
        else:
            process = subprocess.Popen(command,
                                       stdout=log, stderr=subprocess.STDOUT, stdin=subprocess.PIPE)
        deadline = time.monotonic() + args.startup_timeout
        while not done.wait(.2):
            if process is not None and process.poll() is not None:
                raise RuntimeError(f'Niva exited before reporting: {process.returncode}')
            if launch_services_executable is not None:
                if launch_services_pid not in pids_for_invocation(launch_services_executable, config):
                    raise RuntimeError('LaunchServices Niva process exited before reporting')
            if time.monotonic() > deadline:
                progress_path = Path(fixture['progress'])
                progress = json.loads(progress_path.read_text()) if progress_path.exists() else None
                raise TimeoutError(f'remote IPC page did not report within {args.startup_timeout:g} seconds; progress={progress!r}')
        # Keep the app alive past the child's scheduled side effect: terminating
        # Niva first could hide a failure to kill the expired IPC child.
        if Path(fixture['started']).exists():
            # The child writes after six seconds. Its IPC call can now be
            # rejected promptly at the three-second lease boundary, so retain
            # the app until the delayed write would have happened.
            time.sleep(3.5)
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
        if launch_services_executable is not None:
            terminate_invocation(launch_services_executable, config)
        if launcher is not None and launcher.poll() is None:
            launcher.terminate()
            try:
                launcher.wait(timeout=3)
            except subprocess.TimeoutExpired:
                launcher.kill()
                launcher.wait(timeout=3)
        log.close()
        server.shutdown()
        server.server_close()
    report.update(engine='real-webview-ipc', authorization='trusted-debug-token' if args.trusted_debug else 'explicit-remote-grants',
                  nativeBinarySha256=hashlib.sha256(binary.read_bytes()).hexdigest(), pageLoads=page_loads,
                  progressPosts=progress_posts, childStatusResults=child_status_results,
                  leaseCheck='skipped' if args.skip_lease_check or args.skip_session_liveness else 'included',
                  sessionLiveness='skipped' if args.skip_session_liveness else 'included',
                  launchServicesBundle=str(launch_services_bundle) if launch_services_bundle else None,
                  launchServicesPid=launch_services_pid)
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report.get('ok') else 1)


if __name__ == '__main__':
    main()

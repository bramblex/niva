"""Run the pinned application's unmodified CommonJS backend under reference Node."""
import argparse
import json
import os
import socket
import subprocess
import time
import urllib.request
from pathlib import Path
from api_suite import run


def start(root, output, index):
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
    env = os.environ.copy()
    env.update(VITE_BACKEND_PORT=str(port), PORT='3000', NODE_ENV='test')
    log = (output / f'node-{index}.log').open('wb')
    process = subprocess.Popen(['node', 'backend/app.js'], cwd=root, env=env, stdout=log, stderr=subprocess.STDOUT)
    base = f'http://127.0.0.1:{port}'
    try:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError(f'Reference backend exited {process.returncode}')
            try:
                with urllib.request.urlopen(base + '/', timeout=1) as response:
                    if response.status == 200: return process, log, base
            except OSError:
                time.sleep(.1)
        raise TimeoutError('Reference backend startup deadline')
    except BaseException:
        stop(process, log)
        raise


def stop(process, log):
    if process.poll() is None: process.terminate()
    try: process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill(); process.wait(timeout=5)
    if process.stdin is not None:
        process.stdin.close()
    log.close()


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('compiled'); parser.add_argument('--output', required=True)
    args = parser.parse_args(); root = Path(args.compiled).resolve(); output = Path(args.output).resolve(); output.mkdir(parents=True, exist_ok=True)
    manifest = json.loads((root / 'build-manifest.json').read_text())
    version = subprocess.check_output(['node', '--version'], text=True).strip()
    if version != 'v' + manifest['baselineNode']: raise RuntimeError('Reference Node version does not match manifest')
    process, log, base = start(root, output, 1)
    try: report = run(base, root / 'data/database.json')
    finally: stop(process, log)
    if report['ok']:
        process, log, base = start(root, output, 2)
        try: persisted = run(base, root / 'data/database.json', report['created']['account'])
        finally: stop(process, log)
        report['checks'].extend(persisted['checks']); report['ok'] = report['ok'] and persisted['ok']
    report.update(engine='reference-node', node=version, upstreamCommit=manifest['actualCommit'], dependencyLockSha256=manifest['dependencyLockSha256'])
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['ok'] else 1)

if __name__ == '__main__': main()

"""Execute the pinned, compiled backend inside Niva; Python only drives HTTP."""
import argparse
import json
import os
import socket
import subprocess
import time
import urllib.request
import uuid
from pathlib import Path
from api_suite import run
from reference import stop


RUNNER = r'''"use strict";
window.addEventListener("load", function () {
  try {
    if (typeof require !== "function" || require("fs").readFile !== Niva.fs.readFile) {
      throw new Error("Niva CommonJS interface identity is missing");
    }
    require("./backend/app.js");
    document.getElementById("status").textContent = "Original RealWorld backend loaded in Niva";
    if (globalThis.NIVA_ACCEPTANCE_UI) {
      Niva.window.open({entry: globalThis.NIVA_ACCEPTANCE_UI, title: "RealWorld original frontend", visible: true})
        .catch(error => Niva.fs.writeFileSync("niva-runner-error.json", JSON.stringify({message: String(error), stack: error.stack}), "utf8"));
    }
  } catch (error) {
    var failure = {message: String(error), stack: error && error.stack};
    document.getElementById("status").textContent = JSON.stringify(failure, null, 2);
    try { Niva.fs.writeFileSync("niva-runner-error.json", JSON.stringify(failure), "utf8"); }
    catch (_) { console.error(failure); }
  }
});
'''


def start(root, output, binary, index, timeout, backend_port=None, frontend_port=3000, ui_url=None):
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = backend_port or sock.getsockname()[1]
    failure = root / 'niva-runner-error.json'
    failure.unlink(missing_ok=True)
    (root / 'niva-runner.js').write_text('globalThis.NIVA_ACCEPTANCE_UI=' + json.dumps(ui_url) + ';\n' + RUNNER)
    (root / 'niva-runner.html').write_text(
        '<!doctype html><meta charset="utf-8"><title>RealWorld Niva acceptance</title>'
        '<h1>RealWorld backend in Niva</h1><pre id="status">Loading original backend…</pre>'
        '<script src="niva-runner.js"></script>')
    config = output / f'niva-{index}.json'
    config.write_text(json.dumps({
        'name': 'Niva RealWorld acceptance', 'uuid': str(uuid.uuid4()),
        'injectCommonJs': True, 'injectEsm': False,
        'window': {'entry': 'niva-runner.html', 'title': 'Niva RealWorld acceptance',
                   'size': {'width': 780, 'height': 560}, 'visible': True},
    }, indent=2))
    env = os.environ.copy()
    env.update(VITE_BACKEND_PORT=str(port), PORT=str(frontend_port), NODE_ENV='test')
    log = (output / f'niva-{index}.stdio.log').open('wb')
    process = subprocess.Popen([str(binary), f'--config={config}', f'--resource={root}'],
                               cwd=root, env=env, stdin=subprocess.PIPE,
                               stdout=log, stderr=subprocess.STDOUT)
    base = f'http://127.0.0.1:{port}'
    try:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if failure.exists():
                raise RuntimeError(f'Niva backend load failed: {failure.read_text()}')
            if process.poll() is not None:
                raise RuntimeError(f'Niva exited {process.returncode}; see {log.name}')
            try:
                with urllib.request.urlopen(base + '/', timeout=1) as response:
                    if response.status == 200:
                        return process, log, base
            except OSError:
                time.sleep(.1)
        raise TimeoutError(f'Niva backend startup deadline; see {log.name}')
    except BaseException:
        stop(process, log)
        raise


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('compiled')
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--startup-timeout', type=float, default=120)
    args = parser.parse_args()
    root = Path(args.compiled).resolve()
    binary = Path(args.binary).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    if not binary.is_file():
        raise FileNotFoundError(binary)
    manifest = json.loads((root / 'build-manifest.json').read_text())
    report = {'ok': False, 'checks': [], 'engine': 'niva-webview',
              'upstreamCommit': manifest['actualCommit'],
              'dependencyLockSha256': manifest['dependencyLockSha256']}
    import hashlib
    report['nativeBinarySha256'] = hashlib.sha256(binary.read_bytes()).hexdigest()
    try:
        process, log, base = start(root, output, binary, 1, args.startup_timeout)
        try:
            first = run(base, root / 'data/database.json')
            report.update(first)
        finally:
            stop(process, log)
        if report['ok']:
            process, log, base = start(root, output, binary, 2, args.startup_timeout)
            try:
                persisted = run(base, root / 'data/database.json', report['created']['account'])
            finally:
                stop(process, log)
            report['checks'].extend(persisted['checks'])
            report['ok'] = report['ok'] and persisted['ok']
    except Exception as error:
        report['ok'] = False
        report['error'] = str(error)
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__':
    main()

"""Verify real Niva process pipes, split UTF-8, and EOF without auto-exit."""
import argparse
import hashlib
import json
from pathlib import Path
import queue
import subprocess
import threading
import time
import uuid


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    binary = Path(args.binary).resolve()
    if not binary.is_file():
        raise FileNotFoundError(binary)
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    resources = Path(__file__).resolve().parent
    config = json.loads((resources / 'niva.json').read_text())
    config['uuid'] = str(uuid.uuid4())
    config_path = output / 'niva.json'
    config_path.write_text(json.dumps(config))
    stderr = (output / 'stderr.bin').open('wb')
    process = subprocess.Popen([str(binary), f'--resource={resources}', f'--config={config_path}'],
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=stderr)
    lines = queue.Queue()
    collected = bytearray()

    def reader():
        while True:
            line = process.stdout.readline()
            if not line:
                lines.put(None)
                return
            collected.extend(line)
            lines.put(line)

    thread = threading.Thread(target=reader, daemon=True)
    thread.start()
    checks = []
    report = {'ok': False, 'checks': checks, 'nativeBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest()}

    def expect_line(expected):
        try:
            value = lines.get(timeout=30)
        except queue.Empty as error:
            raise TimeoutError(f'Niva page did not write the expected application line within 30 seconds: {expected!r}') from error
        if value != expected:
            raise AssertionError(f'got {value!r}, expected {expected!r}')

    try:
        expect_line(b'ready\n')
        checks.append({'name': 'application readiness is first stdout content', 'ok': True})
        payload = '你好，parent\n'.encode()
        for fragment in (payload[:1], payload[1:4], payload[4:]):
            process.stdin.write(fragment)
            process.stdin.flush()
            time.sleep(.1)
        expect_line('Hello, 你好，parent!\n'.encode())
        checks.append({'name': 'pipe preserves split UTF-8 application text', 'ok': True})
        process.stdin.close()
        expect_line(b'stdin-eof\n')
        checks.append({'name': 'stdin emits end after EOF', 'ok': True})
        time.sleep(.5)
        if process.poll() is not None:
            raise AssertionError('EOF unexpectedly terminated UI')
        checks.append({'name': 'window remains alive after stdin EOF', 'ok': True})
        if not lines.empty():
            raise AssertionError('unexpected extra stdout after application protocol')
        stderr.flush()
        if (output / 'stderr.bin').stat().st_size:
            raise AssertionError('framework or WebView emitted unexpected stderr; inspect stderr.bin')
        checks.append({'name': 'stderr stays free of framework diagnostics during exchange', 'ok': True})
        report['ok'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        if process.poll() is None:
            process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)
        if not process.stdin.closed:
            process.stdin.close()
        thread.join(timeout=2)
        process.stdout.close()
        stderr.close()
        (output / 'stdout.bin').write_bytes(collected)
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__':
    main()

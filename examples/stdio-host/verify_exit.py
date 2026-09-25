"""Check that process.exit cleans owned children before the native process ends."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
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
    started = output / 'started.json'
    orphan = output / 'orphan.txt'
    started.unlink(missing_ok=True)
    orphan.unlink(missing_ok=True)
    child_program = ('import json,os,pathlib,time; pathlib.Path(' + repr(str(started)) +
                     ').write_text(json.dumps({"pid":os.getpid()})); time.sleep(4); pathlib.Path(' +
                     repr(str(orphan)) + ').write_text("survived parent exit")')
    (output / 'index.html').write_text('<!doctype html><meta charset="utf-8"><title>Native exit cleanup</title><script src="case.js"></script>')
    script = '''(async () => {
  const child = Niva.child_process.spawn(PYTHON, ['-c', PROGRAM]);
  child.on('error', error => { console.error(error); Niva.process.exit(1); });
  const deadline = Date.now() + 10000;
  while (!Niva.fs.existsSync(STARTED)) {
    if (Date.now() > deadline) throw new Error('child did not start');
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  Niva.process.exit(0);
})().catch(error => { console.error(error); Niva.process.exit(1); });
'''
    for key, value in [('PYTHON', sys.executable), ('PROGRAM', child_program), ('STARTED', str(started))]:
        script = script.replace(key, json.dumps(value))
    (output / 'case.js').write_text(script)
    (output / 'niva.json').write_text(json.dumps({'name': 'Niva exit cleanup', 'uuid': str(uuid.uuid4()),
        'window': {'entry': 'index.html', 'visible': True}}))
    report = {'ok': False, 'nativeBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    with (output / 'stdio.log').open('wb') as log:
        process = subprocess.Popen([str(binary), f'--resource={output}'], stdout=log, stderr=subprocess.STDOUT)
        try:
            status = process.wait(timeout=30)
            if status != 0:
                raise AssertionError(f'Niva exit status {status}')
            if not started.exists():
                raise AssertionError('fixture never started its child')
            # Crucially, do not terminate the child from this driver before
            # observing whether its scheduled side effect happened.
            time.sleep(4.5)
            if orphan.exists():
                raise AssertionError('owned child survived Niva.process.exit')
            report.update(ok=True, checks=[{'name': 'process.exit stops its owned child before delayed mutation', 'ok': True}])
        except Exception as error:
            report['error'] = str(error)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__':
    main()

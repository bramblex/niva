"""Run the unmodified TypeScript CLI in Niva's CommonJS realm, without host Node."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import uuid


def run_case(binary, compiler, root, invalid):
    root.mkdir(parents=True, exist_ok=True)
    (root / 'src').mkdir(exist_ok=True)
    (root / 'src' / 'greet.ts').write_text('export function greet(name: string): string { return `Hello ${name}`; }\n')
    (root / 'src' / 'index.ts').write_text(
        'import { greet } from "./greet";\n' +
        ('const result: number = greet("Niva");\n' if invalid else 'export const result: string = greet("Niva");\n'))
    (root / 'tsconfig.json').write_text(json.dumps({
        'compilerOptions': {'strict': True, 'target': 'ES2020', 'module': 'CommonJS',
                            'outDir': 'out', 'noEmitOnError': True, 'types': [],
                            'declaration': True, 'sourceMap': True},
        'include': ['src/**/*.ts'],
    }))
    entry = compiler / 'lib' / 'tsc.js'
    (root / 'runner.js').write_text(
        'window.addEventListener("load", () => {\n'
        '  try {\n'
        '    if (require("fs") !== Niva.fs) throw new Error("Host fs substitution");\n'
        f'    process.argv = [process.execPath, {json.dumps(str(entry))}, "--project", {json.dumps(str(root / "tsconfig.json"))}, "--pretty", "false"];\n'
        f'    require({json.dumps(str(entry))});\n'
        '  } catch (error) {\n'
        f'    Niva.fs.writeFileSync({json.dumps(str(root / "runner-error.json"))}, JSON.stringify({{message: String(error), stack: error.stack}}), "utf8");\n'
        '    Niva.process.exit(70);\n'
        '  }\n'
        '});\n')
    (root / 'index.html').write_text('<!doctype html><meta charset="utf-8"><title>TypeScript CLI in Niva</title><script src="runner.js"></script>')
    (root / 'niva.json').write_text(json.dumps({
        'name': 'TypeScript CLI smoke', 'uuid': str(uuid.uuid4()),
        'injectCommonJs': True, 'injectEsm': False,
        'window': {'entry': 'index.html', 'visible': True},
    }))
    with (root / 'stdout.log').open('wb') as stdout, (root / 'stderr.log').open('wb') as stderr:
        process = subprocess.Popen([str(binary), f'--resource={root}'], cwd=root, stdout=stdout, stderr=stderr)
        try:
            code = process.wait(timeout=90)
        except subprocess.TimeoutExpired:
            process.terminate()
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: process.kill(); process.wait()
            return {'ok': False, 'invalid': invalid, 'error': 'CLI did not exit within 90 seconds'}
    result = {'ok': False, 'invalid': invalid, 'exitCode': code}
    failure = root / 'runner-error.json'
    if failure.exists():
        result['error'] = json.loads(failure.read_text())
    elif invalid:
        diagnostic = (root / 'stdout.log').read_text(errors='replace')
        result['ok'] = code == 1 and 'TS2322' in diagnostic and not (root / 'out').exists()
        result['diagnostic'] = diagnostic
    else:
        expected = ['index.js', 'greet.js', 'index.d.ts', 'greet.d.ts', 'index.js.map', 'greet.js.map']
        result['emittedFiles'] = [name for name in expected if (root / 'out' / name).is_file()]
        result['ok'] = code == 0 and len(result['emittedFiles']) == len(expected)
        if result['ok']:
            result['ok'] = 'require("./greet")' in (root / 'out' / 'index.js').read_text()
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', required=True)
    parser.add_argument('--compiler', default='node_modules/typescript')
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    binary, compiler, output = map(lambda value: Path(value).resolve(), (args.binary, args.compiler, args.output))
    output.mkdir(parents=True, exist_ok=True)
    version = json.loads((compiler / 'package.json').read_text())['version']
    cases = []
    for invalid in [False, True]:
        root = output / ('invalid' if invalid else 'valid')
        if root.exists(): raise FileExistsError(f'Use a fresh evidence directory: {root}')
        case = run_case(binary, compiler, root, invalid)
        cases.append(case)
        print(json.dumps(case), flush=True)
    report = {'ok': all(case['ok'] for case in cases), 'cases': cases, 'typescriptVersion': version,
              'engine': 'niva-webview', 'nativeBinarySha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
              'compilerSha256': hashlib.sha256((compiler / 'lib' / '_tsc.js').read_bytes()).hexdigest()}
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    raise SystemExit(0 if report['ok'] else 1)


if __name__ == '__main__': main()

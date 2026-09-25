// Only the selected callbackify fixture subprocesses use this test entrypoint.
import assert from 'node:assert';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import Module from 'node:module';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../upstream');
const manifest = JSON.parse(readFileSync(path.join(root, 'manifest.json')));
const file = path.resolve(process.argv[2]);
const entry = manifest.files.find(row => row.kind === 'fixture' && path.resolve(root, 'node-v22.14.0', row.path) === file);
assert(entry && /^test\/fixtures\/uncaught-exceptions\/callbackify[12]\.js$/.test(entry.path));
const source = readFileSync(file, 'utf8');
assert.equal(createHash('sha256').update(source).digest('hex'), entry.sha256);
globalThis.__niva_runtime_config = { injectCommonJs: false, injectEsm: false };
globalThis.__niva_node_bootstrap = {
  nivaVersion: 'v0.9.9',
  os: { platform: process.platform, arch: process.arch, homedir: process.env.HOME || '/', tmpdir: process.env.TMPDIR || '/tmp', EOL: process.platform === 'win32' ? '\r\n' : '\n' },
  process: { version: 'v0.9.9', versions: { niva: '0.9.9' }, argv: process.argv.slice(), env: Object.assign({}, process.env), execPath: process.execPath, pid: process.pid, arch: process.arch, platform: process.platform },
};
await import('../dist/bootstrap.js');
globalThis.Niva.bridge.callSync = (method, args = []) => {
  assert.equal(method, 'process.currentDir');
  assert.deepEqual(args, []);
  return process.cwd();
};
const util = (await import('../dist/source/util.js')).default;
assert.notStrictEqual(util.callbackify, (await import('node:util')).callbackify);
process.argv = [process.execPath, file];
const module = new Module(file);
module.filename = file;
module.require = id => {
  if (id === 'util') return util;
  if (id === 'assert') return assert;
  throw new Error('Unknown fixture dependency: ' + id);
};
module._compile(source, file);

// Only the selected callbackify fixture subprocesses use this test entrypoint.
import assert from 'node:assert';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import Module from 'node:module';
import util from '../src/util.js';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../upstream');
const manifest = JSON.parse(readFileSync(path.join(root, 'manifest.json')));
const file = path.resolve(process.argv[2]);
const entry = manifest.files.find(row => row.kind === 'fixture' && path.resolve(root, 'node-v22.14.0', row.path) === file);
assert(entry && /^test\/fixtures\/uncaught-exceptions\/callbackify[12]\.js$/.test(entry.path));
const source = readFileSync(file, 'utf8');
assert.equal(createHash('sha256').update(source).digest('hex'), entry.sha256);
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

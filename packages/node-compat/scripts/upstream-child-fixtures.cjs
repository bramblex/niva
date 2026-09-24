// Preload only for the external pinned-Node child_process test fixtures.
// Product spawning remains Niva Native. Fixtures themselves remain unchanged.
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const Module = require('node:module');
const upstream = path.resolve(__dirname, '../upstream');
const manifest = JSON.parse(fs.readFileSync(path.join(upstream, 'manifest.json')));
const original = Module._extensions['.js'];
Module._extensions['.js'] = function(module, filename) {
  const relative = path.relative(path.join(upstream, 'node-v22.14.0'), filename).split(path.sep).join('/');
  if (!['test/fixtures/exit.js','test/fixtures/echo.js'].includes(relative)) return original(module, filename);
  assert.equal(process.version, 'v22.14.0');
  const entry = manifest.files.find(row => row.kind === 'fixture' && row.path === relative);
  const source = fs.readFileSync(filename, 'utf8');
  assert.equal(crypto.createHash('sha256').update(source).digest('hex'), entry.sha256);
  module.require = name => {
    if (name === 'assert') return assert;
    if (name === '../common') return new Proxy({}, {get(_, key) {throw new Error('Unknown fixture helper: '+String(key));}});
    throw new Error('Unknown child fixture dependency: '+name);
  };
  module._compile(source, filename);
};

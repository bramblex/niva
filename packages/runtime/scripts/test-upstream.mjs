// Test infrastructure only. Never loaded into the Niva release bundle.
import assert from 'node:assert';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync, mkdirSync, writeFileSync, writeSync, mkdtempSync, rmSync, existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { spawnSync, execFile as hostExecFile } from 'node:child_process';
import { createRequire } from 'node:module';
import vm from 'node:vm';
import { inspect } from 'node:util';
import * as hostUtil from 'node:util';
import { totalmem as hostTotalMemory, tmpdir as hostTmpdir, homedir as hostHomeDir, hostname as hostHostname, release as hostOsRelease, type as hostOsType } from 'node:os';
import { startNativeBridge } from './upstream-native-bridge.mjs';
import { environmentPolicy, policySha256, prepareEnvironmentExclusions } from './upstream-environment-exclusions.mjs';

const script = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(script), '..');
const upstream = path.join(root, 'upstream');
const snapshot = path.join(upstream, 'node-v22.14.0');
const includeEnvironmentSpecific = process.argv.includes('--include-environment-specific');
const args = process.argv.slice(2).filter(arg => arg !== '--include-environment-specific');
const hostBuffer = globalThis.Buffer;
const hostProcess = process;
const infrastructureRequire = createRequire(import.meta.url);
function hostBootstrap() {
  const platform = hostProcess.platform;
  return {
    nivaVersion: 'v0.9.9',
    os: {
      info: { os: platform, arch: hostProcess.arch, version: hostOsRelease() },
      arch: hostProcess.arch,
      platform,
      homedir: hostHomeDir(),
      tmpdir: hostTmpdir(),
      hostname: hostHostname(),
      release: hostOsRelease(),
      type: hostOsType(),
      version: hostOsRelease(),
      EOL: platform === 'win32' ? '\r\n' : '\n',
      totalmem: hostTotalMemory(),
    },
    process: {
      arch: hostProcess.arch,
      platform,
      argv: hostProcess.argv.slice(),
      env: Object.assign({}, hostProcess.env),
      execPath: hostProcess.execPath,
      pid: hostProcess.pid,
      version: 'v0.9.9',
      versions: { niva: '0.9.9' },
    },
  };
}
async function installRuntimeFixture(nativeBridge) {
  globalThis.__niva_node_bootstrap = nativeBridge?.bootstrap || hostBootstrap();
  globalThis.__niva_runtime_config = { injectCommonJs: false, injectEsm: false };
  delete globalThis.Niva;
  await import(pathToFileURL(path.join(root, 'dist/bootstrap.js')).href);
  if (nativeBridge) {
    for (const name of ['call', 'callSync', 'stream', 'streamSend']) {
      globalThis.Niva.bridge[name] = nativeBridge[name];
    }
  } else {
    globalThis.Niva.bridge.callSync = (method, parameters = []) => {
      assert.equal(method, 'process.currentDir', 'Only cwd is provided by the contract OS fixture');
      assert.deepStrictEqual(parameters, []);
      return hostProcess.cwd();
    };
    globalThis.Niva.bridge.call = (method) => Promise.reject(new Unsupported(`Native call is unavailable in the JS contract suite: ${method}`));
    globalThis.Niva.bridge.stream = (method) => { throw new Unsupported(`Native stream is unavailable in the JS contract suite: ${method}`); };
    globalThis.Niva.bridge.streamSend = () => { throw new Unsupported('Native stream send is unavailable in the JS contract suite'); };
  }
}
const marker = '@@NIVA_UPSTREAM_RESULT@@';
const sha256 = data => createHash('sha256').update(data).digest('hex');
class Unsupported extends Error {
  constructor(message) { super(message); this.name = 'Unsupported'; }
}
function strictObject(value, label, onUnknown) {
  return new Proxy(value, {
    get(target, name, receiver) {
      if (!Reflect.has(target, name)) return onUnknown(`Unimplemented ${label}: ${String(name)}`);
      return Reflect.get(target, name, receiver);
    },
  });
}
function verifySnapshot() {
  assert.equal(hostProcess.version, 'v22.14.0', 'Official contract gate requires the pinned Node v22.14.0 test host');
  const manifestText = readFileSync(path.join(upstream, 'manifest.json'));
  assert.equal(sha256(manifestText), '4daa582a0a347db764cae3e5d09de8a0ac16f54ed3b34e09a509090ac8e1a350', 'Expanded 58-file manifest changed; review and repin selection explicitly');
  const manifest = JSON.parse(manifestText);
  assert.equal(manifest.schemaVersion, 1);
  assert.equal(manifest.upstream.commit, '5d2feb257bcee090e57900eb51720171a6aa92f3');
  assert.equal(manifest.upstream.version, 'v22.14.0');
  const seen = new Set();
  for (const entry of manifest.files) {
    assert(!seen.has(entry.path), `Duplicate manifest entry: ${entry.path}`);
    seen.add(entry.path);
    const absolute = path.resolve(snapshot, entry.path);
    assert(absolute.startsWith(snapshot + path.sep), `Invalid snapshot path: ${entry.path}`);
    assert.equal(sha256(readFileSync(absolute)), entry.sha256, `Upstream checksum: ${entry.path}`);
    assert(['test', 'fixture', 'license'].includes(entry.kind), 'Unknown entry kind');
  }
  const tests = manifest.files.filter(entry => entry.kind === 'test');
  const baselineText = readFileSync(path.join(upstream, 'baseline-manifest.json'));
  assert.equal(sha256(baselineText), '7f60ad27203feed67c223519d881b1337eb3e5618846080360c63b057d375a71', 'Original baseline manifest changed');
  const baseline = JSON.parse(baselineText);
  assert.equal(baseline.files.filter(row => row.kind === 'test').length, 30);
  for (const original of baseline.files) assert(manifest.files.some(row => row.path === original.path && row.sha256 === original.sha256 && row.kind === original.kind), `Original selection changed: ${original.path}`);
  assert.equal(tests.length, 58, 'Frozen expanded selection count');
  const onDisk = readdirSync(path.join(snapshot, 'test/parallel')).filter(name => name.endsWith('.js'));
  assert.deepStrictEqual(tests.map(entry => path.basename(entry.path)).sort(), onDisk.sort());
  return { manifest, tests };
}
function verifyNativeRelayContract() {
  const relay = readFileSync(path.join(root, 'scripts/upstream-native-relay.py'), 'utf8');
  const bridge = readFileSync(path.join(root, 'scripts/upstream-native-bridge.mjs'), 'utf8');
  for (const legacy of [/--stdio\b/, /--debug-resource(?:=|\b)/, /Niva\.api\.host/, /\bNiva\.(?:call|callSync|stream|streamSend)\b/, /NivaNodeCompatReady/]) {
    assert.doesNotMatch(relay, legacy, `Native relay must not use legacy control path ${legacy}`);
  }
  assert.match(relay, /Niva\.bridge\.callSync\(/);
  assert.match(relay, /Niva\.bridge\.call\(/);
  assert.match(relay, /Niva\.bridge\.stream\(/);
  assert.match(relay, /Niva\.bridge\.streamSend\(/);
  assert.match(relay, /process\.stdin\.on\("data"/);
  assert.match(relay, /process\.stdout\.write\(/);
  assert.match(relay, /'--config='/);
  assert.match(relay, /'--resource='/);
  assert.match(bridge, /upstream-native-relay\.py/);
}
function sourceFingerprint() {
  const files = {};
  for (const directory of ['src', 'src/runtime']) {
    for (const name of readdirSync(path.join(root, directory)).sort()) {
      if (name.endsWith('.js') || name.endsWith('.ts')) {
        const relative = `${directory}/${name}`;
        files[relative] = sha256(readFileSync(path.join(root, relative)));
      }
    }
  }
  return files;
}
function child(name, synthetic = false) {
  const temporary = mkdtempSync(path.join(hostTmpdir(), 'niva-upstream-result-'));
  const resultFile = path.join(temporary, 'result.json');
  const result = spawnSync(hostProcess.execPath,
    ['--expose-internals', '--no-warnings', '--unhandled-rejections=strict', script, synthetic ? '--probe' : '--case', name, '--result-file', resultFile, ...(includeEnvironmentSpecific ? ['--include-environment-specific'] : [])],
    { cwd: root, encoding: 'utf8', timeout: synthetic && name === 'timeout' ? 300 : 45000, maxBuffer: 2 * 1024 * 1024 });
  let record;
  try { if (existsSync(resultFile)) record = JSON.parse(readFileSync(resultFile, 'utf8')); } catch {}
  finally { rmSync(temporary, {recursive: true, force: true}); }
  if (result.error || result.signal || !record || (result.status !== 0 && record.status === 'pass')) {
    record = { status: 'fail', reason: result.error?.message || result.signal || `Missing/invalid result or abnormal exit (${result.status})` };
  }
  return { ...record, file: name, exitCode: result.status, signal: result.signal,
    stderr: (result.stderr || '').slice(-5000) };
}
async function runCase(name, synthetic) {
  let settled = false;
  let naturallyDrained = false;
  let problem;
  let blocked;
  const checks = [];
  const subtests = [];
  const environmentExclusions = [];
  let executedSourceSha256;
  const finish = () => {
    if (settled) return;
    settled = true;
    if (!naturallyDrained) problem ||= new Error('Test exited before the event loop drained');
    try { for (const check of checks) check(); } catch (error) { problem ||= error; }
    const error = problem || blocked;
    const status = error ? (error instanceof Unsupported ? 'unsupported' : 'fail') : 'pass';
    // Durable one-shot result: exit handlers cannot flush a large nonblocking pipe.
    const value = JSON.stringify({ status, reason: error?.stack || null, ...(subtests.length ? {subtests} : {}), ...(environmentExclusions.length ? {environmentExclusions} : {}), ...(executedSourceSha256 ? {executedSourceSha256} : {}) });
    if (args[2] === '--result-file') writeFileSync(args[3], value + '\n');
    else {
      hostProcess.stdout._handle?.setBlocking?.(true);
      const record = hostBuffer.from(marker + value + '\n');
      for (let offset = 0; offset < record.length;) offset += writeSync(1, record, offset, record.length - offset);
    }
    if (error) hostProcess.exitCode = 1;
  };
  hostProcess.on('beforeExit', () => { naturallyDrained = true; });
  hostProcess.once('exit', finish);
  hostProcess.on('uncaughtException', error => { problem ||= error; if (settled) hostProcess.stderr.write('Late test failure: ' + error.stack + '\n'); hostProcess.exitCode = 1; setImmediate(() => hostProcess.exit(1)); });
  hostProcess.on('unhandledRejection', error => { problem ||= error; hostProcess.exitCode = 1; setImmediate(() => hostProcess.exit(1)); });
  const unsupported = message => { const error = new Unsupported(message); blocked ||= error; throw error; };
  function mustCall(fn = () => {}, expected = 1) {
    if (typeof fn === 'number') { expected = fn; fn = () => {}; }
    assert.equal(typeof fn, 'function');
    assert(Number.isInteger(expected) && expected >= 0);
    let calls = 0;
    checks.push(() => assert.equal(calls, expected, `mustCall(${fn.name || 'anonymous'})`));
    return function (...values) { calls++; return Reflect.apply(fn, this, values); };
  }
  function mustNotCall(message = 'mustNotCall invoked') {
    return function mustNotCall(...values) { const error = new Error(`${message}: ${inspect(values)}`); problem ||= error; throw error; };
  }
  // These three helpers follow Node v22.14.0 test/common/index.js. Node LICENSE
  // is retained in the snapshot. They are test infrastructure, not Niva APIs.
  function invalidArgTypeHelper(input) {
    if (input == null) return ` Received ${input}`;
    if (typeof input === 'function') return ` Received function ${input.name}`;
    if (typeof input === 'object') {
      if (input.constructor?.name) return ` Received an instance of ${input.constructor.name}`;
      return ` Received ${inspect(input, { depth: -1 })}`;
    }
    let inspected = inspect(input, { colors: false });
    if (inspected.length > 28) inspected = `${inspected.slice(inspected, 0, 25)}...`;
    return ` Received type ${typeof input} (${inspected})`;
  }
  function expectsError(validator, exact) {
    return mustCall((...values) => {
      assert.equal(values.length, 1);
      const error = values[0];
      assert.equal(Object.prototype.propertyIsEnumerable.call(error, 'message'), false);
      assert.throws(() => { throw error; }, validator);
      return true;
    }, exact);
  }
  function getArrayBufferViews(buf) {
    const { buffer, byteOffset, byteLength } = buf;
    return [Int8Array, Uint8Array, Uint8ClampedArray, Int16Array, Uint16Array,
      Int32Array, Uint32Array, Float32Array, Float64Array, BigInt64Array, BigUint64Array, DataView]
      .filter(type => byteLength % (type.BYTES_PER_ELEMENT || 1) === 0)
      .map(type => new type(buffer, byteOffset, byteLength / (type.BYTES_PER_ELEMENT || 1)));
  }
  function mustSucceed(fn = () => {}, expected = 1) {
    return mustCall(function(error, ...values) { assert.ifError(error); return Reflect.apply(fn, this, values); }, expected);
  }
  const warningExpectations = [];
  let warningListener;
  function expectWarning(type, message, code) {
    if (!warningListener) {
      warningListener = warning => {
        const expected = warningExpectations.find(row => !row.called && row.type === warning.name && row.code === warning.code && (row.message instanceof RegExp ? row.message.test(warning.message) : row.message === warning.message));
        assert(expected, 'Unexpected warning: ' + warning);
        expected.called = true;
      };
      hostProcess.on('warning', warningListener);
      checks.push(() => { for (const row of warningExpectations) assert(row.called, `Missing warning ${row.code}: ${row.message}`); });
    }
    if (typeof type !== 'string') return unsupported('Unsupported expectWarning overload');
    warningExpectations.push({type, message, code, called: false});
  }
  const common = strictObject({ mustCall, mustNotCall, mustSucceed, expectWarning, expectsError, invalidArgTypeHelper,
    getArrayBufferViews, isWindows: hostProcess.platform === 'win32', isLinux: hostProcess.platform === 'linux', localhostIPv4: '127.0.0.1',
    hasOpenSSL3: Number(hostProcess.versions.openssl?.split('.')[0]) >= 3,
    hasCrypto: Boolean(hostProcess.versions.openssl) && !hostProcess.env.NODE_SKIP_CRYPTO,
    get enoughTestMem() { return hostTotalMemory() > 0x70000000; },
    skip: reason => unsupported(`Upstream requested skip: ${reason}`),
    printSkipMessage: reason => unsupported(`Upstream requested skip: ${reason}`),
  }, 'test/common helper', unsupported);
  try {
    const selected = !synthetic && verifySnapshot().tests.find(test => test.path === name);
    let nativeFixture;
    if (selected?.execution === 'native') {
      const binary = hostProcess.env.NIVA_UPSTREAM_BINARY;
      if (!binary) return unsupported('Real Native tests require NIVA_UPSTREAM_BINARY');
      nativeFixture = startNativeBridge(path.resolve(binary));
    }
    if (selected?.module === 'child_process') hostProcess.env.NODE_OPTIONS = '--require ' + JSON.stringify(path.join(root, 'scripts/upstream-child-fixtures.cjs'));
    await installRuntimeFixture(nativeFixture?.bridge);
    const modules = {};
    for (const id of ['buffer', 'path', 'events', 'querystring', 'string_decoder', 'crypto', 'stream', 'stream/promises', 'timers', 'timers/promises', 'url', ...(selected?.module === 'assert' ? ['assert', 'assert/strict'] : []), ...(selected?.module === 'util' ? ['util'] : []), ...(nativeFixture ? ['fs', 'fs/promises', 'os', 'process', 'net', 'dgram', 'tls', 'http', 'https', 'dns', 'dns/promises', ...(selected?.module === 'child_process' ? ['child_process'] : [])] : [])]) {
      modules[id] = (await import(pathToFileURL(path.join(root, 'dist/source', `${id.replace('/', '-')}.js`)).href)).default;
    }
    Object.defineProperty(modules, 'zlib', { get: () => globalThis.Niva.zlib });
    assert.notStrictEqual(modules.path, path, 'Must not route to host Node path');
    assert.notStrictEqual(modules.buffer.Buffer, hostBuffer, 'Must not route to host Node Buffer');
    assert.notStrictEqual(modules.crypto.createHash, createHash, 'Crypto inputs must use Niva crypto, not host Node crypto');
    globalThis.Buffer = modules.buffer.Buffer;
    const requireNiva = id => {
      if (id === '../common') return common;
      if (id === '../common/tmpdir') {
        if (!nativeFixture) return unsupported('tmpdir requires a real Native fixture');
        const temporary = path.join(nativeFixture.directory, 'test-tmp');
        return strictObject({path:temporary, resolve:(...parts)=>path.resolve(temporary,...parts), refresh(){
          modules.fs.rmSync(temporary,{recursive:true,force:true});modules.fs.mkdirSync(temporary,{recursive:true});
        }}, 'tmpdir helper', unsupported);
      }
      if (id === '../common/crypto') return strictObject({hasOpenSSL3: Number(hostProcess.versions.openssl?.split('.')[0]) >= 3}, 'crypto platform helper', unsupported);
      if (id === '../common/fixtures') return strictObject({ readKey: (name) => {
        const relative = `test/fixtures/keys/${name}`;
        const {manifest} = verifySnapshot();
        assert(manifest.files.some(row => row.kind === 'fixture' && row.path === relative), 'Unknown key fixture');
        return modules.buffer.Buffer.from(readFileSync(path.join(snapshot, relative)));
      }, path: (...parts) => {
        const file = path.resolve(snapshot, 'test/fixtures', ...parts);
        const relative = path.relative(snapshot, file).split(path.sep).join('/');
        const { manifest } = verifySnapshot();
        if (!manifest.files.some(row => row.kind === 'fixture' && row.path === relative)) return unsupported(`Unknown fixture: ${relative}`);
        return file;
      } }, 'fixture helper', unsupported);
      const normalized = id.replace(/^node:/, '');
      if (normalized === 'events' && selected?.module === 'child_process') return {...modules.events, getEventListeners: infrastructureRequire('events').getEventListeners};
      if (Object.hasOwn(modules, normalized)) return modules[normalized];
      if (normalized === 'worker_threads' && selected?.module === 'process') return strictObject({isMainThread: true}, 'test execution context', unsupported);
      if (normalized === 'test') return strictObject({ test(title, options, fn) {
        if (typeof options === 'function') { fn = options; options = {}; }
        options ||= {};
        const result = {name: title, status: 'pending'}; subtests.push(result);
        if (options.skip) {
          const platformTitle = hostProcess.platform === 'win32' ? 'fileURLToPath with posix path' : 'fileURLToPath with windows path';
          if (name === 'test/parallel/test-url-fileurltopath.js' && title === platformTitle) { result.status = 'platform-excluded'; return Promise.resolve(); }
          return unsupported(`Subtest requested skip: ${title}`);
        }
        if (Object.keys(options).some(key => key !== 'skip')) return unsupported('Unknown node:test options');
        const pending = Promise.resolve().then(() => fn(strictObject({}, 'node:test context', unsupported))).then(() => {result.status = 'pass';}, error => {result.status = 'fail';result.reason = error.stack;problem ||= error;});
        checks.push(() => assert.notEqual(result.status, 'pending', `Unfinished subtest: ${title}`));
        return pending;
      } }, 'node:test helper', unsupported);
      if (normalized === 'fs' && selected?.module === 'util') return strictObject({stat: infrastructureRequire('fs').stat, statSync: infrastructureRequire('fs').statSync}, 'util filesystem fixture', unsupported);
      if (normalized === 'internal/util' && selected?.module === 'util') return strictObject({customPromisifyArgs: globalThis[Symbol.for('niva.node-compat.runtime')].util?.customPromisifyArgs || Symbol.for('nodejs.util.promisify.customArgs')}, 'util introspection helper', unsupported);
      if (normalized === 'internal/errors') {
        return strictObject({ codes: strictObject({ ERR_OUT_OF_RANGE: infrastructureRequire('internal/errors').codes.ERR_OUT_OF_RANGE }, 'expected-error constructor', unsupported) }, 'internal/errors helper', unsupported);
      }
      if (normalized === 'internal/event_target') {
        return strictObject({ kEvents: infrastructureRequire('internal/event_target').kEvents }, 'EventTarget introspection helper', unsupported);
      }
      if (normalized === 'internal/test/binding') {
        return strictObject({ internalBinding(binding) {
          if (binding !== 'buffer') return unsupported(`Unknown internal binding: ${binding}`);
          const implementation = globalThis[Symbol.for('niva.node-compat.runtime')].bufferBinding;
          if (!implementation) return unsupported('Niva buffer binding primitive is unavailable');
          return strictObject(implementation, 'Niva buffer binding', unsupported);
        } }, 'binding helper', unsupported);
      }
      if (normalized === 'child_process') {
        return strictObject({ execFile(program, argv, callback) {
          assert.equal(selected?.module, 'util', 'Only util subprocess test fixtures use host execFile');
          assert.equal(program, hostProcess.execPath);
          assert.equal(argv.length, 1);
          const fixture = path.resolve(argv[0]);
          assert(/^callbackify[12]\.js$/.test(path.basename(fixture)));
          const { manifest } = verifySnapshot();
          assert(manifest.files.some(row => row.kind === 'fixture' && path.resolve(snapshot, row.path) === fixture));
          return hostExecFile(program, [path.join(root, 'scripts/upstream-util-fixture.mjs'), fixture], callback);
        }, spawnSync(program, argv, options) {
          assert.equal(program, hostProcess.execPath, 'Only the pinned host executable may run a fixture');
          assert(Array.isArray(argv) && argv.length >= 1);
          const fixture = path.resolve(argv[0]);
          const { manifest } = verifySnapshot();
          assert(manifest.files.some(row => row.kind === 'fixture' && path.resolve(snapshot, row.path) === fixture), 'Only pinned fixtures may spawn');
          return spawnSync(program, ['--expose-internals', script, '--fixture', fixture, ...argv.slice(1)], options);
        } }, 'fixture subprocess helper', unsupported);
      }
      // Explicit test-only infrastructure. No generic createRequire fallback.
      if (normalized === 'assert') return assert;
      if (normalized === 'assert/strict') return assert.strict;
      if (normalized === 'util') return hostUtil;
      if (normalized === 'vm') return vm;
      return unsupported(`Module is outside this harness: ${id}`);
    };
    if (synthetic) {
      const probes = {
        exclusionPolicy() {
          for (const row of environmentPolicy.entries) {
            const original = readFileSync(path.join(snapshot, row.file), 'utf8');
            const prepared = prepareEnvironmentExclusions(row.file, original);
            assert.equal(prepared.entries.length, 1);
            assert.equal(prepared.source.slice(0,row.start), original.slice(0,row.start));
            assert.equal(prepared.source.slice(row.end), original.slice(row.end));
            let reached = 0;
            new Function('__nivaSkip',prepared.source.slice(row.start,row.end))(() => reached++);
            assert.equal(reached, 1);
            assert.equal(prepareEnvironmentExclusions(row.file, original, false).source, original);
            assert.throws(() => prepareEnvironmentExclusions(row.file, original+' '));
          }
        },
        largeResult() { const error = new Error('x'.repeat(20000)); problem = error; },
        subtestFailure() { requireNiva('node:test').test('failure', () => assert.fail('subtest')); },
        subtestPending() { requireNiva('node:test').test('pending', () => new Promise(() => {})); },
        subtestSkip() { requireNiva('node:test').test('skip', {skip:true}, () => {}); },
        subtestSuccess() { requireNiva('node:test').test('success', async () => assert.equal(await Promise.resolve(2), 2)); },
        fixtureRouting() {
          const fixture = path.join(snapshot, 'test/fixtures/path-resolve.js');
          const result = requireNiva('child_process').spawnSync(hostProcess.execPath, [fixture, '.']);
          assert.equal(result.status, 0, String(result.stderr));
          assert.equal(result.stdout.toString().trim(), modules.path.resolve('.'));
        },
        internalRouting() {
          assert.strictEqual(requireNiva('internal/event_target').kEvents, infrastructureRequire('internal/event_target').kEvents);
          const range = new (requireNiva('internal/errors').codes.ERR_OUT_OF_RANGE)('offset', '>= 0', -1);
          assert.equal(range.code, 'ERR_OUT_OF_RANGE');
          assert.throws(() => requireNiva('internal/test/binding').internalBinding('fs'), Unsupported);
        },
        unknownInternal() { try { requireNiva('internal/errors').codes.UNKNOWN_ERROR; } catch {} },
        unknownIntrospection() { try { requireNiva('internal/event_target').unknown; } catch {} },
        nativeRelayContract() { verifyNativeRelayContract(); },
        fixtureEscape() { requireNiva('child_process').spawnSync(hostProcess.execPath, [script]); },
        routing() { assert.strictEqual(requireNiva('path'), modules.path); assert.strictEqual(requireNiva('node:path'), modules.path); assert.strictEqual(Buffer, modules.buffer.Buffer); },
        assertion() { assert.fail('deliberate assertion failure'); },
        missing() { common.mustCall(() => {}); },
        extra() { const fn = common.mustCall(() => {}); fn(); fn(); },
        forbidden() { common.mustNotCall()(); },
        async() { setTimeout(() => assert.fail('deliberate asynchronous failure'), 5); },
        rejection() { Promise.reject(new Error('deliberate unhandled rejection')); },
        unknown() { requireNiva('node:fs'); },
        helper() { common.notImplemented(); },
        swallowedHelper() { try { common.notImplemented(); } catch {} },
        skip() { common.skip('deliberate skip'); },
        swallowed() { try { requireNiva('node:fs'); } catch {} },
        timeout() { setInterval(() => {}, 1000); },
        earlyExit() { hostProcess.exit(0); },
        counted() { setTimeout(common.mustCall(() => {}), 5); },
        lateExtra() { const fn = common.mustCall(() => {}); fn(); hostProcess.once('beforeExit', () => setImmediate(fn)); },
      };
      assert(Object.hasOwn(probes, name));
      probes[name]();
    } else {
      const { tests } = verifySnapshot();
      assert(tests.some(test => test.path === name), `Unselected test: ${name}`);
      const file = path.join(snapshot, name);
      const source = readFileSync(file, 'utf8');
      globalThis[Symbol.for("niva.node-compat.runtime")].source?.registerSource(file, source);
      const prepared = prepareEnvironmentExclusions(name, source, !includeEnvironmentSpecific);
      executedSourceSha256 = prepared.executedSourceSha256;
      const exclusionCalls = prepared.entries.map(() => 0);
      checks.push(() => exclusionCalls.forEach(count => assert.equal(count, 1, 'Approved exclusion site must be reached exactly once')));
      const recordExclusion = index => {
        assert(Number.isInteger(index) && index >= 0 && index < prepared.entries.length, 'Unknown environment exclusion');
        assert.equal(++exclusionCalls[index], 1, 'Repeated environment exclusion');
        const {id,file,startLine,endLine,reason} = prepared.entries[index];
        environmentExclusions.push({id,file,startLine,endLine,reason});
      };
      // Only the two approved statement sites become recorded no-ops. Every
      // other original character, line number, and assertion is preserved.
      const execute = vm.runInThisContext(`(function(process, setTimeout, clearTimeout, setInterval, clearInterval, setImmediate, clearImmediate, __nivaSkip) { return function(require, module, exports, __filename, __dirname) {${prepared.source}\n}; })`, { filename: file });
      const module = { exports: {} };
      const timerHost = selected.module === 'timers' ? modules.timers : globalThis;
      execute(selected.module === 'process' ? modules.process : hostProcess, ...['setTimeout','clearTimeout','setInterval','clearInterval','setImmediate','clearImmediate'].map(key => timerHost[key]), recordExclusion)(requireNiva, module, module.exports, file, path.dirname(file));
    }
  } catch (error) { problem ||= error; setImmediate(() => hostProcess.exit(1)); }
}

if (args[0] === '--fixture') {
  const { manifest } = verifySnapshot();
  const file = path.resolve(args[1]);
  assert(manifest.files.some(row => row.kind === 'fixture' && path.resolve(snapshot, row.path) === file));
  await installRuntimeFixture();
  const nivaPath = (await import(pathToFileURL(path.join(root, 'dist/source/path.js')).href)).default;
  assert.notStrictEqual(nivaPath, path, 'Fixture must not resolve to host Node path');
  hostProcess.argv = [hostProcess.execPath, file, ...args.slice(2)];
  const requireFixture = id => {
    if (id === 'path' || id === 'node:path') return nivaPath;
    throw new Unsupported(`Unknown fixture module: ${id}`);
  };
  vm.runInThisContext(`(function(require) {\n${readFileSync(file, 'utf8')}\n})`, { filename: file })(requireFixture);
} else if (args[0] === '--case' || args[0] === '--probe') {
  await runCase(args[1], args[0] === '--probe');
} else if (args[0] === '--self-test') {
  verifySnapshot();
  const expected = { exclusionPolicy: 'pass', largeResult: 'fail', subtestFailure:'fail', subtestPending:'fail', subtestSkip:'unsupported', subtestSuccess:'pass', routing: 'pass', fixtureRouting: 'pass', internalRouting: 'unsupported', unknownInternal: 'unsupported', unknownIntrospection: 'unsupported', nativeRelayContract: 'pass', fixtureEscape: 'fail', counted: 'pass', assertion: 'fail', missing: 'fail',
    extra: 'fail', forbidden: 'fail', async: 'fail', rejection: 'fail', unknown: 'unsupported',
    helper: 'unsupported', swallowedHelper: 'unsupported', skip: 'unsupported', swallowed: 'unsupported', timeout: 'fail', earlyExit: 'fail', lateExtra: 'fail' };
  for (const [probe, status] of Object.entries(expected)) {
    const result = child(probe, true);
    assert.equal(result.status, status, `${probe}: ${JSON.stringify(result)}`);
    if (probe === 'largeResult') assert(result.reason.includes('x'.repeat(20000)), 'Large result was truncated at process exit');
    console.log(`PASS harness ${probe}`);
  }
  console.log(`Harness self-test: ${Object.keys(expected).length} passed`);
} else {
  let reportPath, suite = 'all';
  for (let i = 0; i < args.length; i += 2) {
    assert(args[i + 1], 'Missing option value');
    if (args[i] === '--report') reportPath = path.resolve(args[i + 1]);
    else if (args[i] === '--suite') suite = args[i + 1];
    else assert.fail('Unknown option: ' + args[i]);
  }
  assert(['all','js','native'].includes(suite), 'Invalid suite');
  const report = { schemaVersion: 1, startedAt: new Date().toISOString(),
    runnerSha256: sha256(readFileSync(script)),
    helperSha256: Object.fromEntries(['upstream-native-bridge.mjs','upstream-native-relay.py','upstream-util-fixture.mjs','upstream-child-fixtures.cjs','upstream-environment-exclusions.mjs'].map(name => [name, sha256(readFileSync(path.join(root,'scripts',name)))])),
    environment: { node: hostProcess.version, platform: hostProcess.platform, arch: hostProcess.arch },
    scope: includeEnvironmentSpecific ? 'Full unfiltered upstream files against Niva adapters and real Native relay' : 'Pinned upstream files with exactly two user-approved environment-specific statement exclusions; every other assertion executes against Niva adapters and real Native relay',
    environmentPolicy: {mode: includeEnvironmentSpecific ? 'unfiltered' : 'approved-exclusions', sha256: policySha256},
    infrastructure: ['host assert oracle except assert-subject tests use Niva assert', 'host util diagnostics except util-subject tests use Niva util', 'host vm for test realms', 'host process/scheduling and Web Crypto platform primitive', 'host memory capacity for upstream enoughTestMem', 'live host cwd/env as explicit OS fixtures', 'host internal expected-error constructor and EventTarget introspection symbol', 'pinned fixture subprocess with Niva path or util routing', 'host fs stat/statSync only as util.promisify fixture', 'native suite: isolated real WebView using Niva.bridge.call/callSync/stream/streamSend over ordinary main-window process stdin/stdout; no host filesystem/network API replacement', 'child_process test uses host process.execPath to launch pinned Node as an external program through real Native', 'host EventTarget listener introspection for child_process abort checks'],
    results: [] };
  try {
    const { manifest, tests: frozenTests } = verifySnapshot();
    const tests = frozenTests.filter(test => suite === 'all' || (test.execution === 'native' ? 'native' : 'js') === suite);
    report.suite = suite;
    assert(tests.length > 0, 'Empty suite is not a passing gate');
    report.frozenTotal = frozenTests.length;
    report.outsideSuite = frozenTests.filter(test => !tests.includes(test)).map(test => test.path);
    if (hostProcess.env.NIVA_UPSTREAM_BINARY) report.native = {binary: path.resolve(hostProcess.env.NIVA_UPSTREAM_BINARY), sha256: sha256(readFileSync(hostProcess.env.NIVA_UPSTREAM_BINARY))};
    report.upstream = manifest.upstream;
    report.manifestSha256 = sha256(readFileSync(path.join(upstream, 'manifest.json')));
    report.sourceSha256 = sourceFingerprint();
    for (const test of tests) {
      const result = { ...child(test.path), module: test.module, execution: test.execution || 'js-host' };
      report.results.push(result);
      console.log(`${result.status.toUpperCase()} ${test.path}${result.status === 'pass' ? '' : `: ${result.reason?.split('\n')[0]}`}`);
    }
    assert.deepStrictEqual(sourceFingerprint(), report.sourceSha256, 'Niva source changed while suite ran; rerun on a stable snapshot');
    verifySnapshot();
    report.counts = { total: tests.length, pass: 0, fail: 0, unsupported: 0 };
    for (const result of report.results) report.counts[result.status]++;
    assert.equal(report.results.length, tests.length, 'Missing test results');
    report.environmentExclusions = report.results.flatMap(result => result.environmentExclusions || []);
    report.environmentExclusionCount = report.environmentExclusions.length;
    const expectedExclusions = includeEnvironmentSpecific ? 0 : environmentPolicy.entries.filter(row => tests.some(test => test.path === row.file)).length;
    assert.equal(report.environmentExclusionCount, expectedExclusions, 'Missing or extra approved environment exclusions');
    report.passed = report.counts.pass === tests.length;
  } catch (error) { report.passed = false; report.error = error.stack; }
  report.finishedAt = new Date().toISOString();
  if (reportPath) { mkdirSync(path.dirname(reportPath), { recursive: true }); writeFileSync(reportPath, JSON.stringify(report, null, 2) + '\n'); }
  console.log(JSON.stringify({ passed: report.passed, counts: report.counts, environmentExclusionCount: report.environmentExclusionCount, error: report.error }));
  if (!report.passed) hostProcess.exitCode = 1;
}

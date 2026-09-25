import "./setup-runtime.js";
import test from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import nodeHttp from 'node:http';
import '../dist/source/runtime/bridge.js';
import '../dist/source/runtime/vendor.js';
import '../dist/source/runtime/events.js';
import '../dist/source/runtime/http.js';

const runtime = globalThis[Symbol.for('niva.node-compat.runtime')];
let connected = 0;
let nextStreamId = 1;
const streams = [];
runtime.createNetModule = () => ({
  connect(options) { connected++; return net.connect(options); },
  createServer: (...args) => net.createServer(...args),
});

function nodeHeaders(raw) {
  const headers = {};
  for (const [name, value] of raw) {
    if (headers[name] === undefined) headers[name] = value;
    else headers[name] = Array.isArray(headers[name]) ? [...headers[name], value] : [headers[name], value];
  }
  return headers;
}

const niva = { bridge: {
  stream(method, args, handlers) {
    assert.equal(method, 'http.requestStream');
    const options = args[0], id = nextStreamId++;
    let resolve, reject;
    const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
    const state = { id, options, handlers, response: null, responseStarted: false, settled: false, uploads: [], responseAcks: 0 };
    const request = nodeHttp.request(new URL(options.url), { method: options.method, headers: nodeHeaders(options.headers) });
    state.request = request;
    request.on('response', response => {
      state.response = response;
      state.responseStarted = true;
      handlers.onEvent('response', {
        statusCode: response.statusCode,
        statusMessage: response.statusMessage,
        headers: response.headers,
        rawHeaders: response.rawHeaders,
        httpVersionMajor: 1,
        httpVersionMinor: 1,
      });
      response.on('data', chunk => {
        response.pause();
        handlers.onChunk(Uint8Array.from(chunk));
      });
      response.on('end', () => { state.settled = true; resolve(); });
    });
    request.on('error', error => { state.settled = true; reject(error); });
    const call = {
      id,
      promise,
      cancel() {
        request.destroy();
        if (!state.settled) {
          state.settled = true;
          reject(Object.assign(new Error('cancelled'), { code: 'ABORT_ERR' }));
        }
        return true;
      },
    };
    state.call = call;
    streams.push(state);
    return call;
  },
  streamSend(id, data, end) {
    const state = streams.find(candidate => candidate.id === id);
    if (!state) return false;
    const bytes = Buffer.from(data);
    if (state.responseStarted && !end && bytes.length === 0) {
      state.responseAcks++;
      state.response.resume();
      return true;
    }
    if (end) {
      state.request.end(bytes);
      return true;
    }
    state.uploads.push(bytes.length);
    state.request.write(bytes, () => state.handlers.onEvent('uploadAck', {}));
    return true;
  },
} };

const http = runtime.createHttpModule('http', niva);
const listen = server => new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const close = server => new Promise(resolve => server.close(resolve));
const collect = response => new Promise((resolve, reject) => {
  const chunks = [];
  response.on('data', chunk => chunks.push(chunk));
  response.on('end', () => resolve(Buffer.concat(chunks)));
  response.on('error', reject);
});

test('HTTP client streams a chunked request and response through the Native bridge contract', async () => {
  let received;
  const server = nodeHttp.createServer(async (req, res) => {
    received = await collect(req);
    res.setHeader('X-Protocol', 'native-peer');
    res.write('hel');
    res.end('lo');
  });
  await listen(server);
  try {
    const result = await new Promise((resolve, reject) => {
      const req = http.request(`http://127.0.0.1:${server.address().port}/test`, { method: 'POST' }, async response => {
        try { resolve({ body: (await collect(response)).toString(), header: response.headers['x-protocol'] }); }
        catch (error) { reject(error); }
      });
      req.on('error', reject);
      req.write('a');
      req.end('bc');
    });
    assert.deepEqual(result, { body: 'hello', header: 'native-peer' });
    assert.equal(received.toString(), 'abc');
    assert.equal(connected, 0);
    assert.equal(streams.at(-1).options.method, 'POST');
    assert.deepEqual(streams.at(-1).options.headers.map(([name]) => name), ['Host', 'Connection']);
    assert.ok(streams.at(-1).options.url.endsWith('/test'));
    assert.ok(streams.at(-1).responseAcks > 0);
  } finally { await close(server); }
});

test('HTTP client serializes repeated outbound headers as Native header pairs', async () => {
  let receivedRawHeaders;
  const server = nodeHttp.createServer((req, res) => {
    receivedRawHeaders = req.rawHeaders;
    res.end('ok');
  });
  await listen(server);
  try {
    await new Promise((resolve, reject) => {
      const req = http.request(`http://127.0.0.1:${server.address().port}/repeated`, {
        headers: { 'X-Repeat': ['one', 'two'] },
      }, async response => {
        try { assert.equal((await collect(response)).toString(), 'ok'); resolve(); }
        catch (error) { reject(error); }
      });
      req.on('error', reject);
      req.end();
    });
    assert.deepEqual(streams.at(-1).options.headers.slice(0, 2), [['X-Repeat', 'one'], ['X-Repeat', 'two']]);
    assert.deepEqual(streams.at(-1).options.headers.at(-1), ['Connection', 'close']);
    assert.deepEqual(receivedRawHeaders.filter((_, index) => index % 2 === 0 && receivedRawHeaders[index].toLowerCase() === 'x-repeat').length, 2);
  } finally { await close(server); }
});

test('large HTTP request writes are split and acknowledged before the next piece', async () => {
  let received;
  const server = nodeHttp.createServer(async (req, res) => {
    received = await collect(req);
    res.end('ok');
  });
  await listen(server);
  try {
    const payload = Buffer.alloc(40_000, 0x61);
    const result = await new Promise((resolve, reject) => {
      const req = http.request(`http://127.0.0.1:${server.address().port}/large`, {
        method: 'POST', headers: { 'Content-Length': payload.length },
      }, async response => {
        try { resolve((await collect(response)).toString()); }
        catch (error) { reject(error); }
      });
      req.on('error', reject);
      req.end(payload);
    });
    const sent = streams.at(-1).uploads;
    assert.equal(result, 'ok');
    assert.deepEqual(received, payload);
    assert.deepEqual(sent, [16 * 1024, 16 * 1024, 40_000 - 2 * 16 * 1024]);
  } finally { await close(server); }
});

test('JS HTTP server handles Node HTTP request, binary body, and response trailers', async () => {
  const server = http.createServer(async (req, res) => {
    assert.equal(req.url, '/echo');
    assert.equal(req.method, 'POST');
    const body = await collect(req);
    res.setHeader('X-Test', 'server');
    res.addTrailers({ 'X-Trailer': 'complete' });
    res.end(body);
  });
  await listen(server);
  try {
    const result = await new Promise((resolve, reject) => {
      const req = nodeHttp.request({ host: '127.0.0.1', port: server.address().port, path: '/echo', method: 'POST' }, async response => {
        try { resolve({ body: (await collect(response)).toString(), trailers: response.trailers }); }
        catch (error) { reject(error); }
      });
      req.on('error', reject);
      req.end('hello');
    });
    assert.deepEqual(result, { body: 'hello', trailers: { 'x-trailer': 'complete' } });
  } finally { await close(server); }
});

test('JS HTTP server rejects duplicate length and TE/CL before dispatch', async () => {
  let requests = 0, errors = 0;
  const server = http.createServer(() => requests++);
  server.on('clientError', (_error, socket) => { errors++; socket.destroy(); });
  await listen(server);
  try {
    for (const header of ['Content-Length: 1\r\nContent-Length: 1', 'Transfer-Encoding: chunked\r\nContent-Length: 1']) {
      await new Promise((resolve, reject) => {
        const socket = net.connect(server.address().port, '127.0.0.1', () => socket.end('POST / HTTP/1.1\r\nHost: local\r\n' + header + '\r\n\r\n0\r\n\r\n'));
        socket.on('close', resolve);
        socket.on('error', reject);
      });
    }
    assert.equal(requests, 0);
    assert.equal(errors, 2);
  } finally { await close(server); }
});

test('invalid outbound HTTP input fails before allocating a stream', () => {
  const beforeConnections = connected, beforeStreams = streams.length;
  assert.throws(() => http.request('http://127.0.0.1/', { method: 'GET\r\nInjected' }));
  assert.throws(() => http.request('http://127.0.0.1/', { headers: { 'X-Test': 'bad\r\nHeader: value' } }));
  assert.throws(() => http.request('http://@127.0.0.1/'));
  assert.throws(() => http.request('http://127.0.0.1/', { protocol: 'https:' }));
  assert.equal(connected, beforeConnections);
  assert.equal(streams.length, beforeStreams);
});

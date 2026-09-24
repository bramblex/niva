import test from "node:test";
import assert from "node:assert/strict";
import { once as onceEvent } from "node:events";
import { Buffer } from "../src/buffer.js";
import net from "../src/net.js";
import tls from "../src/tls.js";
import dgram from "../src/dgram.js";
import dns from "../src/dns.js";
import dnsPromises from "../src/dns-promises.js";

function mockNiva() {
  const calls = [];
  const sent = [];
  const unary = [];
  const activeDgramBinds = new Map();
  let nextId = 1;
  const niva = {
    calls,
    sent,
    unary,
    stream(method, args, handlers) {
      var resolve, reject;
      const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
      const call = {
        id: nextId++, method, args, handlers, promise,
        resolve, reject, cancelled: false,
        cancel() { call.cancelled = true; },
      };
      calls.push(call);
      if (method === "socket.udpBind") {
        const onEvent = handlers.onEvent;
        handlers.onEvent = (name, data) => {
          if (name === "listening") activeDgramBinds.set(data.socketId, call);
          if (name === "close") activeDgramBinds.delete(data.socketId);
          if (onEvent) onEvent(name, data);
        };
      }
      if (method === "socket.control") {
        const controlArgs = args[0];
        resolve({ ok: true, socketId: controlArgs.socketId, action: controlArgs.action });
        if (controlArgs.action === "close") {
          const bindCall = activeDgramBinds.get(controlArgs.socketId);
          if (bindCall) queueMicrotask(() => {
            bindCall.handlers.onEvent("close", { socketId: controlArgs.socketId, hadError: false });
            bindCall.resolve({ closed: true });
          });
        }
      }
      return call;
    },
    streamSend(id, data, end) {
      sent.push({ id, data: Buffer.from(data), end });
      return true;
    },
    call(method, args) {
      unary.push({ method, args });
      const response = niva.nextDnsResponse;
      niva.nextDnsResponse = undefined;
      return response instanceof Error ? Promise.reject(response) : Promise.resolve(response);
    },
  };
  return niva;
}

function metadata(id = "socket-1") {
  return { socketId: id, localAddress: "127.0.0.1", localPort: 40000, remoteAddress: "192.0.2.1", remotePort: 8080 };
}

test("net.Socket streams writes, acks consumed reads, and half-closes with native frames", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createNetModule(niva);
  const socket = api.connect({ host: "example.test", port: 8080 });
  const connectCall = niva.calls[0];
  assert.equal(connectCall.method, "socket.tcpConnect");
  assert.deepEqual(connectCall.args, [{ host: "example.test", port: 8080 }]);
  const connected = onceEvent(socket, "connect");
  connectCall.handlers.onEvent("connect", metadata());
  await connected;
  assert.equal(socket.remotePort, 8080);

  const writeDone = new Promise((resolve, reject) => socket.write(Buffer.from("request"), (error) => error ? reject(error) : resolve()));
  assert.equal(niva.sent[0].id, connectCall.id);
  assert.equal(niva.sent[0].data.toString(), "request");
  assert.equal(niva.sent[0].end, false);
  connectCall.handlers.onEvent("writeAck", { seq: 1, bytes: 7 });
  await writeDone;
  assert.equal(socket.bytesWritten, 7);

  const data = onceEvent(socket, "data");
  connectCall.handlers.onChunk(Uint8Array.of(1, 2, 3), false);
  assert.deepEqual([...(await data)[0]], [1, 2, 3]);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.ok(niva.calls.some((call) => call.method === "socket.control" && call.args[0].action === "readAck" && call.args[0].bytes === 3), JSON.stringify(niva.calls.map((call) => [call.method, call.args[0] && call.args[0].action, call.args[0] && call.args[0].bytes])));
  assert.equal(socket.bytesRead, 3);

  const finished = onceEvent(socket, "finish");
  socket.end();
  assert.equal(niva.sent[1].data.length, 0);
  assert.equal(niva.sent[1].end, true);
  connectCall.handlers.onEvent("writeAck", { seq: 2, bytes: 0 });
  await finished;
  connectCall.handlers.onEvent("close", { socketId: "socket-1", hadError: false });
  connectCall.resolve({ closed: true });
});

test("net.Socket peer FIN ends readable side and respects allowHalfOpen", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createNetModule(niva);
  const optionProbe = new api.Socket();
  assert.throws(() => optionProbe.setNoDelay(), { code: "ENOTSUP" });
  assert.throws(() => optionProbe.setKeepAlive(true), { code: "ENOTSUP" });

  const socket = api.connect({ host: "example.test", port: 8080 });
  const connectCall = niva.calls[0];
  const connected = onceEvent(socket, "connect");
  connectCall.handlers.onEvent("connect", metadata());
  await connected;

  const readableEnd = onceEvent(socket, "end");
  socket.resume();
  connectCall.handlers.onEvent("end", { socketId: "socket-1" });
  await readableEnd;
  assert.equal(niva.sent.length, 1);
  assert.equal(niva.sent[0].end, true, "default allowHalfOpen:false sends FIN after peer FIN");
  const finished = onceEvent(socket, "finish");
  connectCall.handlers.onEvent("writeAck", { seq: 1, bytes: 0 });
  await finished;
  connectCall.handlers.onEvent("close", { socketId: "socket-1", hadError: false });

  const halfOpen = api.connect({ host: "example.test", port: 8081, allowHalfOpen: true });
  const halfOpenCall = niva.calls.find((call) => call.method === "socket.tcpConnect" && call !== connectCall);
  const halfOpenConnected = onceEvent(halfOpen, "connect");
  halfOpenCall.handlers.onEvent("connect", metadata("socket-2"));
  await halfOpenConnected;
  const halfOpenEnd = onceEvent(halfOpen, "end");
  halfOpen.resume();
  halfOpenCall.handlers.onEvent("end", { socketId: "socket-2" });
  await halfOpenEnd;
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(niva.sent.filter((frame) => frame.id === halfOpenCall.id && frame.end).length, 0,
    "allowHalfOpen:true leaves the write side open after peer FIN");

  const halfOpenFinished = onceEvent(halfOpen, "finish");
  halfOpen.end();
  assert.equal(niva.sent.at(-1).end, true);
  halfOpenCall.handlers.onEvent("writeAck", { seq: 1, bytes: 0 });
  await halfOpenFinished;
  halfOpenCall.handlers.onEvent("close", { socketId: "socket-2", hadError: false });
});

test("net.Socket timeout resets on acknowledged writes and received data", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createNetModule(niva);
  const socket = api.connect({ host: "example.test", port: 8080, timeout: 100 });
  const connectCall = niva.calls[0];
  const timedOut = onceEvent(socket, "timeout");
  socket.resume();
  const connected = onceEvent(socket, "connect");
  connectCall.handlers.onEvent("connect", metadata());
  await connected;

  await new Promise((resolve) => setTimeout(resolve, 45));
  const writeDone = new Promise((resolve, reject) => socket.write("ping", (error) => error ? reject(error) : resolve()));
  connectCall.handlers.onEvent("writeAck", { seq: 1, bytes: 4 });
  await writeDone;
  await new Promise((resolve) => setTimeout(resolve, 70));
  assert.equal(socket._timeoutId !== undefined, true, "write progress restarts the idle timer");

  connectCall.handlers.onChunk(Uint8Array.of(1), false);
  await new Promise((resolve) => setTimeout(resolve, 70));
  assert.equal(socket._timeoutId !== undefined, true, "received data restarts the idle timer");
  await timedOut;
  assert.equal(socket._timeoutId, undefined);
  socket.destroy();
});

test("net.Server attaches accepted handles on the same bridge connection and waits for close", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createNetModule(niva);
  const server = api.createServer({ allowHalfOpen: true });
  const accepted = onceEvent(server, "connection");
  const listening = onceEvent(server, "listening");
  server.listen(0, "127.0.0.1");
  const listenerCall = niva.calls[0];
  assert.equal(listenerCall.method, "socket.tcpListen");
  listenerCall.handlers.onEvent("listening", { socketId: "listener-1", address: "127.0.0.1", port: 45123 });
  await listening;
  assert.deepEqual(server.address(), { address: "127.0.0.1", family: "IPv4", port: 45123 });

  listenerCall.handlers.onEvent("connection", { socketId: "pending-1", address: "192.0.2.8", port: 53100 });
  const attachCall = niva.calls.find((call) => call.method === "socket.tcpAttach");
  assert.ok(attachCall);
  assert.deepEqual(attachCall.args, [{ socketId: "pending-1" }]);
  attachCall.handlers.onEvent("connect", Object.assign(metadata("active-1"), { remoteAddress: "192.0.2.8", remotePort: 53100 }));
  const [socket] = await accepted;
  assert.equal(socket.remotePort, 53100);
  assert.equal(socket.allowHalfOpen, true);

  const closed = onceEvent(server, "close");
  server.close();
  attachCall.handlers.onEvent("close", { socketId: "active-1", hadError: false });
  attachCall.resolve({ closed: true });
  await closed;
  assert.equal(listenerCall.cancelled, true);
});

test("TLS requires verified Native transport and maps CA/server identities", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createTlsModule(niva);
  assert.throws(() => api.connect({ host: "example.test", port: 443, rejectUnauthorized: false }), { code: "ENOTSUP" });
  assert.throws(() => api.connect({ host: "example.test", port: 443, ALPNProtocols: ["h2"] }), { code: "ENOTSUP" });
  assert.throws(() => api.connect({ host: "example.test", port: 443, allowHalfOpen: true }), { code: "ENOTSUP" });
  assert.throws(() => api.connect({ host: "example.test", servername: "other.test", port: 443 }), { code: "ENOTSUP" });
  assert.throws(() => api.connect({ hostname: "example.test", servername: "other.test", port: 443 }), { code: "ENOTSUP" });
  assert.throws(() => api.connect({ servername: "other.test", port: 443 }), { code: "ENOTSUP" });
  assert.throws(() => api.connect({ host: "example.test", port: 443, checkServerIdentity() {} }), { code: "ENOTSUP" });
  for (const option of ["minVersion", "maxVersion", "ciphers", "secureContext", "session", "sigalgs", "ecdhCurve", "secureProtocol", "secureOptions"]) {
    assert.throws(() => api.connect({ host: "example.test", port: 443, [option]: "explicit" }), { code: "ENOTSUP" });
    assert.throws(() => api.connectGuarded({ host: "example.test", port: 443, [option]: "explicit" }), { code: "ENOTSUP" });
  }
  assert.throws(() => api.connectGuarded({ host: "example.test", servername: "other.test", port: 443 }), { code: "ENOTSUP" });
  assert.throws(() => api.connectGuarded({ hostname: "example.test", servername: "other.test", port: 443 }), { code: "ENOTSUP" });
  assert.throws(() => api.connectGuarded({ servername: "other.test", port: 443 }), { code: "ENOTSUP" });
  assert.throws(() => api.connectGuarded({ host: "example.test", port: 443, checkServerIdentity() {} }), { code: "ENOTSUP" });
  assert.equal(niva.calls.length, 0, "unsupported verification options must fail before opening a Native stream");
  assert.throws(() => api.createServer({ key: "pem-key", cert: "pem-cert", allowHalfOpen: true }), { code: "ENOTSUP" });
  assert.throws(() => new api.TLSSocket({ secure: true, allowHalfOpen: true }), { code: "ENOTSUP" });
  const secure = onceEvent(api.connect({ host: "example.test", port: 443, ca: ["pem-ca"] }), "secureConnect");
  const call = niva.calls[0];
  assert.equal(call.method, "socket.tlsConnect");
  assert.deepEqual(call.args, [{ host: "example.test", port: 443, ca: ["pem-ca"] }]);
  call.handlers.onEvent("secureConnect", metadata("tls-1"));
  await secure;

  const server = api.createServer({ key: "pem-key", cert: "pem-cert" });
  const listening = onceEvent(server, "listening");
  server.listen(0);
  const listenCall = niva.calls.find((item) => item.method === "socket.tlsListen");
  assert.deepEqual(listenCall.args, [{ host: "127.0.0.1", port: 0, identity: { key: "pem-key", cert: "pem-cert" } }]);
  listenCall.handlers.onEvent("listening", { socketId: "tls-listener", address: "127.0.0.1", port: 45555 });
  await listening;
  server.close();
  await onceEvent(server, "close");
});

test("TLS server copies DataView credentials into the Native identity", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createTlsModule(niva);
  const keyBytes = Buffer.from("xPRIVATEy");
  const certBytes = Buffer.from("xCERTy");
  const key = new DataView(keyBytes.buffer, keyBytes.byteOffset + 1, 7);
  const cert = new DataView(certBytes.buffer, certBytes.byteOffset + 1, 4);
  const server = api.createServer({ key, cert, ca: false });
  const listening = onceEvent(server, "listening");
  server.listen(0);
  const call = niva.calls.find((item) => item.method === "socket.tlsListen");
  assert.deepEqual(call.args, [{ host: "127.0.0.1", port: 0, identity: { key: "PRIVATE", cert: "CERT" } }]);
  call.handlers.onEvent("listening", { socketId: "tls-data-view", address: "127.0.0.1", port: 45556 });
  await listening;
  const closed = onceEvent(server, "close");
  server.close();
  await closed;
});

test("TLS server unwraps one credential and rejects multiple Native identities at listen", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createTlsModule(niva);
  const unsupported = api.createServer({ key: ["key-one", "key-two"], cert: ["cert-one", "cert-two"] });
  assert.throws(() => unsupported.listen(0), { code: "ENOTSUP" });
  assert.equal(niva.calls.length, 0);

  const server = api.createServer({ key: ["single-key"], cert: ["single-cert"] });
  const listening = onceEvent(server, "listening");
  server.listen(0);
  const call = niva.calls.find((item) => item.method === "socket.tlsListen");
  assert.deepEqual(call.args, [{ host: "127.0.0.1", port: 0, identity: { key: "single-key", cert: "single-cert" } }]);
  call.handlers.onEvent("listening", { socketId: "tls-single-array", address: "127.0.0.1", port: 45557 });
  await listening;
  const closed = onceEvent(server, "close");
  server.close();
  await closed;
});

test("dgram preserves datagram boundaries and sends one END-terminated payload", async () => {
  const niva = mockNiva();
  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createDgramModule(niva);
  const socket = api.createSocket("udp4");
  const listening = onceEvent(socket, "listening");
  socket.bind(0, "127.0.0.1");
  const bindCall = niva.calls[0];
  assert.equal(bindCall.method, "socket.udpBind");
  bindCall.handlers.onEvent("listening", { socketId: "udp-1", address: "127.0.0.1", port: 49001 });
  await listening;

  const message = onceEvent(socket, "message");
  bindCall.handlers.onEvent("datagram", { socketId: "udp-1", address: "192.0.2.2", port: 53, size: 3 });
  bindCall.handlers.onBlob(new Blob([Uint8Array.of(1, 2, 3)]));
  const [payload, peer] = await message;
  assert.deepEqual([...payload], [1, 2, 3]);
  assert.deepEqual(peer, { address: "192.0.2.2", port: 53, family: "IPv4", size: 3 });
  assert.ok(niva.calls.some((call) => call.method === "socket.control" && call.args[0].action === "readAck" && call.args[0].bytes === 3));

  const sent = new Promise((resolve, reject) => socket.send(Buffer.from("dns"), 53, "192.0.2.53", (error, bytes) => error ? reject(error) : resolve(bytes)));
  const sendCall = niva.calls.find((call) => call.method === "socket.udpSend");
  assert.deepEqual(sendCall.args, [{ socketId: "udp-1", address: "192.0.2.53", port: 53 }]);
  assert.equal(niva.sent[0].data.toString(), "dns");
  assert.equal(niva.sent[0].end, true);
  sendCall.resolve({ sent: 3 });
  assert.equal(await sent, 3);

  const closed = onceEvent(socket, "close");
  socket.close();
  const closeCall = niva.calls.find((call) => call.method === "socket.control" && call.args[0].action === "close");
  assert.deepEqual(closeCall.args, [{ socketId: "udp-1", action: "close" }]);
  assert.equal(bindCall.cancelled, false, "dgram.close leaves the bind stream alive for Native close delivery");
  await closed;
});

test("DNS uses OS lookup separately from validated DNS packet queries over UDP", async () => {
  const niva = mockNiva();
  const dnsPacket = globalThis[Symbol.for("niva.node-compat.runtime")].vendor.dnsPacket;
  const udpBindings = new Map();
  const tcpBuffers = new Map();
  let spoofNextResponse = true;
  let truncateNextResponse = false;
  let tcpFallbackCount = 0;
  const baseStream = niva.stream.bind(niva);
  niva.stream = (method, args, handlers) => {
    const call = baseStream(method, args, handlers);
    if (method === "socket.udpBind") {
      const socketId = "dns-udp-" + call.id;
      udpBindings.set(socketId, call);
      queueMicrotask(() => handlers.onEvent("listening", { socketId, address: "0.0.0.0", port: 50000 }));
    }
    if (method === "socket.tcpConnect") {
      tcpFallbackCount += 1;
      tcpBuffers.set(call.id, []);
      queueMicrotask(() => handlers.onEvent("connect", metadata("dns-tcp-" + call.id)));
    }
    return call;
  };
  function responsePacket(query) {
    const question = query.questions[0];
    const answerData = question.type === "A" ? "203.0.113.10"
      : question.type === "AAAA" ? "2001:db8::10"
        : question.type === "CNAME" ? "alias.example.test"
          : question.type === "MX" ? { preference: 10, exchange: "mail.example.test" }
            : question.type === "TXT" ? ["hello", "world"]
              : question.type === "NS" ? "ns.example.test"
                : question.type === "SRV" ? { priority: 1, weight: 2, port: 443, target: "srv.example.test" }
                  : question.type === "SOA" ? { mname: "ns.example.test", rname: "host.example.test", serial: 1, refresh: 2, retry: 3, expire: 4, minimum: 5 }
                    : "ptr.example.test";
    const answer = { name: question.name, type: question.type, ttl: 60, data: answerData };
    return dnsPacket.encode({
      type: "response", id: query.id,
      flags: dnsPacket.RECURSION_DESIRED | dnsPacket.RECURSION_AVAILABLE | (truncateNextResponse ? dnsPacket.TRUNCATED_RESPONSE : 0),
      questions: query.questions,
      answers: truncateNextResponse ? [] : [answer],
    });
  }
  niva.streamSend = (id, data, end) => {
    const call = niva.calls.find((item) => item.id === id);
    niva.sent.push({ id, data: Buffer.from(data), end });
    if (call && call.method === "socket.udpSend" && end) {
      const query = dnsPacket.decode(data);
      const response = Buffer.from(responsePacket(query));
      const binding = udpBindings.get(call.args[0].socketId);
      queueMicrotask(() => {
        if (spoofNextResponse) {
          spoofNextResponse = false;
          const spoof = Buffer.from(response);
          spoof.writeUInt16BE((query.id + 1) & 0xffff, 0);
          binding.handlers.onEvent("datagram", { socketId: call.args[0].socketId, address: "192.0.2.99", port: 5300, size: spoof.length });
          binding.handlers.onBlob(new Blob([spoof]));
        }
        binding.handlers.onEvent("datagram", { socketId: call.args[0].socketId, address: "127.0.0.1", port: 5300, size: response.length });
        binding.handlers.onBlob(new Blob([response]));
        truncateNextResponse = false;
        call.resolve({ sent: data.length });
      });
    }
    if (call && call.method === "socket.tcpConnect") {
      const parts = tcpBuffers.get(id);
      parts.push(Buffer.from(data));
      const sequence = parts.length;
      call.handlers.onEvent("writeAck", { seq: sequence, bytes: data.length });
      if (end) {
        const framedQuery = Buffer.concat(parts);
        const query = dnsPacket.streamDecode(framedQuery);
        const response = Buffer.from(dnsPacket.streamEncode(responsePacket(query)));
        queueMicrotask(() => {
          call.handlers.onChunk(response, false);
          call.handlers.onEvent("end", { socketId: "dns-tcp-" + id });
          call.handlers.onEvent("close", { socketId: "dns-tcp-" + id, hadError: false });
          call.resolve({ closed: true });
        });
      }
    }
    return true;
  };

  const api = globalThis[Symbol.for("niva.node-compat.runtime")].createDnsModule(niva);
  api.setServers(["127.0.0.1:5300", "[2001:db8::53]:5353"]);
  assert.deepEqual(api.getServers(), ["127.0.0.1:5300", "[2001:db8::53]:5353"]);
  assert.throws(() => api.setServers(["999.1.1.1"]), { code: "EINVAL" });

  assert.deepEqual(await api.promises.resolve4("example.test"), ["203.0.113.10"]);
  truncateNextResponse = true;
  assert.deepEqual(await api.promises.resolve6("example.test"), ["2001:db8::10"]);
  assert.equal(tcpFallbackCount, 1, "truncated UDP response retries the query over TCP");
  assert.deepEqual(await api.promises.resolve("example.test", "MX"), [{ exchange: "mail.example.test", priority: 10 }]);
  assert.deepEqual(await api.promises.resolveTxt("example.test"), [["hello", "world"]]);
  assert.deepEqual(await api.promises.resolveSoa("example.test"), {
    nsname: "ns.example.test", hostmaster: "host.example.test", serial: 1, refresh: 2, retry: 3, expire: 4, minttl: 5,
  });
  await assert.rejects(api.promises.resolve("example.test", "NOT-A-TYPE"), { code: "ENOTIMP" });

  niva.call = (method, args) => {
    niva.unary.push({ method, args });
    return Promise.resolve(args[0].all
      ? [{ address: "192.0.2.10", family: 4 }, { address: "2001:db8::10", family: 6 }]
      : { address: "192.0.2.10", family: 4 });
  };
  const result = await new Promise((resolve, reject) => api.lookup("example.test", { family: 4 }, (error, address, family) => error ? reject(error) : resolve({ address, family })));
  assert.deepEqual(result, { address: "192.0.2.10", family: 4 });
  assert.deepEqual(niva.unary[0], { method: "os.dnsLookup", args: [{ hostname: "example.test", family: 4 }] });
  assert.deepEqual(await api.promises.lookup("example.test", { all: true }), [
    { address: "192.0.2.10", family: 4 },
    { address: "2001:db8::10", family: 6 },
  ]);
  assert.deepEqual(await api.promises.resolve4("example.test"), ["203.0.113.10"]);
  await assert.rejects(api.promises.resolve("example.test", "NOT-A-TYPE"), { code: "ENOTIMP" });
});

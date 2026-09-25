(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createDnsModule === "function") return;

  function createDnsModule(niva?) {
    var dnsPacket = runtime.vendor && runtime.vendor.dnsPacket;
    var datagram = runtime.createDgramModule(niva);
    var net = runtime.createNetModule(niva);
    if (!dnsPacket || !datagram || !net) throw new Error("Load DNS, dgram, and net adapters before dns.");
    function enqueue(callback) {
      if (typeof root.queueMicrotask === "function") root.queueMicrotask(callback);
      else Promise.resolve().then(callback);
    }

    function makeError(code, syscall, hostname, message?) {
      var error = runtime.bridgeError(message || (syscall + " " + code + " " + hostname), code);
      error.errno = code;
      error.syscall = syscall;
      error.hostname = hostname;
      return error;
    }

    function normalizeError(error, syscall, hostname) {
      var result = runtime.nativeError(error);
      if (!result.code) result.code = /not found|no address/i.test(result.message) ? "ENOTFOUND" : "EAI_FAIL";
      if (!result.syscall) result.syscall = syscall;
      if (!result.hostname) result.hostname = hostname;
      if (!result.errno) result.errno = result.code;
      return result;
    }

    function validateName(name) {
      if (typeof name !== "string") { var error = new TypeError('The "name" argument must be of type string'); error.code = "ERR_INVALID_ARG_TYPE"; throw error; }
    }

    function validateHostname(hostname) {
      if (typeof hostname !== "string") throw new TypeError("The \"hostname\" argument must be of type string");
      if (!hostname || /[\s/?#]/.test(hostname)) throw makeError("EINVAL", "queryA", hostname, "Invalid DNS hostname");
    }

    function canonicalHostname(hostname) {
      validateHostname(hostname);
      var trailingDot = hostname.endsWith(".");
      var value = trailingDot ? hostname.slice(0, -1) : hostname;
      if (net.isIP(value)) return value;
      try {
        var ascii = new URL("http://" + value).hostname;
        return trailingDot ? ascii + "." : ascii;
      } catch (_) {
        return hostname;
      }
    }

    function normalizeIp(value) {
      value = String(value || "").toLowerCase().replace(/%25/g, "%");
      if (net.isIP(value) !== 6) return value;
      try { return new URL("http://[" + value + "]/" ).hostname.toLowerCase().replace(/^\[|\]$/g, ""); }
      catch (_) { return value; }
    }

    function parseServer(value) {
      if (typeof value !== "string" || !value) throw makeError("EINVAL", "setServers", value, "DNS server must be an IP address");
      var address;
      var port = 53;
      var bracket = /^\[([^\]]+)\](?::(\d+))?$/.exec(value);
      if (bracket) {
        address = bracket[1];
        if (bracket[2] !== undefined) port = Number(bracket[2]);
      } else if (net.isIP(value)) {
        address = value;
      } else {
        var colon = value.lastIndexOf(":");
        if (colon > 0 && value.indexOf(":") === colon) {
          address = value.slice(0, colon);
          port = Number(value.slice(colon + 1));
        } else {
          throw makeError("EINVAL", "setServers", value, "DNS server must be an IP address with an optional port");
        }
      }
      var family = net.isIP(address);
      if (!family || !Number.isInteger(port) || port < 1 || port > 65535) {
        throw makeError("EINVAL", "setServers", value, "Invalid DNS server address or port");
      }
      return { address: address, port: port, family: family };
    }

    function serverToString(server) {
      if (server.family === 6) return server.port === 53 ? server.address : "[" + server.address + "]:" + server.port;
      return server.port === 53 ? server.address : server.address + ":" + server.port;
    }

    function hostServerList(override) {
      if (override !== null) return override.slice();
      if (typeof runtime.callSync !== "function" || !niva && !root.Niva) {
        throw runtime.bridgeError("System DNS server list is unavailable outside a Niva page", "ENOTSUP");
      }
      var nativeServers = runtime.callSync(niva, "os.dnsServers", []);
      if (!Array.isArray(nativeServers)) throw runtime.bridgeError("Native DNS server list is invalid", "EAI_FAIL");
      return nativeServers.map(parseServer);
    }

    function validatePacket(response: any, query, server, source?) {
      if (response.type !== "response" || response.id !== query.id) return false;
      var questions = response.questions || [];
      if (questions.length !== 1) return false;
      if (String(questions[0].name).toLowerCase() !== String(query.questions[0].name).toLowerCase()) return false;
      if (String(questions[0].type).toUpperCase() !== String(query.questions[0].type).toUpperCase()) return false;
      if (source && (normalizeIp(source.address) !== normalizeIp(server.address) || Number(source.port) !== server.port)) return false;
      return true;
    }

    function queryUdp(server, query, timeout): Promise<any> {
      return new Promise(function (resolve, reject) {
        var socket = datagram.createSocket(server.family === 6 ? "udp6" : "udp4");
        var timer;
        var settled = false;
        function cleanup() {
          if (timer !== undefined) root.clearTimeout(timer);
          socket.removeAllListeners("message");
          socket.removeAllListeners("error");
          try { socket.close(); } catch (_) {}
        }
        function finish(error, response?) {
          if (settled) return;
          settled = true;
          cleanup();
          if (error) reject(error); else resolve(response);
        }
        socket.on("error", function (error) { finish(normalizeError(error, "query", query.questions[0].name)); });
        socket.on("message", function (message, remoteInfo) {
          var response;
          try { response = dnsPacket.decode(message); }
          catch (_) { return; }
          if (!validatePacket(response, query, server, remoteInfo)) return;
          finish(null, response);
        });
        timer = root.setTimeout(function () { finish(makeError("ETIMEOUT", "query", query.questions[0].name, "DNS query timed out")); }, timeout);
        socket.once("listening", function () {
          socket.send(BufferFrom(dnsPacket.encode(query)), server.port, server.address, function (error) {
            if (error) finish(normalizeError(error, "query", query.questions[0].name));
          });
        });
        try { socket.bind(0, server.family === 6 ? "::" : "0.0.0.0"); }
        catch (error) { finish(normalizeError(error, "query", query.questions[0].name)); }
      });
    }

    function BufferFrom(value) { return runtime.vendor.Buffer.from(value); }

    function queryTcp(server, query, timeout): Promise<any> {
      return new Promise(function (resolve, reject) {
        var socket;
        var timer;
        var received = BufferFrom([]);
        var settled = false;
        function cleanup() {
          if (timer !== undefined) root.clearTimeout(timer);
          if (socket && !socket.destroyed) socket.destroy();
        }
        function finish(error, response?) {
          if (settled) return;
          settled = true;
          cleanup();
          if (error) reject(error); else resolve(response);
        }
        try { socket = net.connect({ host: server.address, port: server.port }); }
        catch (error) { finish(normalizeError(error, "query", query.questions[0].name)); return; }
        timer = root.setTimeout(function () { finish(makeError("ETIMEOUT", "query", query.questions[0].name, "DNS TCP query timed out")); }, timeout);
        socket.once("connect", function () {
          try { socket.end(BufferFrom(dnsPacket.streamEncode(query))); }
          catch (error) { finish(normalizeError(error, "query", query.questions[0].name)); }
        });
        socket.on("data", function (chunk) {
          received = runtime.vendor.Buffer.concat([received, BufferFrom(chunk)]);
          if (received.length < 2 || received.length < received.readUInt16BE(0) + 2) return;
          var response;
          try { response = dnsPacket.streamDecode(received); }
          catch (error) { finish(makeError("EBADRESP", "query", query.questions[0].name, error.message)); return; }
          if (!response || !validatePacket(response, query, server)) {
            finish(makeError("EBADRESP", "query", query.questions[0].name, "Invalid DNS TCP response"));
            return;
          }
          finish(null, response);
        });
        socket.once("error", function (error) { finish(normalizeError(error, "query", query.questions[0].name)); });
        socket.once("close", function () {
          if (!settled) finish(makeError("ECONNRESET", "query", query.questions[0].name, "DNS TCP connection closed before a response"));
        });
      });
    }

    function responseError(response, hostname, type) {
      var syscall = "query" + type;
      if (response.rcode !== "NOERROR") {
        var codes = { NXDOMAIN: "ENOTFOUND", SERVFAIL: "ESERVFAIL", REFUSED: "EREFUSED", FORMERR: "EBADRESP", NOTIMP: "ENOTIMP" };
        var code = codes[response.rcode] || "EBADRESP";
        return makeError(code, syscall, hostname, syscall + " " + code + " " + hostname);
      }
      return undefined;
    }

    function extractRecords(response, hostname, type) {
      var answers = (response.answers || []).filter(function (answer) { return String(answer.type).toUpperCase() === type; });
      if (!answers.length) {
        var cname = (response.answers || []).find(function (answer) { return String(answer.type).toUpperCase() === "CNAME"; });
        if (cname && (type === "A" || type === "AAAA")) return { cname: cname.data };
        throw makeError("ENODATA", "query" + type, hostname, "query" + type + " ENODATA " + hostname);
      }
      return answers;
    }

    function createPacket(hostname, type) {
      var random = runtime.crypto && runtime.crypto.randomBytes ? runtime.crypto.randomBytes(2) : undefined;
      var id = random ? random.readUInt16BE(0) : Math.floor(Math.random() * 65536);
      return { type: "query", id: id, flags: dnsPacket.RECURSION_DESIRED, questions: [{ type: type, name: hostname }] };
    }

    function resolvePacket(resolver, hostname, type, seen?): Promise<any> {
      validateHostname(hostname);
      var name = canonicalHostname(hostname);
      seen = seen || new Set();
      if (seen.has(name.toLowerCase())) return Promise.reject(makeError("EBADRESP", "query" + type, hostname, "DNS CNAME loop"));
      seen.add(name.toLowerCase());
      var query = createPacket(name, type);
      var servers;
      try { servers = resolver.getServers().map(parseServer); }
      catch (error) { return Promise.reject(error); }
      if (!servers.length) return Promise.reject(makeError("ENODATA", "query" + type, hostname, "No DNS servers configured"));
      var lastError;

      function tryServer(serverIndex, attempt) {
        if (serverIndex >= servers.length) return Promise.reject(lastError || makeError("ETIMEOUT", "query" + type, hostname, "DNS query timed out"));
        if (attempt >= resolver.tries) return tryServer(serverIndex + 1, 0);
        var server = servers[serverIndex];
        return queryUdp(server, query, resolver.timeout).then(function (response) {
          if (response.flags & dnsPacket.TRUNCATED_RESPONSE) return queryTcp(server, query, resolver.timeout);
          return response;
        }).then(function (response) {
          var error = responseError(response, hostname, type);
          if (error) {
            if (error.code === "ENOTFOUND" || error.code === "ENODATA") throw error;
            lastError = error;
            return tryServer(serverIndex + 1, 0);
          }
          var records;
          try { records = extractRecords(response, hostname, type); }
          catch (error) {
            if (error.cname) return resolvePacket(resolver, error.cname, type, seen);
            throw error;
          }
          return records;
        }, function (error) {
          lastError = error;
          return tryServer(serverIndex, attempt + 1);
        });
      }
      return tryServer(0, 0);
    }

    function formatRecord(record) {
      var data = record.data;
      switch (String(record.type).toUpperCase()) {
        case "A": case "AAAA": case "CNAME": case "NS": case "PTR":
          return data;
        case "MX":
          return { exchange: data.exchange, priority: data.preference };
        case "TXT":
          return Array.isArray(data) ? data.map(String) : [String(data)];
        case "SRV":
          return { priority: data.priority, weight: data.weight, port: data.port, name: data.target };
        case "SOA":
          return { nsname: data.mname, hostmaster: data.rname, serial: data.serial, refresh: data.refresh, retry: data.retry, expire: data.expire, minttl: data.minimum };
        default:
          return data;
      }
    }

    function Resolver(options: any = {}, _niva?: any): any {
      if (!(this instanceof Resolver)) return Reflect.construct(Resolver, [options, _niva]);
      options = options || {};
      if (options === null || typeof options !== "object") throw new TypeError("options must be an object");
      this.timeout = options.timeout === undefined ? 2000 : Number(options.timeout);
      this.tries = options.tries === undefined ? 2 : Number(options.tries);
      if (!Number.isInteger(this.timeout) || this.timeout < 1) throw new RangeError("timeout must be a positive integer");
      if (!Number.isInteger(this.tries) || this.tries < 1) throw new RangeError("tries must be a positive integer");
      this._serverOverride = null;
    }
    Resolver.prototype.setServers = function (servers) {
      if (!Array.isArray(servers)) throw new TypeError("servers must be an array");
      this._serverOverride = servers.length ? servers.map(parseServer) : null;
    };
    Resolver.prototype.getServers = function () {
      return hostServerList(this._serverOverride).map(serverToString);
    };
    Resolver.prototype._resolve = function (hostname, type, callback, options) {
      validateHostname(hostname);
      var self = this;
      resolvePacket(this, hostname, type).then(function (records) {
        if (type === "SOA") callback(null, formatRecord(records[0]));
        else if (type === "A" || type === "AAAA") callback(null, records.map(function (record) {
          return options && options.ttl ? { address: record.data, ttl: record.ttl } : record.data;
        }));
        else callback(null, records.map(formatRecord));
      }, function (error) { callback(error); });
      return undefined;
    };
    Resolver.prototype.resolve = function (hostname, rrtype, callback) {
      if (typeof rrtype === "function") { callback = rrtype; rrtype = "A"; }
      rrtype = rrtype === undefined ? "A" : String(rrtype).toUpperCase();
      if (typeof callback !== "function") { var error = new TypeError("The \"callback\" argument must be of type function"); error.code = "ERR_INVALID_ARG_TYPE"; throw error; }
      if (!["A", "AAAA", "CNAME", "MX", "TXT", "NS", "SRV", "SOA", "PTR"].includes(rrtype)) {
        enqueue(function () { callback(makeError("ENOTIMP", "query", hostname, "query type not supported")); });
        return undefined;
      }
      return this._resolve(hostname, rrtype, callback);
    };
    Resolver.prototype.resolve4 = function (hostname, options, callback) {
      if (typeof options === "function") { callback = options; options = undefined; }
      if (typeof callback !== "function") { var error = new TypeError("The \"callback\" argument must be of type function"); error.code = "ERR_INVALID_ARG_TYPE"; throw error; }
      return this._resolve(hostname, "A", callback, options);
    };
    Resolver.prototype.resolve6 = function (hostname, options, callback) {
      if (typeof options === "function") { callback = options; options = undefined; }
      if (typeof callback !== "function") { var error = new TypeError("The \"callback\" argument must be of type function"); error.code = "ERR_INVALID_ARG_TYPE"; throw error; }
      return this._resolve(hostname, "AAAA", callback, options);
    };
    ["Cname:CNAME", "Mx:MX", "Txt:TXT", "Ns:NS", "Srv:SRV", "Soa:SOA", "Ptr:PTR"].forEach(function (pair) {
      var parts = pair.split(":");
      Resolver.prototype["resolve" + parts[0]] = function (hostname, callback) {
        validateName(hostname);
        if (typeof callback !== "function") { var error = new TypeError("The \"callback\" argument must be of type function"); error.code = "ERR_INVALID_ARG_TYPE"; throw error; }
        return this._resolve(hostname, parts[1], callback);
      };
    });

    var defaultResolver: any = Reflect.construct(Resolver as any, []);
    var promises: any = {};
    ["lookup", "resolve", "resolve4", "resolve6", "resolveCname", "resolveMx", "resolveTxt", "resolveNs", "resolveSrv", "resolveSoa", "resolvePtr"].forEach(function (name) {
      promises[name] = function () {
        var args = Array.prototype.slice.call(arguments);
        if (name !== "lookup") validateName(args[0]);
        return new Promise(function (resolve, reject) {
          args.push(function (error, value, family) {
            if (error) reject(error);
            else if (name === "lookup" && !(args[1] && args[1].all)) resolve({ address: value, family: family });
            else resolve(value);
          });
          defaultResolver[name].apply(defaultResolver, args);
        });
      };
    });

    function lookup(hostname, options, callback) {
      if (typeof options === "function") { callback = options; options = undefined; }
      if (typeof callback !== "function") { var error = new TypeError("The \"callback\" argument must be of type function"); error.code = "ERR_INVALID_ARG_TYPE"; throw error; }
      validateHostname(hostname);
      if (typeof options === "number") options = { family: options };
      options = options || {};
      if (typeof options !== "object") throw new TypeError("options must be an object");
      var family = options.family === undefined ? 0 : Number(options.family);
      if (![0, 4, 6].includes(family)) throw makeError("EINVAL", "getaddrinfo", hostname, "family must be 0, 4, or 6");
      var request: Record<string, any> = { hostname: hostname };
      if (family) request.family = family;
      if (options.all === true) request.all = true;
      runtime.call(niva, "os.dnsLookup", [request]).then(function (result) {
        var addresses = Array.isArray(result) ? result : result ? [result] : [];
        if (!addresses.length) { callback(makeError("ENOTFOUND", "getaddrinfo", hostname)); return; }
        if (options.all === true) callback(null, addresses.map(function (item) { return { address: item.address, family: Number(item.family) }; }));
        else callback(null, addresses[0].address, Number(addresses[0].family));
      }, function (error) { callback(normalizeError(error, "getaddrinfo", hostname)); });
      return undefined;
    }

    promises.lookup = function (hostname, options) {
      return new Promise(function (resolve, reject) {
        lookup(hostname, options, function (error, address, family) {
          if (error) reject(error);
          else if (options && options.all) resolve(address);
          else resolve({ address: address, family: family });
        });
      });
    };
    function DnsResolver(options: any = {}): any { return Reflect.construct(Resolver as any, [options, niva]); }
    DnsResolver.prototype = Resolver.prototype;
    promises.Resolver = DnsResolver;

    var module: any = {
      Resolver: Resolver,
      lookup: lookup,
      resolve: function () { return defaultResolver.resolve.apply(defaultResolver, arguments); },
      resolve4: function () { return defaultResolver.resolve4.apply(defaultResolver, arguments); },
      resolve6: function () { return defaultResolver.resolve6.apply(defaultResolver, arguments); },
      resolveCname: function () { return defaultResolver.resolveCname.apply(defaultResolver, arguments); },
      resolveMx: function () { return defaultResolver.resolveMx.apply(defaultResolver, arguments); },
      resolveTxt: function () { return defaultResolver.resolveTxt.apply(defaultResolver, arguments); },
      resolveNs: function () { return defaultResolver.resolveNs.apply(defaultResolver, arguments); },
      resolveSrv: function () { return defaultResolver.resolveSrv.apply(defaultResolver, arguments); },
      resolveSoa: function () { return defaultResolver.resolveSoa.apply(defaultResolver, arguments); },
      resolvePtr: function () { return defaultResolver.resolvePtr.apply(defaultResolver, arguments); },
      setServers: function (servers) { return defaultResolver.setServers(servers); },
      getServers: function () { return defaultResolver.getServers(); },
      promises: promises,
    };
    return module;
  }

  runtime.createDnsModule = createDnsModule;
  runtime.dns = createDnsModule();
})(globalThis);

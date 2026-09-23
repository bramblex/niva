(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function copyBytes(value) {
    if (value instanceof Uint8Array) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    throw new TypeError("data must be a string, Buffer, Uint8Array, or ArrayBuffer");
  }

  function bytesToBase64(bytes) {
    if (typeof btoa === "function") {
      var binary = "";
      var step = 0x8000;
      for (var i = 0; i < bytes.length; i += step) {
        binary += String.fromCharCode.apply(null, bytes.subarray(i, i + step));
      }
      return btoa(binary);
    }
    if (typeof Buffer !== "undefined") return Buffer.from(bytes).toString("base64");
    throw new Error("base64 encoding is unavailable in this runtime");
  }

  function base64ToBytes(value) {
    if (typeof atob === "function") {
      var binary = atob(value);
      var bytes = new Uint8Array(binary.length);
      for (var i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
      return bytes;
    }
    if (typeof Buffer !== "undefined") return new Uint8Array(Buffer.from(value, "base64"));
    throw new Error("base64 decoding is unavailable in this runtime");
  }

  function encodeString(value, encoding) {
    var normalized = String(encoding || "utf8").toLowerCase().replace(/[-_]/g, "");
    if (normalized === "utf8" || normalized === "utf") return new TextEncoder().encode(value);
    if (normalized === "base64") return base64ToBytes(value);
    if (normalized === "hex") {
      if (value.length % 2 !== 0 || !/^[0-9a-f]*$/i.test(value)) throw new TypeError("Invalid hex string");
      var hex = new Uint8Array(value.length / 2);
      for (var i = 0; i < hex.length; i += 1) hex[i] = parseInt(value.slice(i * 2, i * 2 + 2), 16);
      return hex;
    }
    if (normalized === "ascii" || normalized === "latin1" || normalized === "binary") {
      var mask = normalized === "ascii" ? 0x7f : 0xff;
      var single = new Uint8Array(value.length);
      for (var j = 0; j < value.length; j += 1) single[j] = value.charCodeAt(j) & mask;
      return single;
    }
    if (normalized === "utf16le" || normalized === "ucs2" || normalized === "ucs2") {
      var wide = new Uint8Array(value.length * 2);
      for (var k = 0; k < value.length; k += 1) {
        var code = value.charCodeAt(k);
        wide[k * 2] = code & 0xff;
        wide[k * 2 + 1] = code >>> 8;
      }
      return wide;
    }
    throw new TypeError("Unsupported encoding: " + encoding);
  }

  function decodeBytes(bytes, encoding) {
    var normalized = String(encoding || "utf8").toLowerCase().replace(/[-_]/g, "");
    if (normalized === "base64") return bytesToBase64(bytes);
    if (normalized === "hex") {
      var parts = new Array(bytes.length);
      for (var i = 0; i < bytes.length; i += 1) parts[i] = bytes[i].toString(16).padStart(2, "0");
      return parts.join("");
    }
    if (normalized === "ascii" || normalized === "latin1" || normalized === "binary") {
      var chars = new Array(bytes.length);
      for (var j = 0; j < bytes.length; j += 1) chars[j] = String.fromCharCode(bytes[j] & (normalized === "ascii" ? 0x7f : 0xff));
      return chars.join("");
    }
    if (normalized === "utf16le" || normalized === "ucs2") {
      var wide = "";
      for (var k = 0; k + 1 < bytes.length; k += 2) wide += String.fromCharCode(bytes[k] | (bytes[k + 1] << 8));
      return wide;
    }
    var label = normalized === "utf" ? "utf-8" : encoding;
    try { return new TextDecoder(label).decode(bytes); }
    catch (_) { throw new TypeError("Unsupported encoding: " + encoding); }
  }

  function readOptions(options) {
    if (typeof options === "string") return { encoding: options };
    return options || {};
  }

  function createStats(raw, path) {
    var size = Number(raw.size || 0);
    var atimeMs = Number(raw.accessed || 0);
    var mtimeMs = Number(raw.modified || 0);
    var birthtimeMs = Number(raw.created || 0);
    var isFile = !!raw.isFile;
    var isDirectory = !!raw.isDir;
    var isSymbolicLink = !!raw.isSymlink;
    var stats = {
      size: size, atimeMs: atimeMs, mtimeMs: mtimeMs, birthtimeMs: birthtimeMs,
      atime: new Date(atimeMs), mtime: new Date(mtimeMs), birthtime: new Date(birthtimeMs),
      path: path,
      isFile: function () { return isFile; },
      isDirectory: function () { return isDirectory; },
      isSymbolicLink: function () { return isSymbolicLink; },
      isBlockDevice: function () { return false; },
      isCharacterDevice: function () { return false; },
      isFIFO: function () { return false; },
      isSocket: function () { return false; },
    };
    return stats;
  }

  function createDirent(name, parentPath, stats) {
    var entry = {
      name: name,
      parentPath: parentPath,
      path: parentPath,
      isFile: function () { return stats.isFile(); },
      isDirectory: function () { return stats.isDirectory(); },
      isSymbolicLink: function () { return stats.isSymbolicLink(); },
      isBlockDevice: function () { return false; },
      isCharacterDevice: function () { return false; },
      isFIFO: function () { return false; },
      isSocket: function () { return false; },
    };
    return entry;
  }

  function createFsModule(niva) {
    function fsApi() { return runtime.api(niva, "fs"); }
    function bridge(method, args) {
      try {
        return Promise.resolve(fsApi()[method].apply(null, args)).catch(function (error) {
          throw runtime.nativeError(error);
        });
      } catch (error) {
        return Promise.reject(runtime.nativeError(error));
      }
    }

    function readFile(path, options) {
      var parsed = readOptions(options);
      var encoding = parsed.encoding === undefined ? "utf8" : parsed.encoding;
      if (encoding === null) {
        return bridge("read", [path, "base64"]).then(function (value) {
          var bytes = base64ToBytes(value);
          return runtime.buffer ? runtime.buffer.Buffer.from(bytes) : bytes;
        });
      }
      if (typeof encoding !== "string") return Promise.reject(new TypeError("encoding must be a string or null"));
      if (encoding.toLowerCase() === "utf8" || encoding.toLowerCase() === "utf-8") {
        return bridge("read", [path, "utf8"]);
      }
      return bridge("read", [path, "base64"]).then(function (value) { return decodeBytes(base64ToBytes(value), encoding); });
    }

    function writeLike(method, path, data, options) {
      var parsed = readOptions(options);
      if (parsed.mode !== undefined) return Promise.reject(runtime.bridgeError("File permission modes are unavailable from the Niva bridge", "ENOTSUP"));
      var encoding = parsed.encoding || "utf8";
      if (typeof data === "string") {
        var normalized = encoding.toLowerCase().replace(/[-_]/g, "");
        if (normalized === "utf8" || normalized === "utf") return bridge(method, [path, data, "utf8"]);
        if (normalized === "base64") return bridge(method, [path, bytesToBase64(base64ToBytes(data)), "base64"]);
        return bridge(method, [path, bytesToBase64(encodeString(data, encoding)), "base64"]);
      }
      return bridge(method, [path, bytesToBase64(copyBytes(data)), "base64"]);
    }

    function stat(path, options) {
      if (options && options.bigint) return Promise.reject(runtime.bridgeError("bigint Stats are unavailable from the Niva bridge", "ENOTSUP"));
      return bridge("stat", [path]).then(function (raw) { return createStats(raw, path); });
    }

    var module = {
      constants: { F_OK: 0, R_OK: 4, W_OK: 2, X_OK: 1, COPYFILE_EXCL: 1, COPYFILE_FICLONE: 2, COPYFILE_FICLONE_FORCE: 4 },
      readFile: readFile,
      writeFile: function (path, data, options) { return writeLike("write", path, data, options); },
      appendFile: function (path, data, options) { return writeLike("append", path, data, options); },
      mkdir: function (path, options) {
        if (options && options.mode !== undefined) return Promise.reject(runtime.bridgeError("Directory permission modes are unavailable from the Niva bridge", "ENOTSUP"));
        var recursive = !!(options && options.recursive);
        return bridge(recursive ? "createDirAll" : "createDir", [path]);
      },
      readdir: function (path, options) {
        var parent = path === undefined ? "." : path;
        var withFileTypes = !!(options && options.withFileTypes);
        return bridge("readDir", [parent]).then(function (names) {
          if (!withFileTypes) return names;
          return Promise.all(names.map(function (name) {
            var childPath = runtime.path.join(parent, name);
            return stat(childPath).then(function (value) { return createDirent(name, parent, value); });
          }));
        });
      },
      stat: stat,
      access: function (path, mode) {
        if (mode !== undefined && mode !== 0) return Promise.reject(runtime.bridgeError("Niva can only check whether a path exists; permission modes are unsupported", "ENOTSUP"));
        return bridge("exists", [path]).then(function (exists) {
          if (!exists) throw runtime.bridgeError("ENOENT: no such file or directory, access '" + path + "'", "ENOENT", { path: path, syscall: "access" });
        });
      },
      rename: function (oldPath, newPath) {
        return bridge("move", [oldPath, newPath, { contentOnly: true, overwrite: true }]);
      },
      rm: function (path, options) {
        options = options || {};
        return bridge("exists", [path]).then(function (exists) {
          if (!exists && options.force) return undefined;
          if (!exists) throw runtime.bridgeError("ENOENT: no such file or directory, rm '" + path + "'", "ENOENT", { path: path, syscall: "rm" });
          return stat(path).then(function (value) {
            if (value.isDirectory() && !options.recursive) {
              return module.readdir(path).then(function (entries) {
                if (entries.length > 0) throw runtime.bridgeError("ERR_FS_EISDIR: Path is a directory; use recursive: true to remove it", "ERR_FS_EISDIR", { path: path });
                return bridge("remove", [path]);
              });
            }
            return bridge("remove", [path]);
          });
        });
      },
      cp: function (source, destination, options) {
        options = options || {};
        return stat(source).then(function (sourceStats) {
          if (sourceStats.isDirectory() && !options.recursive) {
            throw runtime.bridgeError("ERR_FS_CP_DIR_TO_NON_DIR: recursive must be true when copying a directory", "ERR_FS_CP_DIR_TO_NON_DIR", { path: source });
          }
          if (options.dereference === false || options.verbatimSymlinks || options.preserveTimestamps || options.filter) {
            throw runtime.bridgeError("The requested fs.cp option is unsupported by the Niva bridge", "ENOTSUP");
          }
          return bridge("copy", [source, destination, {
            contentOnly: true,
            overwrite: options.force !== false,
            skipExist: false,
          }]);
        });
      },
      copyFile: function (source, destination, mode) {
        if (mode !== undefined && (mode & ~module.constants.COPYFILE_EXCL) !== 0) {
          return Promise.reject(runtime.bridgeError("Niva does not support copyFile clone flags", "ENOTSUP"));
        }
        return stat(source).then(function (sourceStats) {
          if (!sourceStats.isFile()) throw runtime.bridgeError("EISDIR: copyFile source is not a regular file", "EISDIR", { path: source });
          return bridge("copy", [source, destination, {
            overwrite: !(mode & module.constants.COPYFILE_EXCL),
            contentOnly: true,
          }]);
        });
      },
    };
    var promises = {};
    Object.keys(module).forEach(function (key) {
      if (key !== "promises") promises[key] = module[key];
    });
    module.promises = promises;
    return module;
  }

  runtime.createFsModule = createFsModule;
  runtime.fs = createFsModule();
})(globalThis);

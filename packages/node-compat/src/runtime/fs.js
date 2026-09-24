(function (root) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createFsModule === "function") return;
    function createFsModule(niva) {
        var Buffer = runtime.vendor.Buffer;
        var constants = { F_OK: 0, R_OK: 4, W_OK: 2, X_OK: 1, COPYFILE_EXCL: 1, COPYFILE_FICLONE: 2, COPYFILE_FICLONE_FORCE: 4 };
        function path(value) {
            if (value instanceof URL)
                value = runtime.url.fileURLToPath(value);
            if (Buffer.isBuffer(value))
                value = value.toString();
            if (typeof value !== "string")
                throw new TypeError("path must be a string, Buffer, or file URL");
            if (value.indexOf("\0") !== -1)
                throw new TypeError("path must not contain NUL");
            return value;
        }
        function options(value) { return typeof value === "string" ? { encoding: value } : value || {}; }
        function stats(raw, opts) {
            var result = {};
            ["dev", "ino", "mode", "nlink", "uid", "gid", "rdev", "size", "blksize", "blocks", "atimeMs", "mtimeMs", "ctimeMs", "birthtimeMs"].forEach(function (name) {
                result[name] = opts && opts.bigint ? BigInt(Math.trunc(raw[name] || 0)) : Number(raw[name] || 0);
            });
            ["atime", "mtime", "ctime", "birthtime"].forEach(function (name) { result[name] = new Date(Number(raw[name + "Ms"] || 0)); });
            ["File", "Directory", "SymbolicLink", "BlockDevice", "CharacterDevice", "FIFO", "Socket"].forEach(function (name) {
                result["is" + name] = function () { return !!raw[name === "Directory" ? "isDir" : name === "SymbolicLink" ? "isSymlink" : "is" + name]; };
            });
            return result;
        }
        function prepare(name, values) {
            var args = { path: path(values[0]) }, opts;
            switch (name) {
                case "readFile":
                    opts = options(values[1]);
                    args.flag = opts.flag;
                    break;
                case "writeFile":
                case "appendFile":
                    opts = options(values[2]);
                    var data = values[1];
                    args.data = (ArrayBuffer.isView(data) ? Buffer.from(data.buffer, data.byteOffset, data.byteLength) : Buffer.from(data, opts.encoding || "utf8")).toString("base64");
                    args.flag = opts.flag;
                    args.mode = opts.mode;
                    break;
                case "mkdir":
                    opts = typeof values[1] === "number" ? { mode: values[1] } : options(values[1]);
                    Object.assign(args, opts);
                    break;
                case "readdir":
                case "stat":
                case "lstat":
                    opts = options(values[1]);
                    break;
                case "rename":
                case "copyFile":
                    args.destination = path(values[1]);
                    args.flags = values[2];
                    break;
                case "access":
                    args.mode = values[1];
                    break;
                case "rm":
                    Object.assign(args, options(values[1]));
                    break;
            }
            return { args: args, options: opts || {} };
        }
        function decode(name, raw, opts, pathname) {
            if (name === "readFile") {
                var data = Buffer.from(raw, "base64");
                return opts.encoding ? data.toString(opts.encoding) : data;
            }
            if (name === "stat" || name === "lstat")
                return stats(raw, opts);
            if (name === "readdir")
                return raw.map(function (entry) {
                    var name = opts.encoding === "buffer" ? Buffer.from(entry.name) : entry.name;
                    return opts.withFileTypes ? Object.assign(stats(entry.stats), { name: name, parentPath: pathname, path: pathname }) : name;
                });
            if (name === "mkdir")
                return raw === null ? undefined : raw;
            return raw === null ? undefined : raw;
        }
        function invoke(name, values, sync) {
            var prepared = prepare(name, values);
            if (prepared.options.signal && prepared.options.signal.aborted) {
                var error = runtime.bridgeError("The operation was aborted", "ABORT_ERR");
                error.name = "AbortError";
                throw error;
            }
            if (sync)
                return decode(name, runtime.callSync(niva, "fs.node", [name, prepared.args]), prepared.options, prepared.args.path);
            return runtime.call(niva, "fs.node", [name, prepared.args]).then(function (raw) { return decode(name, raw, prepared.options, prepared.args.path); }, function (error) { throw runtime.nativeError(error); });
        }
        var module = { constants: constants }, promises = { constants: constants };
        ["readFile", "writeFile", "appendFile", "mkdir", "readdir", "stat", "lstat", "realpath", "rename", "copyFile", "access", "rm", "unlink"].forEach(function (name) {
            module[name + "Sync"] = function () { return invoke(name, Array.from(arguments), true); };
            promises[name] = function () { try {
                return Promise.resolve(invoke(name, Array.from(arguments), false));
            }
            catch (error) {
                return Promise.reject(error);
            } };
            module[name] = function () {
                var values = Array.from(arguments), callback = values.pop();
                if (typeof callback !== "function")
                    throw new TypeError("callback must be a function");
                promises[name].apply(null, values).then(function (value) { callback(null, value); }, callback);
            };
        });
        module.existsSync = function (value) { try {
            module.accessSync(value);
            return true;
        }
        catch (error) {
            if (error instanceof TypeError)
                throw error;
            return false;
        } };
        promises.cp = function (source, destination, opts) {
            opts = opts || {};
            if (opts.filter)
                return Promise.resolve(opts.filter(source, destination)).then(function (keep) { if (keep)
                    return promises.cp(source, destination, Object.assign({}, opts, { filter: undefined })); });
            return promises.lstat(source).then(function (stat) {
                if (stat.isDirectory()) {
                    if (!opts.recursive)
                        throw runtime.bridgeError("recursive required for directory copy", "ERR_FS_EISDIR");
                    return promises.mkdir(destination, { recursive: true }).then(function () { return promises.readdir(source); }).then(function (names) { return names.reduce(function (p, name) { return p.then(function () { return promises.cp(runtime.path.join(source, name), runtime.path.join(destination, name), opts); }); }, Promise.resolve()); });
                }
                if (stat.isSymbolicLink())
                    throw runtime.bridgeError("Symbolic link copy is not supported", "ENOTSUP");
                return promises.copyFile(source, destination, opts.force === false ? 1 : 0).catch(function (error) { if (error.code === "EEXIST" && !opts.errorOnExist)
                    return; throw error; });
            });
        };
        module.cp = function (source, destination, opts, callback) { if (typeof opts === "function") {
            callback = opts;
            opts = {};
        } if (typeof callback !== "function")
            throw new TypeError("callback must be a function"); promises.cp(source, destination, opts).then(function () { callback(null); }, callback); };
        function control(id, op, args) {
            try {
                return runtime.stream(niva, "fs.handle", [id, op, args || {}]).promise.catch(function (error) { throw runtime.nativeError(error); });
            }
            catch (error) {
                return Promise.reject(runtime.nativeError(error));
            }
        }
        promises.open = function (value, flag, mode) {
            var pathname = path(value);
            return new Promise(function (resolve, reject) {
                var opened = false, call;
                call = runtime.stream(niva, "fs.openHandle", [pathname, flag || "r", mode === undefined ? 438 : mode], {
                    onEvent: function (event, data) {
                        if (event !== "open")
                            return;
                        opened = true;
                        var closed = false, id = data.handle;
                        function op(name, args) { if (closed)
                            return Promise.reject(runtime.bridgeError("FileHandle is closed", "EBADF")); return control(id, name, args); }
                        var handle = { fd: id,
                            close: function () { if (closed)
                                return Promise.resolve(); return op("close").then(function () { closed = true; }); },
                            stat: function (opts) { return op("stat").then(function (raw) { return stats(raw, opts); }); },
                            sync: function () { return op("sync"); }, datasync: function () { return op("datasync"); },
                            truncate: function (length) { return op("truncate", { length: length || 0 }); },
                            read: function (buffer, offset, length, position) {
                                if (!ArrayBuffer.isView(buffer)) {
                                    var opts = buffer || {};
                                    buffer = opts.buffer || Buffer.alloc(16384);
                                    offset = opts.offset || 0;
                                    length = opts.length === undefined ? buffer.length - offset : opts.length;
                                    position = opts.position;
                                }
                                offset = offset || 0;
                                if (length === undefined)
                                    length = buffer.length - offset;
                                if (!Number.isInteger(offset) || !Number.isInteger(length) || offset < 0 || length < 0 || offset + length > buffer.byteLength)
                                    return Promise.reject(new RangeError("Invalid read range"));
                                return op("read", { length: length, position: position }).then(function (raw) { new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength).set(Buffer.from(raw.data, "base64"), offset); return { bytesRead: raw.bytesRead, buffer: buffer }; });
                            },
                            write: function (buffer, offset, length, position) {
                                var original = buffer, bytes;
                                if (typeof buffer === "string") {
                                    bytes = Buffer.from(buffer, typeof length === "string" ? length : "utf8");
                                    position = offset;
                                }
                                else {
                                    offset = offset || 0;
                                    length = length === undefined ? buffer.byteLength - offset : length;
                                    if (offset < 0 || length < 0 || offset + length > buffer.byteLength)
                                        return Promise.reject(new RangeError("Invalid write range"));
                                    bytes = Buffer.from(buffer.buffer, buffer.byteOffset + offset, length);
                                }
                                return op("write", { data: bytes.toString("base64"), position: position }).then(function (raw) { return { bytesWritten: raw.bytesWritten, buffer: original }; });
                            },
                            readFile: async function (opts) { opts = options(opts); var chunks = [], size = 0; for (;;) {
                                var part = await op("read", { length: 65536 });
                                if (!part.bytesRead)
                                    break;
                                var bytes = Buffer.from(part.data, "base64");
                                chunks.push(bytes);
                                size += bytes.length;
                            } var result = Buffer.concat(chunks, size); return opts.encoding ? result.toString(opts.encoding) : result; },
                            writeFile: async function (value, opts) { var bytes = Buffer.from(value, options(opts).encoding || "utf8"), offset = 0; while (offset < bytes.length) {
                                var part = await op("write", { data: bytes.subarray(offset, offset + 65536).toString("base64") });
                                if (!part.bytesWritten)
                                    throw runtime.bridgeError("No write progress", "EIO");
                                offset += part.bytesWritten;
                            } },
                        };
                        handle.appendFile = handle.writeFile;
                        resolve(handle);
                    }
                });
                call.promise.catch(function (error) { if (!opened)
                    reject(runtime.nativeError(error)); });
            });
        };
        module.createReadStream = function (value, opts) {
            opts = opts || {};
            var position = opts.start, ending = opts.end === undefined ? Infinity : opts.end;
            var opening = promises.open(value, opts.flags || "r", opts.mode), handle;
            var stream = new runtime.vendor.stream.Readable(Object.assign({}, opts, {
                read: function (size) { var self = this; opening.then(function (file) { handle = file; var length = Math.min(size, ending - (position || 0) + 1); if (length <= 0) {
                    self.push(null);
                    return;
                } var bytes = Buffer.alloc(length); return file.read(bytes, 0, length, position).then(function (part) { if (part.bytesRead === 0) {
                    self.push(null);
                    return;
                } if (position !== undefined)
                    position += part.bytesRead; self.push(bytes.subarray(0, part.bytesRead)); }); }).catch(function (error) { self.destroy(error); }); },
                destroy: function (error, callback) { opening.then(function (file) { return file.close(); }).then(function () { callback(error); }, function (closeError) { callback(error || closeError); }); }
            }));
            opening.then(function (file) { stream.emit("open", file.fd); stream.emit("ready"); }, function (error) { stream.destroy(error); });
            return stream;
        };
        module.createWriteStream = function (value, opts) {
            opts = opts || {};
            var position = opts.start, opening = promises.open(value, opts.flags || "w", opts.mode);
            var stream = new runtime.vendor.stream.Writable(Object.assign({}, opts, {
                write: function (chunk, encoding, callback) { opening.then(async function (file) { var offset = 0; while (offset < chunk.length) {
                    var part = await file.write(chunk, offset, Math.min(65536, chunk.length - offset), position);
                    if (!part.bytesWritten)
                        throw runtime.bridgeError("No write progress", "EIO");
                    offset += part.bytesWritten;
                    if (position !== undefined)
                        position += part.bytesWritten;
                } }).then(function () { callback(); }, callback); },
                destroy: function (error, callback) { opening.then(function (file) { return file.close(); }).then(function () { callback(error); }, function (closeError) { callback(error || closeError); }); }
            }));
            opening.then(function (file) { stream.emit("open", file.fd); stream.emit("ready"); }, function (error) { stream.destroy(error); });
            return stream;
        };
        module.watch = function (value, opts, listener) {
            if (typeof opts === "function") {
                listener = opts;
                opts = {};
            }
            opts = options(opts);
            var watcher = new runtime.events.EventEmitter(), closed = false, call;
            watcher.close = function () { if (closed)
                return; closed = true; if (call)
                call.cancel(); queueMicrotask(function () { watcher.emit("close"); }); };
            watcher.ref = watcher.unref = function () { return watcher; };
            if (listener)
                watcher.on("change", listener);
            call = runtime.stream(niva, "fs.watch", [path(value), !!opts.recursive], { onEvent: function (name, data) { if (closed)
                    return; if (name === "change")
                    watcher.emit("change", data.eventType, data.filename === null ? null : opts.encoding === "buffer" ? Buffer.from(data.filename) : data.filename); } });
            call.promise.catch(function (error) { if (!closed) {
                watcher.emit("error", runtime.nativeError(error));
                watcher.close();
            } });
            if (opts.signal) {
                if (opts.signal.aborted)
                    watcher.close();
                else
                    opts.signal.addEventListener("abort", watcher.close, { once: true });
            }
            return watcher;
        };
        module.promises = promises;
        return module;
    }
    runtime.createFsModule = createFsModule;
    runtime.fs = createFsModule();
})(globalThis);

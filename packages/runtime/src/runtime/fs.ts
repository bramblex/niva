(function (root: any) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createFsModule === "function") return;
    function createFsModule(niva?) {
        var Buffer = runtime.vendor.Buffer;
        var constants: any = { F_OK: 0, R_OK: 4, W_OK: 2, X_OK: 1, COPYFILE_EXCL: 1, COPYFILE_FICLONE: 2, COPYFILE_FICLONE_FORCE: 4 };
        // Node exposes the platform's open flags from both `fs.constants` and
        // the legacy `constants` builtin. Keep this table in the JS adapter so
        // applications can inspect the same flags without inventing a Native
        // constants API. These are the values used by Node's supported hosts.
        var platform = niva && niva.bootstrap && niva.bootstrap.os && niva.bootstrap.os.platform;
        var openFlags = platform === "darwin"
            ? { O_RDONLY: 0, O_WRONLY: 1, O_RDWR: 2, O_NONBLOCK: 4, O_APPEND: 8, O_SYNC: 128, O_NOFOLLOW: 256, O_CREAT: 512, O_TRUNC: 1024, O_EXCL: 2048, O_NOCTTY: 131072, O_DIRECTORY: 1048576, O_SYMLINK: 2097152 }
            : platform === "win32"
                ? { O_RDONLY: 0, O_WRONLY: 1, O_RDWR: 2, O_APPEND: 8, O_CREAT: 256, O_TRUNC: 512, O_EXCL: 1024, O_BINARY: 32768 }
                : { O_RDONLY: 0, O_WRONLY: 1, O_RDWR: 2, O_NONBLOCK: 2048, O_APPEND: 1024, O_DSYNC: 4096, O_DIRECT: 16384, O_DIRECTORY: 65536, O_NOFOLLOW: 131072, O_CREAT: 64, O_EXCL: 128, O_NOCTTY: 256, O_TRUNC: 512, O_SYNC: 1052672 };
        Object.assign(constants, openFlags);
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
        function stats(raw, opts?) {
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
            var args: any = { path: path(values[0]) }, opts;
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
                case "cp":
                    args.destination = path(values[1]);
                    opts = options(values[2]);
                    ["recursive", "force", "errorOnExist", "dereference", "preserveTimestamps", "verbatimSymlinks", "mode"].forEach(function (name) {
                        if (opts[name] !== undefined) args[name] = opts[name];
                    });
                    if (values[3] === "createDirectory") args.directoryOnly = true;
                    if (values[3] === "finalizeDirectory") {
                        args.directoryOnly = true;
                        args.directoryFinalize = true;
                    }
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
        function isTrustedLocal() {
            return !!(niva && niva.bridge && typeof niva.bridge.isTrustedLocal === "function" && niva.bridge.isTrustedLocal());
        }
        function invokeTrustedFileStream(name, prepared) {
            var flag = prepared.args.flag;
            if (flag === undefined)
                flag = name === "readFile" ? "r" : name === "appendFile" ? "a" : "w";
            return promises.open(prepared.args.path, flag, prepared.args.mode).then(async function (handle) {
                var result, failure;
                try {
                    if (name === "readFile") {
                        result = await handle.readFile(prepared.options);
                    }
                    else {
                        // File bytes travel through the existing Native file-handle
                        // stream in bounded chunks; the IPC transport Base64-encodes
                        // each binary frame at its boundary.
                        result = await handle.writeFile(Buffer.from(prepared.args.data, "base64"));
                    }
                }
                catch (error) {
                    failure = error;
                }
                try {
                    await handle.close();
                }
                catch (error) {
                    if (!failure)
                        failure = error;
                }
                if (failure)
                    throw failure;
                return result;
            });
        }
        function invoke(name, values, sync) {
            var prepared = prepare(name, values);
            if (prepared.options.signal && prepared.options.signal.aborted) {
                var error = runtime.bridgeError("The operation was aborted", "ABORT_ERR");
                error.name = "AbortError";
                throw error;
            }
            if (!sync && isTrustedLocal() && ["readFile", "writeFile", "appendFile"].includes(name))
                return invokeTrustedFileStream(name, prepared);
            function dispatch(externalPage) {
                if (externalPage) {
                    if (sync)
                        throw runtime.bridgeError("Synchronous file APIs require a trusted local Niva page", "ERR_NIVA_LOCAL_PAGE_REQUIRED");
                    if (name === "readFile") {
                        var readEncoding = prepared.options.encoding;
                        if (typeof readEncoding !== "string" || !["utf8", "utf-8"].includes(readEncoding.toLowerCase()))
                            throw runtime.bridgeError("Remote pages may read text files only with an explicit UTF-8 encoding", "ERR_NIVA_FS_DATA_UNSUPPORTED");
                        return runtime.call(niva, "fs.readText", [prepared.args.path, readEncoding, { flag: prepared.args.flag }]).then(function (value) { return value; }, function (error) { throw runtime.nativeError(error); });
                    }
                    if (name === "writeFile" || name === "appendFile") {
                        var input = values[1];
                        var writeEncoding = prepared.options.encoding || "utf8";
                        if (typeof input !== "string" || typeof writeEncoding !== "string" || !["utf8", "utf-8"].includes(writeEncoding.toLowerCase()))
                            throw runtime.bridgeError("Remote pages may write UTF-8 string data only", "ERR_NIVA_FS_DATA_UNSUPPORTED");
                        var method = name === "appendFile" ? "fs.appendText" : "fs.writeText";
                        var writeOptions = { flag: prepared.args.flag, mode: prepared.args.mode };
                        return runtime.call(niva, method, [prepared.args.path, input, writeEncoding, writeOptions]).then(function () { return undefined; }, function (error) { throw runtime.nativeError(error); });
                    }
                    if (!["stat", "lstat", "readdir", "access", "realpath", "mkdir", "rename", "copyFile", "rm", "unlink", "cp"].includes(name))
                        throw runtime.bridgeError("fs." + name + " requires a trusted local Niva page", "ERR_NIVA_LOCAL_PAGE_REQUIRED");
                }
                if (sync)
                    return decode(name, runtime.callSync(niva, "fs.node", [name, prepared.args]), prepared.options, prepared.args.path);
                return runtime.call(niva, "fs.node", [name, prepared.args]).then(function (raw) { return decode(name, raw, prepared.options, prepared.args.path); }, function (error) { throw runtime.nativeError(error); });
            }
            if (sync)
                return dispatch(!isTrustedLocal());
            return dispatch(!isTrustedLocal());
        }
        var module: any = { constants: constants }, promises: any = { constants: constants };
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
        var syncFileDescriptors = new Map<number, any>();
        function syncFdCall(operation, args) {
            if (!isTrustedLocal())
                throw runtime.bridgeError("Synchronous file descriptors require a trusted local Niva page", "ERR_NIVA_LOCAL_PAGE_REQUIRED");
            return runtime.callSync(niva, "fs.node", [operation, args]);
        }
        function syncFd(fd) {
            if (!Number.isInteger(fd) || fd < 0) throw runtime.bridgeError("Invalid file descriptor", "EBADF");
            var entry = syncFileDescriptors.get(fd);
            if (!entry || entry.closed) throw runtime.bridgeError("Bad file descriptor", "EBADF");
            return entry;
        }
        module.openSync = function (value, flags, mode) {
            var pathname = path(value);
            var flag = flags === undefined ? "r" : flags;
            if (typeof flag !== "string") throw runtime.bridgeError("Numeric open flags are not supported by this runtime", "ERR_NIVA_FS_FLAGS_UNSUPPORTED");
            var fd = syncFdCall("open", { path: pathname, flag: flag, mode: mode === undefined ? 438 : mode });
            if (!Number.isInteger(fd) || fd < 0) throw runtime.bridgeError("Invalid Native file descriptor response", "ERR_NIVA_IPC_RESPONSE");
            var entry: any = { fd: fd, closed: false, owner: undefined };
            entry.__nivaInvalidate = function () { entry.closed = true; syncFileDescriptors.delete(fd); };
            if (typeof runtime.registerResource === "function") entry.owner = runtime.registerResource(entry);
            syncFileDescriptors.set(fd, entry);
            return fd;
        };
        module.closeSync = function (fd) {
            var entry = syncFd(fd);
            syncFdCall("close", { fd: fd });
            entry.closed = true;
            syncFileDescriptors.delete(fd);
            if (entry.owner) entry.owner.release();
        };
        module.fstatSync = function (fd, opts) {
            syncFd(fd);
            return stats(syncFdCall("fstat", { fd: fd }), opts);
        };
        module.readSync = function (fd, buffer, offset, length, position) {
            syncFd(fd);
            if (!ArrayBuffer.isView(buffer)) throw runtime.bridgeError("buffer must be a Buffer or typed array", "ERR_INVALID_ARG_TYPE");
            offset = offset === undefined ? 0 : offset;
            length = length === undefined ? buffer.byteLength - offset : length;
            position = position === undefined ? null : position;
            if (!Number.isInteger(offset) || !Number.isInteger(length) || offset < 0 || length < 0 || offset + length > buffer.byteLength)
                throw new RangeError("Invalid read range");
            if (position !== null && (!Number.isSafeInteger(position) || position < 0)) throw new RangeError("Invalid file position");
            var result = syncFdCall("read", { fd: fd, length: length, position: position });
            if (!result || !Number.isInteger(result.bytesRead) || typeof result.data !== "string")
                throw runtime.bridgeError("Invalid Native file read response", "ERR_NIVA_IPC_RESPONSE");
            new Uint8Array(buffer.buffer, buffer.byteOffset, buffer.byteLength).set(Buffer.from(result.data, "base64"), offset);
            return result.bytesRead;
        };
        module.writeSync = function (fd, buffer, offset, length, position) {
            syncFd(fd);
            var data;
            if (typeof buffer === "string") {
                var encoding = typeof length === "string" ? length : typeof offset === "string" ? offset : "utf8";
                position = typeof offset === "number" || offset === null ? offset : position;
                data = Buffer.from(buffer, encoding);
            } else {
                if (!ArrayBuffer.isView(buffer)) throw runtime.bridgeError("buffer must be a string, Buffer, or typed array", "ERR_INVALID_ARG_TYPE");
                offset = offset === undefined ? 0 : offset;
                length = length === undefined ? buffer.byteLength - offset : length;
                if (!Number.isInteger(offset) || !Number.isInteger(length) || offset < 0 || length < 0 || offset + length > buffer.byteLength)
                    throw new RangeError("Invalid write range");
                data = Buffer.from(buffer.buffer, buffer.byteOffset + offset, length);
            }
            position = position === undefined ? null : position;
            if (position !== null && (!Number.isSafeInteger(position) || position < 0)) throw new RangeError("Invalid file position");
            var result = syncFdCall("write", { fd: fd, data: data.toString("base64"), position: position });
            if (!result || !Number.isInteger(result.bytesWritten)) throw runtime.bridgeError("Invalid Native file write response", "ERR_NIVA_IPC_RESPONSE");
            return result.bytesWritten;
        };
        module.existsSync = function (value) { try {
            module.accessSync(value);
            return true;
        }
        catch (error) {
            if (error instanceof TypeError)
                throw error;
            return false;
        } };
        function validateCpOptions(value) {
            if (value === undefined) return {};
            if (value === null || typeof value !== "object") {
                var invalidOptions = new TypeError("options must be an object");
                invalidOptions.code = "ERR_INVALID_ARG_TYPE";
                throw invalidOptions;
            }
            var opts = Object.assign({}, value);
            ["dereference", "errorOnExist", "force", "preserveTimestamps", "recursive", "verbatimSymlinks"].forEach(function (name) {
                if (opts[name] !== undefined && typeof opts[name] !== "boolean") {
                    var invalidBoolean = new TypeError("options." + name + " must be a boolean");
                    invalidBoolean.code = "ERR_INVALID_ARG_TYPE";
                    throw invalidBoolean;
                }
            });
            if (opts.mode !== undefined && opts.mode !== null) {
                if (!Number.isInteger(opts.mode)) {
                    var invalidMode = new TypeError("options.mode must be an integer");
                    invalidMode.code = "ERR_INVALID_ARG_TYPE";
                    throw invalidMode;
                }
                if (opts.mode < 0 || opts.mode > 7) {
                    var outOfRange = new RangeError("options.mode must be between 0 and 7");
                    outOfRange.code = "ERR_OUT_OF_RANGE";
                    throw outOfRange;
                }
            }
            if (opts.dereference === true && opts.verbatimSymlinks === true) {
                var incompatible = new TypeError("The \"dereference\" and \"verbatimSymlinks\" options are mutually exclusive");
                incompatible.code = "ERR_INCOMPATIBLE_OPTION_PAIR";
                throw incompatible;
            }
            if (opts.filter !== undefined && typeof opts.filter !== "function") {
                var invalidFilter = new TypeError("options.filter must be a function");
                invalidFilter.code = "ERR_INVALID_ARG_TYPE";
                throw invalidFilter;
            }
            return opts;
        }
        promises.cp = function (source, destination, opts) {
            opts = validateCpOptions(opts);
            var sourcePath = path(source), destinationPath = path(destination);
            if (opts.filter === undefined)
                return invoke("cp", [sourcePath, destinationPath, opts], false).then(function () { return undefined; });
            var filter = opts.filter;
            function visit(sourceEntry, destinationEntry) {
                return Promise.resolve().then(function () { return filter(sourceEntry, destinationEntry); }).then(function (keep) {
                    if (!keep) return;
                    var metadata = opts.dereference ? promises.stat(sourceEntry) : promises.lstat(sourceEntry);
                    return metadata.then(function (stat) {
                        if (!stat.isDirectory() || !opts.recursive)
                            return invoke("cp", [sourceEntry, destinationEntry, opts], false).then(function () { return undefined; });
                        var created;
                        return invoke("cp", [sourceEntry, destinationEntry, opts, "createDirectory"], false).then(function (wasCreated) {
                            created = wasCreated;
                            return promises.readdir(sourceEntry).then(function (names) {
                                return names.reduce(function (pending, name) {
                                    return pending.then(function () {
                                        return visit(runtime.path.join(sourceEntry, name), runtime.path.join(destinationEntry, name));
                                    });
                                }, Promise.resolve());
                            });
                        }).then(function () {
                            if (created) return invoke("cp", [sourceEntry, destinationEntry, opts, "finalizeDirectory"], false).then(function () { return undefined; });
                        });
                    });
                });
            }
            return visit(sourcePath, destinationPath);
        };
        module.cp = function (source, destination, opts, callback) { if (typeof opts === "function") {
            callback = opts;
            opts = {};
        } if (typeof callback !== "function")
            throw new TypeError("callback must be a function"); promises.cp(source, destination, opts).then(function () { callback(null); }, callback); };
        function control(id, op, args?) {
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
                        var state = "open", closePromise, invalidationError, id = data.handle, owner;
                        function op(name, args?) { if (state !== "open")
                            return Promise.reject(runtime.bridgeError("FileHandle is closed", "EBADF")); return control(id, name, args); }
                        var handle: any = { fd: id,
                            close: function () {
                                if (state === "invalid") return Promise.reject(invalidationError);
                                if (closePromise) return closePromise;
                                if (state === "closed") return Promise.resolve();
                                state = "closing";
                                closePromise = control(id, "close").then(function () { state = "closed"; if (owner) owner.release(); }, function (error) { if (state === "closing") { state = "open"; closePromise = undefined; } throw error; });
                                return closePromise;
                            },
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
                        handle.__nivaInvalidate = function (error) { state = "invalid"; invalidationError = error; if (owner) owner.release(); };
                        owner = typeof runtime.registerResource === "function"
                            ? runtime.registerResource(handle, function () { return control(id, "close").catch(function () {}); })
                            : null;
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
            var watcher = new runtime.events.EventEmitter(), closed = false, call, owner, abortListener;
            watcher.close = function () { if (closed)
                return; closed = true; if (call)
                call.cancel(); if (owner) owner.release(); if (opts.signal && abortListener) opts.signal.removeEventListener("abort", abortListener); queueMicrotask(function () { watcher.emit("close"); }); };
            watcher.ref = watcher.unref = function () { return watcher; };
            watcher.__nivaInvalidate = function (error) {
                if (closed) return;
                closed = true;
                if (opts.signal && abortListener) opts.signal.removeEventListener("abort", abortListener);
                if (watcher.listenerCount("error")) {
                    try { watcher.emit("error", error); } catch (_) { /* notify other resources during session cleanup */ }
                }
                watcher.emit("close");
            };
            if (listener)
                watcher.on("change", listener);
            var watcherRef = typeof WeakRef === "function" ? new WeakRef(watcher) : null;
            call = runtime.stream(niva, "fs.watch", [path(value), !!opts.recursive], { onEvent: function (name, data) {
                var target = watcherRef && watcherRef.deref();
                if (!target || closed) return;
                if (name === "change") target.emit("change", data.eventType, data.filename === null ? null : opts.encoding === "buffer" ? Buffer.from(data.filename) : data.filename);
            } });
            var streamId = call.id;
            if (typeof runtime.registerResource === "function") owner = runtime.registerResource(watcher, function () { return runtime.cancelStream(streamId); });
            call.promise.catch(function (error) {
                var target = watcherRef && watcherRef.deref();
                if (!target || closed) return;
                if (target.listenerCount("error")) {
                    try { target.emit("error", runtime.nativeError(error)); } catch (_) { /* close watcher even if an error listener throws */ }
                }
                target.close();
            });
            if (opts.signal) {
                if (opts.signal.aborted)
                    watcher.close();
                else {
                    abortListener = function () { var target = watcherRef && watcherRef.deref(); if (target) target.close(); };
                    opts.signal.addEventListener("abort", abortListener, { once: true });
                }
            }
            return watcher;
        };
        module.promises = promises;
        return module;
    }
    runtime.createFsModule = createFsModule;
    runtime.fs = createFsModule();
})(globalThis);

(function (root: any) {
    "use strict";
    var key = Symbol.for("niva.node-compat.runtime");
    var runtime = root[key] || {};
  if (typeof runtime.resolveNiva === "function") return;
    function resolveNiva(niva) {
        var target = niva || root.Niva;
        if (!target) {
            throw new Error("Niva is not available. Load the adapter inside a Niva page.");
        }
        return target;
    }
    function api(niva, namespace) {
        var target = resolveNiva(niva);
        if (!target[namespace]) {
            throw new Error("Niva bridge namespace is unavailable: " + namespace);
        }
        return target[namespace];
    }
    function call(niva, method, args) {
        var target = resolveNiva(niva);
        if (!target.bridge || typeof target.bridge.call !== "function") {
            throw new Error("Niva.bridge.call is unavailable for " + method);
        }
        return Promise.resolve(target.bridge.call(method, args || [])).catch(function (error) {
            throw nativeError(error);
        });
    }
    function callSync(niva, method, args) {
        var target = resolveNiva(niva);
        if (!target.bridge || typeof target.bridge.callSync !== "function")
            throw bridgeError("Synchronous Niva bridge unavailable", "ERR_METHOD_NOT_IMPLEMENTED");
        try {
            return target.bridge.callSync(method, args || []);
        }
        catch (error) {
            throw nativeError(error);
        }
    }
    function callSyncAs(niva, apiName, method, args) {
        var target = resolveNiva(niva);
        var call = target.bridge && target.bridge[Symbol.for("niva.internal.bridge.callSyncApi")];
        if (typeof call === "function") {
            try {
                return call.call(target.bridge, apiName, method, args || []);
            }
            catch (error) {
                throw nativeError(error);
            }
        }
        return callSync(target, method, args || []);
    }
    function stream(niva, method, args, handlers) {
        var target = resolveNiva(niva);
        if (!target.bridge || typeof target.bridge.stream !== "function") {
            throw new Error("Niva.bridge.stream is unavailable for " + method + ". This API requires a local Niva page.");
        }
        return target.bridge.stream(method, args || [], handlers || {});
    }
    function streamRelated(niva, ownerCall, method, args, handlers) {
        var target = resolveNiva(niva);
        var call = target.bridge && target.bridge[Symbol.for("niva.internal.bridge.streamRelated")];
        if (typeof call !== "function") {
            throw bridgeError("Niva bridge cannot preserve the Native resource owner", "ERR_NIVA_STREAM_OWNER_CLOSED");
        }
        return call.call(target.bridge, ownerCall, method, args || [], handlers || {});
    }
    function streamSend(niva, id, data, end) {
        var target = resolveNiva(niva);
        if (!target.bridge || typeof target.bridge.streamSend !== "function") {
            throw new Error("Niva.bridge.streamSend is unavailable");
        }
        return target.bridge.streamSend(id, data, end);
    }
    function bridgeError(message, code?, extra?) {
        var error = new Error(message);
        if (code)
            error.code = code;
        if (extra && typeof extra === "object") {
            Object.keys(extra).forEach(function (name) { error[name] = extra[name]; });
        }
        return error;
    }
    function nativeError(value, fallback?) {
        var data = value && typeof value === "object" && value.data && typeof value.data === "object" ? value.data : undefined;
        var message = value && typeof value === "object" && typeof value.message === "string" ? value.message : value == null ? fallback || "Niva bridge call failed" : String(value);
        var lower = String(message).toLowerCase();
        var prefixedCode = /^([A-Z][A-Z_0-9]+):/.exec(message);
        var dataCode = data && typeof data.code === "string" ? data.code : undefined;
        var code = dataCode || (prefixedCode || [])[1] || (/cannot find module|cannot find package/.test(lower) ? "MODULE_NOT_FOUND" : /no such file|not found/.test(lower) ? "ENOENT"
                : /permission denied|access is denied/.test(lower) ? "EACCES"
                    : /already exists/.test(lower) ? "EEXIST"
                        : /not a directory/.test(lower) ? "ENOTDIR"
                                : /is a directory/.test(lower) ? "EISDIR"
                                    : value && typeof value === "object" && value.code ? "NIVA_BRIDGE_ERROR" : undefined);
        if (value instanceof Error) {
            var native = value as any;
            if (dataCode || (prefixedCode && native.code === "NIVA_BRIDGE_ERROR")) {
                if (native.code !== dataCode && native.bridgeCode === undefined) native.bridgeCode = native.code;
                native.code = code;
            } else if (!native.code && code) native.code = code;
            if (data && data.errno !== undefined && native.errno === undefined) native.errno = data.errno;
            if (data && data.syscall !== undefined && native.syscall === undefined) native.syscall = data.syscall;
            if (data && data.path !== undefined && native.path === undefined) native.path = data.path;
            return native;
        }
        if (value && typeof value === "object") {
            var error: any = bridgeError(message, code);
            error.bridgeCode = value.bridgeCode !== undefined ? value.bridgeCode : value.code;
            if (data && data.errno !== undefined) error.errno = data.errno;
            if (data && data.syscall !== undefined) error.syscall = data.syscall;
            if (data && data.path !== undefined) error.path = data.path;
            error.bridgeResult = value;
            return error;
        }
        return bridgeError(message, code || "NIVA_BRIDGE_ERROR");
    }
    function createEmitter() {
        var listeners = Object.create(null);
        var emitter = {
            on: function (event, listener) {
                if (typeof listener !== "function")
                    throw new TypeError("listener must be a function");
                (listeners[event] || (listeners[event] = [])).push(listener);
                return emitter;
            },
            addListener: function (event, listener) { return emitter.on(event, listener); },
            once: function (event, listener) {
                var wrapped = function () {
                    emitter.off(event, wrapped);
                    return listener.apply(emitter, arguments);
                };
                (wrapped as any).listener = listener;
                return emitter.on(event, wrapped);
            },
            off: function (event, listener) {
                var current = listeners[event];
                if (current)
                    listeners[event] = current.filter(function (item: any) {
                        return item !== listener && item.listener !== listener;
                    });
                return emitter;
            },
            removeListener: function (event, listener) { return emitter.off(event, listener); },
            removeAllListeners: function (event) {
                if (event === undefined)
                    listeners = Object.create(null);
                else
                    delete listeners[event];
                return emitter;
            },
            listeners: function (event) {
                return (listeners[event] || []).map(function (item) { return item.listener || item; });
            },
            listenerCount: function (event) { return (listeners[event] || []).length; },
            emit: function (event) {
                var args = Array.prototype.slice.call(arguments, 1);
                var current = (listeners[event] || []).slice();
                current.forEach(function (listener) { listener.apply(emitter, args); });
                return current.length > 0;
            },
        };
        return emitter;
    }
    runtime.resolveNiva = resolveNiva;
    runtime.api = api;
    runtime.call = call;
    runtime.callSync = callSync;
    runtime.callSyncAs = callSyncAs;
    runtime.stream = stream;
    runtime.streamRelated = streamRelated;
    runtime.streamSend = streamSend;
    runtime.bridgeError = bridgeError;
    runtime.nativeError = nativeError;
    runtime.createEmitter = createEmitter;
    root[key] = runtime;
})(globalThis);

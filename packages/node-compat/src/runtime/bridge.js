(function (root) {
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
        if (!target.api || !target.api[namespace]) {
            throw new Error("Niva bridge namespace is unavailable: " + namespace);
        }
        return target.api[namespace];
    }
    function call(niva, method, args) {
        var target = resolveNiva(niva);
        if (typeof target.call !== "function") {
            throw new Error("Niva.call is unavailable for " + method);
        }
        return target.call(method, args || []);
    }
    function callSync(niva, method, args) {
        var target = resolveNiva(niva);
        if (typeof target.callSync !== "function")
            throw bridgeError("Synchronous Niva bridge unavailable", "ERR_METHOD_NOT_IMPLEMENTED");
        try {
            return target.callSync(method, args || []);
        }
        catch (error) {
            throw nativeError(error);
        }
    }
    function stream(niva, method, args, handlers) {
        var target = resolveNiva(niva);
        if (typeof target.stream !== "function") {
            throw new Error("Niva.stream is unavailable for " + method + ". This API requires a local Niva page.");
        }
        return target.stream(method, args || [], handlers || {});
    }
    function bridgeError(message, code, extra) {
        var error = new Error(message);
        if (code)
            error.code = code;
        if (extra && typeof extra === "object") {
            Object.keys(extra).forEach(function (name) { error[name] = extra[name]; });
        }
        return error;
    }
    function nativeError(value, fallback) {
        if (value instanceof Error)
            return value;
        if (value && typeof value === "object") {
            var message = value.message || fallback || "Niva bridge call failed";
            var lower = String(message).toLowerCase();
            var code = value.data && value.data.code || (/^(E[A-Z_0-9]+):/.exec(message) || [])[1] || (/no such file|not found/.test(lower) ? "ENOENT"
                : /permission denied|access is denied/.test(lower) ? "EACCES"
                    : /already exists/.test(lower) ? "EEXIST"
                        : /not a directory/.test(lower) ? "ENOTDIR"
                            : /is a directory/.test(lower) ? "EISDIR"
                                : value.code ? "NIVA_BRIDGE_ERROR" : undefined);
            var error = bridgeError(message, code);
            error.bridgeCode = value.code;
            error.bridgeResult = value;
            return error;
        }
        return bridgeError(value == null ? (fallback || "Niva bridge call failed") : String(value), "NIVA_BRIDGE_ERROR");
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
                wrapped.listener = listener;
                return emitter.on(event, wrapped);
            },
            off: function (event, listener) {
                var current = listeners[event];
                if (current)
                    listeners[event] = current.filter(function (item) {
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
    runtime.stream = stream;
    runtime.bridgeError = bridgeError;
    runtime.nativeError = nativeError;
    runtime.createEmitter = createEmitter;
    root[key] = runtime;
})(globalThis);

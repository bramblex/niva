(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createEventsModule === "function") return;
  var errorMonitor = Symbol.for("events.errorMonitor");
  var abortError = function (signal) {
    var error = new Error("The operation was aborted");
    error.name = "AbortError";
    error.code = "ABORT_ERR";
    if (signal && "reason" in signal) error.cause = signal.reason;
    return error;
  };

  function invalidArgType(name, expected, value) {
    var received;
    if (value === null) received = "Received null";
    else if (typeof value === "function") received = "Received function " + (value.name || "");
    else {
      var type = typeof value;
      var printable = type === "string" ? "'" + value + "'" : String(value);
      received = "Received type " + type + " (" + printable + ")";
    }
    var error = new TypeError("The \"" + name + "\" argument must be " + expected + ". " + received);
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function validAbortSignal(signal) {
    return signal === undefined || (signal !== null && typeof signal === "object" &&
      typeof signal.aborted === "boolean" && typeof signal.addEventListener === "function" &&
      typeof signal.removeEventListener === "function");
  }

  function addEventListener(emitter, event, listener, options) {
    if (typeof emitter.on === "function") return emitter.on(event, listener);
    return emitter.addEventListener(event, listener, options);
  }

  function removeEventListener(emitter, event, listener) {
    if (typeof emitter.removeListener === "function") return emitter.removeListener(event, listener);
    return emitter.removeEventListener(event, listener);
  }

  function abortObserver(signal) {
    // AbortSignal.any propagates via the platform abort algorithm, so a listener
    // that stops the original abort event cannot suppress this observer. Older
    // WebViews fall back to a capture listener on the original signal.
    if (typeof root.AbortSignal === "function" && typeof root.AbortSignal.any === "function") {
      try {
        return root.AbortSignal.any([signal]);
      } catch (_) {}
    }
    return signal;
  }

  function EventEmitter(options) {
    if (!(this instanceof EventEmitter)) throw new TypeError("Class constructor EventEmitter cannot be invoked without 'new'");
    options = options || {};
    if (typeof options !== "object") throw new TypeError("options must be an object");
    this._events = Object.create(null);
    this._maxListeners = undefined;
    this._captureRejections = options.captureRejections === undefined
      ? EventEmitter.captureRejections
      : !!options.captureRejections;
  }

  var defaultMaxListeners = 10;
  Object.defineProperty(EventEmitter, "defaultMaxListeners", {
    enumerable: true,
    get: function () { return defaultMaxListeners; },
    set: function (value) {
      if (typeof value !== "number" || Number.isNaN(value) || value < 0) {
        throw new RangeError("defaultMaxListeners must be a non-negative number");
      }
      defaultMaxListeners = value;
    },
  });
  EventEmitter.captureRejections = false;
  EventEmitter.errorMonitor = errorMonitor;
  EventEmitter.prototype._events = undefined;
  EventEmitter.prototype._maxListeners = undefined;

  EventEmitter.prototype.setMaxListeners = function (count) {
    if (typeof count !== "number" || Number.isNaN(count) || count < 0) {
      throw new RangeError("The value of \"n\" is out of range. It must be a non-negative number.");
    }
    this._maxListeners = count;
    return this;
  };

  EventEmitter.prototype.getMaxListeners = function () {
    return this._maxListeners === undefined ? (EventEmitter as any).defaultMaxListeners : this._maxListeners;
  };

  EventEmitter.prototype.emit = function (event, ..._args: any[]) {
    var args = Array.prototype.slice.call(arguments, 1);
    var current = this._events[event];
    if (event === "error") {
      var monitor = this._events[errorMonitor];
      if (monitor) invoke(this, errorMonitor, monitor, args);
      if (!current) {
        var error = args[0];
        if (error instanceof Error) throw error;
        var unhandled = new Error("Unhandled error." + (error === undefined ? "" : " (" + String(error) + ")"));
        unhandled.context = error;
        throw unhandled;
      }
    }
    if (!current) return false;
    invoke(this, event, current, args);
    return true;
  };

  EventEmitter.prototype.addListener = function (event, listener) { return add(this, event, listener, false, false); };
  EventEmitter.prototype.on = EventEmitter.prototype.addListener;
  EventEmitter.prototype.prependListener = function (event, listener) { return add(this, event, listener, true, false); };
  EventEmitter.prototype.once = function (event, listener) { return add(this, event, listener, false, true); };
  EventEmitter.prototype.prependOnceListener = function (event, listener) { return add(this, event, listener, true, true); };

  EventEmitter.prototype.removeListener = function (event, listener) {
    if (typeof listener !== "function") throw new TypeError("listener must be a function");
    var list = this._events[event];
    if (!list) return this;
    var index = -1;
    for (var i = list.length - 1; i >= 0; i -= 1) {
      if (list[i] === listener || list[i].listener === listener) { index = i; break; }
    }
    if (index < 0) return this;
    var removed = list.splice(index, 1)[0];
    if (list.length === 0) delete this._events[event];
    if (event !== "removeListener") this.emit("removeListener", event, removed.listener || removed);
    return this;
  };

  EventEmitter.prototype.off = EventEmitter.prototype.removeListener;

  EventEmitter.prototype.removeAllListeners = function (event) {
    if (event !== undefined) {
      var list = this._events[event];
      if (!list) return this;
      if (event === "removeListener" || !this._events.removeListener) {
        delete this._events[event];
        return this;
      }
      while (list.length) this.removeListener(event, list[list.length - 1]);
      delete this._events[event];
      return this;
    }
    if (!this._events.removeListener) {
      this._events = Object.create(null);
      return this;
    }
    Reflect.ownKeys(this._events).forEach(function (name) {
      if (name !== "removeListener") this.removeAllListeners(name);
    }, this);
    this.removeAllListeners("removeListener");
    return this;
  };

  EventEmitter.prototype.listeners = function (event) {
    return (this._events[event] || []).map(function (listener) { return listener.listener || listener; });
  };

  EventEmitter.prototype.rawListeners = function (event) {
    return (this._events[event] || []).slice();
  };

  EventEmitter.prototype.listenerCount = function (event, listener) {
    var list = this._events[event] || [];
    if (listener === undefined) return list.length;
    return list.filter(function (item) { return item === listener || item.listener === listener; }).length;
  };

  EventEmitter.prototype.eventNames = function () { return Reflect.ownKeys(this._events); };
  EventEmitter.listenerCount = function (emitter, event) { return emitter.listenerCount(event); };

  function add(emitter, event, listener, prepend, once) {
    if (typeof listener !== "function") throw new TypeError("listener must be a function");
    if (emitter._events.newListener) emitter.emit("newListener", event, listener.listener || listener);
    var stored = listener;
    if (once) {
      stored = function () {
        emitter.removeListener(event, stored);
        return listener.apply(emitter, arguments);
      };
      stored.listener = listener;
    }
    var list = emitter._events[event];
    if (!list) {
      emitter._events[event] = [stored];
    } else {
      if (prepend) list.unshift(stored);
      else list.push(stored);
      warnIfNeeded(emitter, event, list);
    }
    return emitter;
  }

  function warnIfNeeded(emitter, event, list) {
    var max = emitter.getMaxListeners();
    if (!list.warned && max > 0 && list.length > max) {
      list.warned = true;
      if (typeof console !== "undefined" && typeof console.warn === "function") {
        console.warn("Possible EventEmitter memory leak detected. " + list.length + " " + String(event) + " listeners added.");
      }
    }
  }

  function invoke(emitter, event, list, args) {
    var listeners = typeof list === "function" ? [list] : list.slice();
    listeners.forEach(function (listener) {
      var result = listener.apply(emitter, args);
      if (!emitter._captureRejections || event === "error" || !result || typeof result.then !== "function") return;
      Promise.resolve(result).catch(function (error) {
        enqueue(function () { emitter.emit("error", error); });
      });
    });
  }

  function enqueue(callback) {
    if (typeof root.queueMicrotask === "function") root.queueMicrotask(callback);
    else Promise.resolve().then(callback);
  }

  async function once(emitter, event, options) {
    if (options === undefined) options = {};
    else if (options === null || typeof options !== "object" || Array.isArray(options)) {
      throw invalidArgType("options", "of type object", options);
    }
    var signal = options.signal;
    if (!validAbortSignal(signal)) {
      throw invalidArgType("options.signal", "an instance of AbortSignal", signal);
    }
    if (signal && signal.aborted) throw abortError(signal);

    var isEmitter = !!emitter && typeof emitter.on === "function" && typeof emitter.removeListener === "function";
    var isEventTarget = !!emitter && typeof emitter.addEventListener === "function" &&
      typeof emitter.removeEventListener === "function";
    if (!isEmitter && !isEventTarget) {
      throw invalidArgType("emitter", "an instance of EventEmitter or EventTarget", emitter);
    }

    var observedSignal = signal ? abortObserver(signal) : null;
    var trackOriginalSignal = !!signal && observedSignal !== signal;
    return new Promise(function (resolve, reject) {
      var settled = false;
      function cleanup() {
        removeEventListener(emitter, event, onEvent);
        if (isEmitter && event !== "error") removeEventListener(emitter, "error", onError);
        if (observedSignal) observedSignal.removeEventListener("abort", onAbort, { capture: true });
        if (trackOriginalSignal) signal.removeEventListener("abort", onAbort, { capture: true });
      }
      function finish(callback, value) {
        if (settled) return;
        settled = true;
        cleanup();
        callback(value);
      }
      function onEvent() { finish(resolve, Array.prototype.slice.call(arguments)); }
      function onError(error) { finish(reject, error); }
      function onAbort() { finish(reject, abortError(signal)); }

      if (isEmitter) {
        if (typeof emitter.once === "function") emitter.once(event, onEvent);
        else emitter.on(event, onEvent);
        if (event !== "error" && typeof emitter.once === "function") emitter.once("error", onError);
        else if (event !== "error" && typeof emitter.on === "function") emitter.on("error", onError);
      } else {
        emitter.addEventListener(event, onEvent, { once: true });
      }
      if (observedSignal) {
        if (trackOriginalSignal) signal.addEventListener("abort", onAbort, { once: true, capture: true });
        observedSignal.addEventListener("abort", onAbort, { once: true, capture: true });
      }
    });
  }

  function on(emitter, event, options) {
    if (options === undefined) options = {};
    else if (options === null || typeof options !== "object" || Array.isArray(options)) {
      throw invalidArgType("options", "of type object", options);
    }
    var signal = options.signal;
    if (!validAbortSignal(signal)) {
      throw invalidArgType("options.signal", "an instance of AbortSignal", signal);
    }
    var isEmitter = !!emitter && typeof emitter.on === "function" && typeof emitter.removeListener === "function";
    var isEventTarget = !!emitter && typeof emitter.addEventListener === "function" &&
      typeof emitter.removeEventListener === "function";
    if (!isEmitter && !isEventTarget) {
      throw invalidArgType("emitter", "an instance of EventEmitter or EventTarget", emitter);
    }
    var closeEvents = options.close === undefined ? [] : options.close;
    if (!Array.isArray(closeEvents)) throw new TypeError("options.close must be an array");
    var highWaterMark = options.highWaterMark === undefined ? Number.MAX_SAFE_INTEGER : Number(options.highWaterMark);
    var lowWaterMark = options.lowWaterMark === undefined ? 0 : Number(options.lowWaterMark);
    if (!Number.isFinite(highWaterMark) || highWaterMark < 1 || !Number.isFinite(lowWaterMark) || lowWaterMark < 0 || lowWaterMark > highWaterMark) {
      throw new RangeError("highWaterMark and lowWaterMark must be non-negative, ordered integers");
    }
    var values = [];
    var waiters = [];
    var ended = false;
    var failure;
    var paused = false;
    var iterator;
    var observedSignal = signal ? abortObserver(signal) : null;
    var trackOriginalSignal = !!signal && observedSignal !== signal;

    function cleanup() {
      removeEventListener(emitter, event, eventHandler);
      if (isEmitter && event !== "error") removeEventListener(emitter, "error", errorHandler);
      closeEvents.forEach(function (closeEvent) { removeEventListener(emitter, closeEvent, closeHandler); });
      if (observedSignal) observedSignal.removeEventListener("abort", abortHandler, { capture: true });
      if (trackOriginalSignal) signal.removeEventListener("abort", abortHandler, { capture: true });
      if (paused && typeof emitter.resume === "function") emitter.resume();
      paused = false;
    }
    function settleEnd(error?) {
      if (ended) return;
      ended = true;
      failure = error;
      cleanup();
      while (waiters.length) {
        var waiter = waiters.shift();
        if (error) waiter.reject(error);
        else waiter.resolve({ value: undefined, done: true });
      }
    }
    function eventHandler() {
      if (closeEvents.indexOf(event) >= 0) { settleEnd(); return; }
      var value = Array.prototype.slice.call(arguments);
      if (waiters.length) waiters.shift().resolve({ value: value, done: false });
      else values.push(value);
      if (!paused && values.length >= highWaterMark && typeof emitter.pause === "function") {
        emitter.pause();
        paused = true;
      }
    }
    function errorHandler(error) { settleEnd(error); }
    function closeHandler() { settleEnd(); }
    function abortHandler() { settleEnd(abortError(signal)); }
    function resumeIfNeeded() {
      if (paused && values.length <= lowWaterMark && typeof emitter.resume === "function") {
        emitter.resume();
        paused = false;
      }
    }
    iterator = {
      next: function () {
        if (values.length) {
          var value = values.shift();
          resumeIfNeeded();
          return Promise.resolve({ value: value, done: false });
        }
        if (failure) return Promise.reject(failure);
        if (ended) return Promise.resolve({ value: undefined, done: true });
        return new Promise(function (resolve, reject) { waiters.push({ resolve: resolve, reject: reject }); });
      },
      return: function () {
        settleEnd();
        return Promise.resolve({ value: undefined, done: true });
      },
      throw: function (error) {
        settleEnd(error);
        return Promise.reject(error);
      },
      [Symbol.asyncIterator]: function () { return this; },
    };

    if (signal && signal.aborted) {
      settleEnd(abortError(signal));
    } else {
      addEventListener(emitter, event, eventHandler, { once: true });
      if (isEmitter && event !== "error") addEventListener(emitter, "error", errorHandler, { once: true });
      closeEvents.forEach(function (closeEvent) {
        addEventListener(emitter, closeEvent, closeHandler, { once: true });
      });
      if (observedSignal) {
        if (trackOriginalSignal) signal.addEventListener("abort", abortHandler, { once: true, capture: true });
        observedSignal.addEventListener("abort", abortHandler, { once: true, capture: true });
      }
    }
    return iterator;
  }

  EventEmitter.EventEmitter = EventEmitter;
  EventEmitter.errorMonitor = errorMonitor;
  EventEmitter.once = once;
  EventEmitter.on = on;
  runtime.createEventsModule = function () { return EventEmitter; };
  runtime.events = EventEmitter;
})(globalThis);

(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createTimersModule === "function") return;
  var MAX_DELAY = 0x7fffffff;
  // Capture browser primitives before injectCommonJs can install the Node-shaped
  // globals back onto the same window object.
  var nativeSetTimeout = root.setTimeout && root.setTimeout.bind(root);
  var nativeClearTimeout = root.clearTimeout && root.clearTimeout.bind(root);
  var nativeSetInterval = root.setInterval && root.setInterval.bind(root);
  var nativeClearInterval = root.clearInterval && root.clearInterval.bind(root);
  var nativeSetImmediate = root.setImmediate && root.setImmediate.bind(root);
  var nativeClearImmediate = root.clearImmediate && root.clearImmediate.bind(root);
  function normalizeDelay(delay) {
    var value = Number(delay);
    if (!Number.isFinite(value) || value < 1 || value > MAX_DELAY) return 1;
    return Math.trunc(value);
  }

  function validateCallback(callback) {
    if (typeof callback !== "function") {
      throw new TypeError("The \"callback\" argument must be of type function");
    }
  }

  function Timeout(callback, delay, args, repeat): any {
    this._callback = callback;
    this._args = args;
    this._delay = delay;
    this._repeat = repeat;
    this._refed = true;
    this._active = false;
    this._id = undefined;
    this._schedule();
  }

  Timeout.prototype._schedule = function () {
    var self = this;
    this._active = true;
    var invoke = function () {
      if (!self._repeat) self._active = false;
      self._callback.apply(self, self._args);
    };
    this._id = this._repeat
      ? nativeSetInterval(invoke, this._delay)
      : nativeSetTimeout(invoke, this._delay);
  };

  Timeout.prototype.ref = function () { this._refed = true; return this; };
  Timeout.prototype.unref = function () { this._refed = false; return this; };
  Timeout.prototype.hasRef = function () { return this._refed; };
  Timeout.prototype.refresh = function () {
    if (this._active) this._repeat ? nativeClearInterval(this._id) : nativeClearTimeout(this._id);
    this._schedule();
    return this;
  };
  Timeout.prototype.close = function () {
    clearTimeoutCompat(this);
    return this;
  };
  Timeout.prototype[Symbol.toPrimitive] = function () { return this._id; };
  if (Symbol.dispose) Timeout.prototype[Symbol.dispose] = function () { clearTimeoutCompat(this); };

  function Immediate(callback, args): any {
    this._callback = callback;
    this._args = args;
    this._refed = true;
    this._active = true;
    this._id = undefined;
    var self = this;
    var invoke = function () {
      self._active = false;
      self._callback.apply(self, self._args);
    };
    this._native = typeof nativeSetImmediate === "function";
    this._id = this._native ? nativeSetImmediate(invoke) : nativeSetTimeout(invoke, 0);
  }
  Immediate.prototype.ref = function () { this._refed = true; return this; };
  Immediate.prototype.unref = function () { this._refed = false; return this; };
  Immediate.prototype.hasRef = function () { return this._refed; };
  Immediate.prototype[Symbol.toPrimitive] = function () { return this._id; };
  if (Symbol.dispose) Immediate.prototype[Symbol.dispose] = function () { clearImmediateCompat(this); };

  function setTimeoutCompat(callback, delay) {
    validateCallback(callback);
    return Reflect.construct(Timeout as any, [callback, normalizeDelay(delay), Array.prototype.slice.call(arguments, 2), false]);
  }
  function setIntervalCompat(callback, delay) {
    validateCallback(callback);
    return Reflect.construct(Timeout as any, [callback, normalizeDelay(delay), Array.prototype.slice.call(arguments, 2), true]);
  }
  function clearTimeoutCompat(handle: any) {
    if (handle instanceof Timeout) {
      var timeout: any = handle;
      if (!timeout._active) return;
      timeout._active = false;
      if (timeout._repeat) nativeClearInterval(timeout._id);
      else nativeClearTimeout(timeout._id);
      return;
    }
    nativeClearTimeout(handle);
  }
  function clearIntervalCompat(handle) {
    clearTimeoutCompat(handle);
  }
  function setImmediateCompat(callback) {
    validateCallback(callback);
    return Reflect.construct(Immediate as any, [callback, Array.prototype.slice.call(arguments, 1)]);
  }
  function clearImmediateCompat(handle: any) {
    if (handle instanceof Immediate) {
      var immediate: any = handle;
      if (!immediate._active) return;
      immediate._active = false;
      if (immediate._native && typeof nativeClearImmediate === "function") nativeClearImmediate(immediate._id);
      else nativeClearTimeout(immediate._id);
      return;
    }
    if (typeof nativeClearImmediate === "function") nativeClearImmediate(handle);
    else nativeClearTimeout(handle);
  }

  function abortError(signal) {
    var error = new Error("The operation was aborted");
    error.name = "AbortError";
    error.code = "ABORT_ERR";
    if (signal && signal.reason !== undefined) error.cause = signal.reason;
    return error;
  }

  function promiseOptions(options?) {
    if (options === undefined) return {};
    if (options === null || typeof options !== "object") throw new TypeError("The \"options\" argument must be an object");
    if (options.signal !== undefined && (!options.signal || typeof options.signal.addEventListener !== "function")) {
      throw new TypeError("The \"signal\" option must be an AbortSignal");
    }
    if (options.ref !== undefined && typeof options.ref !== "boolean") throw new TypeError("The \"ref\" option must be a boolean");
    return options;
  }

  function timeoutPromise(delay, value, options) {
    options = promiseOptions(options);
    var signal = options.signal;
    if (signal && signal.aborted) return Promise.reject(abortError(signal));
    return new Promise(function (resolve, reject) {
      var timer;
      function cleanup() { if (signal) signal.removeEventListener("abort", onAbort); }
      function onAbort() { clearTimeoutCompat(timer); cleanup(); reject(abortError(signal)); }
      timer = setTimeoutCompat(function () { cleanup(); resolve(value); }, delay);
      if (options.ref === false) timer.unref();
      if (signal) signal.addEventListener("abort", onAbort, { once: true });
    });
  }

  function immediatePromise(value, options?) {
    options = promiseOptions(options);
    var signal = options.signal;
    if (signal && signal.aborted) return Promise.reject(abortError(signal));
    return new Promise(function (resolve, reject) {
      var immediate: any;
      function cleanup() { if (signal) signal.removeEventListener("abort", onAbort); }
      function onAbort() { clearImmediateCompat(immediate); cleanup(); reject(abortError(signal)); }
      immediate = setImmediateCompat(function () { cleanup(); resolve(value); });
      if (options.ref === false) immediate.unref();
      if (signal) signal.addEventListener("abort", onAbort, { once: true });
    });
  }

  function intervalIterator(delay, value, options) {
    options = promiseOptions(options);
    var signal = options.signal;
    var values = [];
    var waiters = [];
    var ended = false;
    var failure;
    var timer: any;
    function cleanup() {
      if (timer) clearIntervalCompat(timer);
      timer = undefined;
      if (signal) signal.removeEventListener("abort", onAbort);
    }
    function terminate(error?) {
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
    function onAbort() { terminate(abortError(signal)); }
    var iterator = {
      next: function () {
        if (values.length) return Promise.resolve({ value: values.shift(), done: false });
        if (failure) return Promise.reject(failure);
        if (ended) return Promise.resolve({ value: undefined, done: true });
        return new Promise(function (resolve, reject) { waiters.push({ resolve: resolve, reject: reject }); });
      },
      return: function () {
        terminate();
        return Promise.resolve({ value: undefined, done: true });
      },
      throw: function (error) {
        terminate(error);
        return Promise.reject(error);
      },
      [Symbol.asyncIterator]: function () { return this; },
    };
    if (signal && signal.aborted) {
      terminate(abortError(signal));
      return iterator;
    }
    timer = setIntervalCompat(function () {
      if (ended) return;
      if (waiters.length) waiters.shift().resolve({ value: value, done: false });
      else values.push(value);
    }, delay);
    if (options.ref === false) timer.unref();
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    return iterator;
  }

  function TimerScheduler() {
    throw runtime.bridgeError("Illegal constructor", "ERR_ILLEGAL_CONSTRUCTOR");
  }
  var scheduler = Object.create(TimerScheduler.prototype);
  scheduler.wait = function (delay, options) { return timeoutPromise(delay, undefined, options); };
  scheduler.yield = function () { return immediatePromise(undefined); };

  var promises = {
    setTimeout: timeoutPromise,
    setImmediate: immediatePromise,
    setInterval: intervalIterator,
    scheduler: scheduler,
  };

  var module = {
    setTimeout: setTimeoutCompat,
    clearTimeout: clearTimeoutCompat,
    setInterval: setIntervalCompat,
    clearInterval: clearIntervalCompat,
    setImmediate: setImmediateCompat,
    clearImmediate: clearImmediateCompat,
    Timeout: Timeout,
    Immediate: Immediate,
    promises: promises,
  };
  runtime.createTimersModule = function () { return module; };
  runtime.timers = module;
})(globalThis);

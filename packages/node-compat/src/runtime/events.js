(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  var errorMonitor = Symbol.for("events.errorMonitor");

  function EventEmitter(options) {
    if (!(this instanceof EventEmitter)) throw new TypeError("Class constructor EventEmitter cannot be invoked without 'new'");
    if (options && options.captureRejections) throw new TypeError("EventEmitter captureRejections is not supported by node-compat");
    this._events = Object.create(null);
    this._maxListeners = undefined;
  }

  var defaultMaxListeners = 10;
  Object.defineProperty(EventEmitter, "defaultMaxListeners", {
    enumerable: true,
    get: function () { return defaultMaxListeners; },
    set: function (value) {
      if (!Number.isInteger(value) || value < 0) throw new RangeError("defaultMaxListeners must be a non-negative integer");
      defaultMaxListeners = value;
    },
  });
  EventEmitter.errorMonitor = errorMonitor;
  EventEmitter.prototype._events = undefined;
  EventEmitter.prototype._maxListeners = undefined;

  EventEmitter.prototype.setMaxListeners = function (count) {
    if (!Number.isInteger(count) || count < 0) throw new RangeError("The value of \"n\" is out of range. It must be a non-negative integer.");
    this._maxListeners = count;
    return this;
  };

  EventEmitter.prototype.getMaxListeners = function () {
    return this._maxListeners === undefined ? EventEmitter.defaultMaxListeners : this._maxListeners;
  };

  EventEmitter.prototype.emit = function (event) {
    var args = Array.prototype.slice.call(arguments, 1);
    var current = this._events[event];
    if (event === "error") {
      var monitor = this._events[errorMonitor];
      if (monitor) invoke(monitor, this, args);
      if (!current) {
        var error = args[0];
        if (error instanceof Error) throw error;
        var unhandled = new Error("Unhandled error." + (error === undefined ? "" : " (" + String(error) + ")"));
        unhandled.context = error;
        throw unhandled;
      }
    }
    if (!current) return false;
    invoke(current, this, args);
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
    if (event === undefined) {
      var names = Reflect.ownKeys(this._events);
      for (var i = names.length - 1; i >= 0; i -= 1) this.removeAllListeners(names[i]);
      this._events = Object.create(null);
      return this;
    }
    var list = this._events[event];
    if (!list) return this;
    if (event === "removeListener" || !this._events.removeListener) {
      delete this._events[event];
      return this;
    }
    while (list.length) this.removeListener(event, list[list.length - 1]);
    delete this._events[event];
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
      if (!list.warned && emitter.getMaxListeners() > 0 && list.length > emitter.getMaxListeners()) {
        list.warned = true;
        if (typeof console !== "undefined" && typeof console.warn === "function") {
          console.warn("Possible EventEmitter memory leak detected. " + list.length + " " + String(event) + " listeners added.");
        }
      }
    }
    return emitter;
  }

  function invoke(list, receiver, args) {
    if (typeof list === "function") return list.apply(receiver, args);
    list.slice().forEach(function (listener) { listener.apply(receiver, args); });
  }

  var module = { EventEmitter: EventEmitter, errorMonitor: errorMonitor };
  Object.defineProperty(module, "defaultMaxListeners", {
    enumerable: true,
    get: function () { return EventEmitter.defaultMaxListeners; },
    set: function (value) { EventEmitter.defaultMaxListeners = value; },
  });
  runtime.createEventsModule = function () { return module; };
  runtime.events = module;
})(globalThis);

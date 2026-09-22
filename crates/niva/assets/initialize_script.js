
(function () {
  // Disable drag and open file
  window.addEventListener("dragover", function (ev) { ev.preventDefault(); }, false);

  var Niva = {};

  // === Event ===
  var eventListeners = {};

  function addEventListener(event, listener) {
    if (!eventListeners[event]) {
      eventListeners[event] = [];
    }
    eventListeners[event].push(listener);
  }

  function removeEventListener(event, listener) {
    if (!eventListeners[event]) {
      return;
    }
    var listeners = eventListeners[event];
    var newListeners = [];
    for (var i = 0; i < listeners.length; i++) {
      if (listeners[i] !== listener) {
        newListeners.push(listeners[i]);
      }
    }
    eventListeners[event] = newListeners;
  }

  function removeAllEventListeners(event) {
    if (!eventListeners[event]) {
      return;
    }
    eventListeners[event] = [];
  }

  function emit(event, data) {
    setTimeout(function () {
      var keys = [event, event.split('.')[0] + '.*', '*'];

      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];

        if (eventListeners[key]) {
          var listeners = eventListeners[key];
          for (var j = 0; j < listeners.length; j++) {
            listeners[j](event, data);
          }
        }
      }

    }, 0);
  }

  Niva.addEventListener = addEventListener;
  Niva.removeEventListener = removeEventListener;
  Niva.removeAllEventListeners = removeAllEventListeners;
  EventListener = removeEventListener;
  Niva.__emit__ = emit;

  // === API Call over our own WebSocket ===
  // The native side appends a bootstrap snippet after this script defining:
  //   window.__niva_ws_url / window.__niva_window_id / window.__niva_token
  var getNextCallbackId = (function () {
    var callbackId = 0;
    return function () {
      if (callbackId >= Number.MAX_SAFE_INTEGER) {
        callbackId = 0;
      }
      return ++callbackId;
    }
  })();

  var callbacks = {};
  var sendQueue = [];
  var socket = null;

  function flushQueue() {
    if (socket && socket.readyState === WebSocket.OPEN) {
      while (sendQueue.length > 0) {
        socket.send(sendQueue.shift());
      }
    }
  }

  function connectSocket() {
    var url = window.__niva_ws_url;
    if (!url || typeof WebSocket === 'undefined') {
      return;
    }
    try {
      socket = new WebSocket(url + "?token=" + encodeURIComponent(window.__niva_token || ""));
    } catch (e) {
      socket = null;
      return;
    }
    socket.addEventListener("open", function () {
      try {
        socket.send(JSON.stringify(["hello", window.__niva_window_id || 0]));
      } catch (e) {}
      flushQueue();
    });
    socket.addEventListener("message", function (ev) {
      try {
        var msg = JSON.parse(ev.data);
        // ["event", name, payload] — same envelope the native side used
        // to evaluate with.
        if (msg && msg[0] === "event") {
          emit(msg[1], msg[2]);
        }
      } catch (e) {}
    });
    socket.addEventListener("close", function () {
      socket = null;
      // Reconnect (e.g. dev-server HMR should not kill the bridge).
      setTimeout(connectSocket, 1000);
    });
  }

  function call(method, args) {
    var callbackId = getNextCallbackId();
    sendQueue.push(JSON.stringify([
      callbackId,
      method,
      args
    ]));
    flushQueue();

    var _resolve, _reject;
    var promise = new Promise((resolve, reject) => {
      _resolve = resolve;
      _reject = reject;
    })
    promise.resolve = _resolve;
    promise.reject = _reject;

    callbacks[callbackId] = promise;
    return promise;
  }

  function resolve(response) {
    setTimeout(function () {
      var callbackId = response[0];
      var code = response[1];
      var data = response[3];

      var promise = callbacks[callbackId];
      if (promise) {
        if (code === 0) {
          promise.resolve(data);
        } else {
          promise.reject(response);
        }
        delete callbacks[callbackId];
      }
    }, 0);
  }

  if (typeof Proxy !== 'undefined') {
    Niva.api = new Proxy({}, {
      get: function (_, namespace) {
        return new Proxy({}, {
          get: function (_, method) {
            return function () {
              return Niva.call(namespace + '.' + method, Array.prototype.slice.call(arguments))
            }
          }
        })
      }
    });
  } else {
    console.log('Proxy not supported, please use Niva.call instead');
  }

  Niva.call = call;
  Niva.__resolve__ = resolve;

  Niva.addEventListener('ipc.callback', function (event, response) {
    Niva.__resolve__(response);
  });

  delete window.close;
  delete window.open;

  // === Tauri API ===
  window.Niva = Niva;
  console.log('Niva loaded');

  connectSocket();

  (function docReady(func) {
    if (document.readyState === "complete" || document.readyState === "interactive") {
      setTimeout(func);
    } else {
      document.addEventListener("DOMContentLoaded", func);
    }
  })(function () {
  });

}());

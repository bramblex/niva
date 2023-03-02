
(function () {
  // Disable drag and open file
  window.addEventListener("dragover", function (ev) { ev.preventDefault(); }, false);

  var TauriLite = {};

  // === API Call ===
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

  function call(namespace, method, data) {
    var callbackId = getNextCallbackId();
    window.ipc.postMessage(JSON.stringify({
      namespace: namespace,
      method: method,
      data: data,
      callback_id: callbackId
    }));

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
      var promise = callbacks[response.callback_id];
      if (promise) {
        if (response.code === 0) {
          promise.resolve(response.data);
        } else {
          promise.reject(response);
        }
        delete callbacks[response.callback_id];
      }
    }, 0);
  }

  if (typeof Proxy !== 'undefined') {
    TauriLite.api = new Proxy({}, {
      get: function (_, namespace) {
        return new Proxy({}, {
          get: function (_, method) {
            return function (data) {
              return call(namespace, method, data || {})
            }
          }
        })
      }
    });
  } else {
    console.log('Proxy not supported, please use TauriLite.call instead');
  }

  TauriLite.call = function (_method, data) {
    var r = _method.split('.');
    return call(r[0], r[1], data);
  };
  TauriLite.__resolve__ = resolve;

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
    eventListeners = [];
    for (var i = 0; i < listeners.length; i++) {
      if (listeners[i] !== listener) {
        eventListeners.push(listeners[i]);
      }
    }
  }

  function emit(event, data) {
    setTimeout(function () {
      var keys = [event, event.split('.')[0] + '.*', '*'];

      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];

        if (eventListeners[key]) {
          var listeners = eventListeners[key];
          for (var i = 0; i < listeners.length; i++) {
            listeners[i](event, data);
          }
        }
      }

    }, 0);
  }

  TauriLite.addEventListener = addEventListener;
  TauriLite.removeEventListener = removeEventListener;
  TauriLite.__emit__ = emit;

  // === Tauri API ===
  window.TauriLite = TauriLite;
  console.log('TauriLite loaded');

  (function docReady(func) {
    if (document.readyState === "complete" || document.readyState === "interactive") {
      setTimeout(func, 1);
    } else {
      document.addEventListener("DOMContentLoaded", func);
    }
  })(function () {
  });

}());

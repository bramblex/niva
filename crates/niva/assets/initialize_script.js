
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

  function call(method, args) {
    var callbackId = getNextCallbackId();
    window.ipc.postMessage(JSON.stringify([
      callbackId,
      method,
      args
    ]));

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

  (function docReady(func) {
    if (document.readyState === "complete" || document.readyState === "interactive") {
      setTimeout(func);
    } else {
      document.addEventListener("DOMContentLoaded", func);
    }
  })(function () {
  });

}());

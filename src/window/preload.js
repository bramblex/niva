
(function () {
	// Disable drag and open file
	window.addEventListener("dragover", function (ev) { ev.preventDefault(); }, false);

	var TauriLite = {};

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
		var promise = callbacks[response.callback_id];
		if (promise) {
			if (response.code === 0) {
				promise.resolve(response.data);
			} else {
				promise.reject(response);
			}
			delete callbacks[response.callback_id];
		}
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
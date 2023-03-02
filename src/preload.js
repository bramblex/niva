
(function () {
	// Disable drag and open file
	window.addEventListener("dragover", function (ev) { ev.preventDefault(); }, false);

	window.addEventListener("message", function (ev) {
		var data = ev.data
	}, false);

	var TauriLite = {};

	function parseJsonResponse(jsonStr) {
		try {
			return JSON.parse(jsonStr);
		} catch (err) {
			// ignore
		}
		return {
			code: -1,
			message: "Invalid JSON response",
		};
	}

	function call(namespace, method, data) {
		var xhr = new XMLHttpRequest();
		xhr.open("POST", '/api');
		xhr.setRequestHeader("Content-Type", "application/json");
		xhr.send(JSON.stringify({
			namespace: namespace,
			method: method,
			data: data
		}));
		return new Promise((resolve, reject) => {
			xhr.onload = () => {
				var response = parseJsonResponse(xhr.responseText);
				if (xhr.status >= 200 && xhr.status < 300 && response.code === 0) {
					return resolve(response.data);
				}
				return reject(response)
			}
		});
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

	window.TauriLite = TauriLite;
	console.log('TauriLite loaded');


	(function docReady(func) {
		if (document.readyState === "complete" || document.readyState === "interactive") {
			setTimeout(func, 1);
		} else {
			document.addEventListener("DOMContentLoaded", func);
		}
	})(function () {
		// window.ipc.postMessage();
	});

}());

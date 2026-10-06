# Bridge route smoke

This is a real macOS WebView integration smoke for the stable API control lane and the selected stream data lane. It does not run from a browser or count cross-compilation as a platform result. The default `both` run starts two separate windows: the WS lane loads the default resource through Niva's custom protocol; the CSP-denied lane uses the explicit loopback debug entry required to enforce `connect-src 'self'` against Niva's separate local WS endpoint.

Build a Native debug executable first, then run:

```sh
python3 examples/bridge-route-smoke/run.py \
  --binary target/debug/niva \
  --trusted-debug \
  --output /tmp/niva-bridge-route-smoke
```

The runner-level `--trusted-debug` option is required because the CSP lane needs the explicit loopback debug-origin token to exercise stream APIs; it is not passed as an unsupported Niva binary option. Use `--mode ws` or `--mode ipc` to run one lane. The runner writes `result.json`, one `niva.log` per lane, and the generated config/resources beneath the output directory. Standard output contains only a short lane summary; the JSON file retains detailed events. The WS lane observes an actual `attached` server response, a ready-socket attach after the eval `channelOpened` capability, v2 binary payload traffic in both directions, and each ACK matched to a prior downlink id/sequence. It instruments after the document-start runtime, so the initial hello may precede capture and is not used as a pass condition. The CSP lane requires `channelAttach`, `channelSend`, and `channelAck` over IPC. Both lanes require `Niva.runtimeConfig.injectCommonJs` and `trustedLocal`, the installed global `require`, the exact binary child-process echoes through both `require('node:child_process')` and `Niva.child_process`, plus a same-origin child-frame unary call observed in independent top and child sessions. Only the WS lane calls `process.cwd()` through both Node-compatible `require('node:process')` and `Niva.process`, and asserts the retained synchronous-XHR compatibility path; the CSP lane's restrictive policy blocks that endpoint as well as WS.

The page first tries to observe the standard Wry IPC handler called by `window.ipc.postMessage` (the Wry `window.ipc` object is frozen). If that host method is not replaceable, it records matching `JSON.stringify` wire messages as `wire-serialized`; that is evidence of serialization, not proof of sending, and the runner correlates request IDs with Native eval replies. It also observes `WebSocket.prototype.send`, `window.__niva_ipc_reply`, and `XMLHttpRequest.prototype.open`. Credentials, query strings, and binary bodies are redacted. Both lanes write/read a non-text byte pattern with `Niva.fs`, exercise a 1 MiB `FileHandle` write/read plus `stat`/`close`, and pipe bytes through a `child_process` stream. The runner rejects large filesystem API control requests; file bodies must travel on the attached stream. WS use is an optimization for stream data only. API method/args requests must remain IPC `t: "api_call"` in both lanes; the Node `process.cwd()` compatibility method remains on synchronous XHR and is exercised in the WS lane, where CSP permits that compatibility endpoint. The same-origin child frame loads the observer separately and returns only redacted events plus its reply-session count so its unary request can be checked independently.

This smoke does not certify a packaged application, every Native API, other operating systems, or all WebView behaviors. Keep the actual macOS binary SHA and `result.json` with release evidence.

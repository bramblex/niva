(async () => {
  const report = {ok: false, checks: [], origin: location.origin};
  const assert = (name, condition) => {
    if (!condition) throw new Error(name);
    report.checks.push({name, ok: true});
  };
  try {
    const config = await (await fetch('./fixture.json')).json();
    assert('base Niva API exists before application code', !!globalThis.Niva?.fs && !!Niva.bridge);
    assert('os.info is a frozen static object', typeof Niva.os.info === 'object' && Object.isFrozen(Niva.os.info) && typeof Niva.os.info.os === 'string');
    const title = await Promise.race([
      Niva.window.title(),
      new Promise((_, reject) => setTimeout(() => reject(new Error('Native async API did not become ready within 8 seconds')), 8000)),
    ]);
    assert('async Native bridge starts successfully', typeof title === 'string');
    if (config.commonjs) {
      assert('CommonJS globals are injected', typeof require === 'function' && typeof module === 'object' && global === globalThis && exports === module.exports);
      assert('builtins share Niva identity', require('fs') === Niva.fs && require('node:fs/promises') === Niva.fs.promises);
      const loaded = require('./module.cjs');
      assert('CommonJS source executes under strict CSP', loaded.value === 42 && loaded.thisIsExports && loaded.moduleInstance);
      assert('require cache preserves module identity', loaded === require('./module.cjs') && require.cache[require.resolve('./module.cjs')].loaded);
    } else {
      assert('Node globals remain absent when disabled', typeof require === 'undefined' && typeof module === 'undefined' && typeof process === 'undefined' && typeof Buffer === 'undefined' && typeof global === 'undefined');
    }
    if (config.esm) {
      const fs = await import('node:fs');
      assert('fs ESM facade loads', fs.default === Niva.fs);
      const moduleInterface = await import('node:module');
      assert('ESM facade shares Niva identity', fs.default === Niva.fs && fs.readFile === Niva.fs.readFile && moduleInterface.default === Niva.module);
      assert('exactly one effective import map is present', document.querySelectorAll('script[type="importmap"]').length === 1);
      if (config.override) {
        const authorPath = await import('node:path');
        assert('author import map override is preserved', authorPath.default.marker === 'author-override');
        assert('ESM override does not replace CommonJS builtin', require('node:path') === Niva.path);
      }
    } else {
      assert('no import map is injected when disabled', !document.querySelector('script[type="importmap"]'));
    }
    report.ok = true;
  } catch (error) {
    report.error = String(error);
    report.stack = error.stack;
    report.importMaps = Array.from(document.querySelectorAll('script[type="importmap"]'), element => ({text: element.textContent, hasNonce: !!element.nonce}));
    try {
      const response = await fetch('/__niva_runtime/esm/fs.mjs');
      report.esmAsset = {status: response.status, contentType: response.headers.get('content-type'), prefix: (await response.text()).slice(0, 160)};
    } catch (diagnosticError) { report.esmAsset = {error: String(diagnosticError)}; }
  }
  document.getElementById('status').textContent = JSON.stringify(report, null, 2);
  // The diagnostic channel is browser HTTP, deliberately independent of Niva
  // API startup so a failed bootstrap still produces an actionable report.
  await fetch(REPORT_URL, {method: 'POST', headers: {'Content-Type': 'text/plain'}, body: JSON.stringify(report)});
})();

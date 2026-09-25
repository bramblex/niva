(async () => {
  const report = {ok: false, checks: []};
  const assert = (name, condition) => {
    if (!condition) throw new Error(name);
    report.checks.push({name, ok: true});
  };
  try {
    const config = await (await fetch('fixture.json')).json();
    const head = await fetch('large.bin', {method: 'HEAD'});
    assert('large resource HEAD reports full length without a body', head.status === 200 && Number(head.headers.get('content-length')) === config.size && (await head.arrayBuffer()).byteLength === 0);
    const tail = await fetch('large.bin', {headers: {Range: 'bytes=-16'}});
    const bytes = new Uint8Array(await tail.arrayBuffer());
    assert('suffix Range returns exact final bytes', tail.status === 206 && bytes.length === 16 && bytes.every((value, index) => value === (config.size - 16 + index) % 251));
    const invalid = await fetch('large.bin', {headers: {Range: `bytes=${config.size}-`}});
    assert('unsatisfiable range reports 416', invalid.status === 416 && invalid.headers.get('content-range') === `bytes */${config.size}`);
    const full = await fetch('large.bin');
    assert('unbounded large body is rejected', full.status === 413);
    const license = await fetch('META-INF/niva/LICENSE');
    assert('application includes runtime license material', license.status === 200 && (await license.text()).includes('MIT'));
    const loaded = require('./module.cjs');
    assert('packaged CommonJS resolves adjacent resources', loaded.text === 'packaged-module-data' && loaded.fsIdentity);
    const path = await import('node:path');
    assert('packaged ESM interface is the same Niva object', path.default === Niva.path);
    report.ok = true;
  } catch (error) { report.error = String(error); report.stack = error.stack; }
  document.getElementById('status').textContent = JSON.stringify(report, null, 2);
  await fetch(REPORT_URL, {method: 'POST', headers: {'Content-Type': 'text/plain'}, body: JSON.stringify(report)});
})();

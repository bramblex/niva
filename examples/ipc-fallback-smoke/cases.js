/* Runs in an explicitly authorized remote-origin WebView. No Node test runtime. */
(async function () {
  const config = await (await fetch('/fixture')).json();
  const checks = [];
  const pageMarker = `${performance.timeOrigin}:${Math.random().toString(36).slice(2)}`;
  const assert = (ok, message) => { if (!ok) throw new Error(message); };
  async function publishProgress(lastCheck, extra = {}) {
    await fetch('/progress', {method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({lastCheck, checks, pageMarker, documentHidden: document.hidden, ...extra})}).catch(() => {});
  }
  async function check(name, callback) {
    try { await callback(); checks.push({name, ok: true}); }
    catch (error) { checks.push({name, ok: false, error: String(error), stack: error.stack}); }
    await publishProgress(name);
  }
  async function rejects(callback) {
    let rejected = false;
    try { await callback(); } catch (_) { rejected = true; }
    assert(rejected, 'unsupported call did not reject');
  }
  const leaseOnly = new URL(window.location.href).searchParams.get('leaseOnly') === '1';
  if (!leaseOnly) {
  await check('page selects IPC after WS is unavailable', async () => {
    const deadline = Date.now() + 8000;
    while (!Niva.bridge.isIpcOnly() && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert(Niva.bridge.isIpcOnly(), 'unexpected WS transport');
  });
  await check('text file write/read/append', async () => {
    await Niva.fs.promises.writeFile(config.file, '中文 IPC', 'utf8');
    await Niva.fs.promises.appendFile(config.file, '\n追加', 'utf8');
    assert(await Niva.fs.promises.readFile(config.file, 'utf8') === '中文 IPC\n追加', 'text mismatch');
  });
  await check('file stat over IPC', async () => {
    const stat = await Niva.fs.promises.stat(config.file);
    assert(stat.isFile() && stat.size > 0, 'invalid file metadata');
  });
  await check('oversized write rejects before changing the file', async () => {
    await rejects(() => Niva.fs.promises.writeFile(config.file, 'x'.repeat(300 * 1024), 'utf8'));
    assert(await Niva.fs.promises.readFile(config.file, 'utf8') === '中文 IPC\n追加', 'rejected write changed the existing file');
  });
  await check('directory create, rename, copy, list and recursive removal', async () => {
    const fs = Niva.fs.promises;
    const directory = config.directory;
    await fs.mkdir(directory + '/nested', {recursive: true});
    await fs.writeFile(directory + '/nested/original.txt', 'directory workflow', 'utf8');
    await fs.rename(directory + '/nested/original.txt', directory + '/nested/renamed.txt');
    await fs.copyFile(directory + '/nested/renamed.txt', directory + '/nested/copied.txt');
    const names = await fs.readdir(directory + '/nested');
    assert(names.includes('renamed.txt') && names.includes('copied.txt') && !names.includes('original.txt'), 'directory entries mismatch');
    await fs.cp(directory + '/nested', directory + '/copy', {recursive: true});
    assert(await fs.readFile(directory + '/copy/copied.txt', 'utf8') === 'directory workflow', 'recursive copy mismatch');
    await fs.rm(directory, {recursive: true, force: true});
    await rejects(() => fs.stat(directory));
  });
  await check('HTTP text response from independent local server', async () => {
    const result = await Niva.http.requestText({url: config.origin + '/text'});
    assert(result.statusCode === 200 && result.body === 'HTTP 中文', 'HTTP response mismatch');
  });
  await check('HTTP error status preserves response body', async () => {
    const result = await Niva.http.requestText({url: config.origin + '/missing'});
    assert(result.statusCode === 404 && result.body === 'missing', '404 body discarded');
  });
  await check('HTTP response limit rejects rather than truncating', () => rejects(() =>
    Niva.http.requestText({url: config.origin + '/text', maxResponseBytes: 4})));
  await check('HTTPS text request with normal certificate validation', async () => {
    const result = await Niva.https.requestText({url: config.httpsUrl});
    assert(result.statusCode === 200 && result.body.length > 0, 'HTTPS response mismatch');
  });
  await check('HTTP POST preserves text body and headers', async () => {
    const result = await Niva.http.requestText({url: config.origin + '/echo', method: 'POST',
      headers: {'Content-Type': 'text/plain; charset=utf-8', 'X-Niva-Test': 'ipc'}, body: 'POST 中文'});
    const echo = JSON.parse(result.body);
    assert(result.statusCode === 200 && echo.body === 'POST 中文' && echo.header === 'ipc', 'POST mismatch');
  });
  await check('exec text output', async () => {
    const result = await Niva.child_process.execText(config.command);
    assert(result.status === 0 && result.stdout.trim() === 'niva-ipc', 'exec result mismatch');
  });
  await check('execFile text output', async () => {
    const result = await Niva.child_process.execFileText(config.python, ['-c', 'print("niva-exec-file")']);
    assert(result.status === 0 && result.stdout.trim() === 'niva-exec-file', 'execFile result mismatch');
  });
  await check('exec output limit rejects rather than truncating', () => rejects(() =>
    Niva.child_process.execFileText(config.python, ['-c', 'print("x"*4096)'], {maxOutputBytes: 128})));
  await check('exec timeout terminates instead of hanging', () => rejects(() =>
    Niva.child_process.execFileText(config.python, ['-c', 'import time;time.sleep(5)'], {timeoutMs: 200})));
  await check('binary file read rejects', () => rejects(() => Niva.fs.promises.readFile(config.file)));
  await check('synchronous file read rejects', () => rejects(() => Niva.fs.readFileSync(config.file, 'utf8')));
  await check('raw fs.node cannot bypass binary restriction', () => rejects(() =>
    Niva.bridge.call('fs.node', ['readFile', {path: config.file}])));
  await check('Native synchronous process RPC cannot bypass IPC restrictions', () => rejects(() => Niva.bridge.call('process.spawnSync', ['unused', {}])));
  await check('IPC cannot create a persistent Native window', () => rejects(() => Niva.bridge.call('window.open', [{}])));
  await check('IPC cannot obtain a persistent file descriptor', () => rejects(() => Niva.bridge.call('fs.node', ['open', {path: config.file, flag: 'r'}])));
  await check('file unlink over IPC', () => Niva.fs.promises.unlink(config.file));
  await check('active IPC heartbeat keeps a long exec alive', async () => {
    await publishProgress('active IPC heartbeat request started');
    const outcome = await Promise.race([
      Niva.child_process.execFileText(config.python,
        ['-c', 'import time; time.sleep(4); print("heartbeat-alive")'])
        .then(result => ({result}), error => ({error})),
      new Promise(resolve => setTimeout(() => resolve({timedOut: true}), 10000)),
    ]);
    await publishProgress('active IPC heartbeat request settled', {
      timedOut: !!outcome.timedOut,
      rejected: !!outcome.error,
    });
    assert(!outcome.timedOut, 'active IPC heartbeat call did not settle within 10 seconds');
    assert(!outcome.error, `active IPC heartbeat call rejected: ${String(outcome.error)}`);
    const result = outcome.result;
    assert(result.status === 0 && result.stdout.trim() === 'heartbeat-alive', 'active lease was not renewed');
  });
  }
  if (leaseOnly) {
    await check('page selects IPC after WS is unavailable', async () => {
      const deadline = Date.now() + 8000;
      while (!Niva.bridge.isIpcOnly() && Date.now() < deadline) {
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      assert(Niva.bridge.isIpcOnly(), 'unexpected WS transport');
    });
  }
  await check('lost IPC heartbeat delivery expires the lease and cancels exec', async () => {
    await publishProgress('lease check entered', {pageVisible: !document.hidden, ipcOnly: Niva.bridge.isIpcOnly()});
    // Keep WebKit's page loop and Native reply path intact while suppressing
    // only the runtime's one-second lease-renewal timer in this harness.
    const timerDescriptor = Object.getOwnPropertyDescriptor(window, 'setTimeout');
    const originalSetTimeout = window.setTimeout.bind(window);
    let suppressedHeartbeatTimers = 0;
    Object.defineProperty(window, 'setTimeout', {
      configurable: true,
      writable: true,
      value(callback, delay, ...args) {
        if (Number(delay) === 1000) {
          suppressedHeartbeatTimers++;
          return 0;
        }
        return originalSetTimeout(callback, delay, ...args);
      },
    });
    try {
      await publishProgress('IPC heartbeat timer suppression installed');
      // This is deliberately last: an expired realm must not revive its requests.
      const program = 'import pathlib,time; pathlib.Path(' + JSON.stringify(config.started) +
        ').write_text("started"); time.sleep(6); pathlib.Path(' + JSON.stringify(config.orphan) +
        ').write_text("orphan survived")';
      const pending = Niva.child_process.execFileText(config.python, ['-c', program]);
      let failure;
      const settled = pending.then(() => {}, error => { failure = error; });
      await publishProgress('pending exec dispatched', {nativeRequestStarted: true});
      await publishProgress('child-status fetch started', {started: false});
      let statusResponse = await fetch('/child-started', {cache: 'no-store'});
      let status = await statusResponse.json();
      await publishProgress('child-status fetch returned', {
        started: status.started,
        httpStatus: statusResponse.status,
      });
      const deadline = Date.now() + 5000;
      let started = status.started;
      let childStatusFetches = 1;
      while (!started && Date.now() < deadline) {
        statusResponse = await fetch('/child-started', {cache: 'no-store'});
        status = await statusResponse.json();
        childStatusFetches++;
        started = status.started;
        if (started) break;
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      assert(started, 'fixture child never started');
      await publishProgress('child-status fetch confirmed child started', {started: true,
        childStatusFetches});
      const heartbeatDeadline = Date.now() + 2500;
      while (suppressedHeartbeatTimers === 0 && Date.now() < heartbeatDeadline) {
        await new Promise(resolve => originalSetTimeout(resolve, 25));
      }
      assert(suppressedHeartbeatTimers > 0, 'the harness did not suppress an IPC heartbeat timer');
      await publishProgress('lease child started; waiting for Native result', {started: true,
        suppressedHeartbeatTimers, childStatusFetches});
      const outcome = await Promise.race([
        settled.then(() => 'settled'),
        new Promise(resolve => originalSetTimeout(() => resolve('reply-timeout'), 7000)),
      ]);
      await publishProgress('Native IPC promise settled', {outcome, failure: String(failure)});
      assert(outcome === 'settled', 'Native did not reject the pending call after heartbeat delivery stopped');
      assert(failure && /session|lease|disconnect|失联|连接/i.test(String(failure)), 'lease loss was not reported');
    } finally {
      if (timerDescriptor) Object.defineProperty(window, 'setTimeout', timerDescriptor);
      else delete window.setTimeout;
    }
  });
  const report = {ok: checks.every(check => check.ok), checks, pageMarker, documentHidden: document.hidden};
  document.getElementById('status').textContent = JSON.stringify(report, null, 2);
  await fetch('/result', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(report)});
})().catch(async error => {
  const report = {ok: false, error: String(error), stack: error.stack};
  document.getElementById('status').textContent = JSON.stringify(report, null, 2);
  await fetch('/result', {method: 'POST', body: JSON.stringify(report)});
});

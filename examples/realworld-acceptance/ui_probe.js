/* Test instrumentation only; the upstream frontend and backend are unchanged. */
(async () => {
  const report = {ok: false, checks: [], engine: 'niva-webview-original-frontend'};
  const delay = milliseconds => new Promise(resolve => setTimeout(resolve, milliseconds));
  async function until(read, label) {
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline) {
      const result = read();
      if (result) return result;
      await delay(100);
    }
    throw new Error('Timed out: ' + label);
  }
  const select = name => document.querySelector(`[data-test="${name}"]`);
  function setInput(name, value) {
    const wrapper = select(name);
    const input = wrapper?.matches('input') ? wrapper : wrapper?.querySelector('input');
    if (!input) throw new Error('Missing input: ' + name);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
    input.dispatchEvent(new Event('input', {bubbles: true}));
    input.dispatchEvent(new Event('change', {bubbles: true}));
    input.dispatchEvent(new Event('blur', {bubbles: true}));
  }
  try {
    await until(() => select('signin-username'), 'sign-in form renders');
    report.checks.push({name: 'original React sign-in form renders', ok: true});
    setInput('signin-username', 'Katharina_Bernier');
    setInput('signin-password', 'incorrect-password');
    await delay(200);
    select('signin-submit').click();
    await until(() => select('signin-error')?.textContent, 'wrong password error');
    report.checks.push({name: 'UI reports real backend authentication failure', ok: true});
    setInput('signin-password', 's3cret');
    await delay(200);
    select('signin-submit').click();
    await until(() => select('sidenav-username')?.textContent.includes('Katharina_Bernier'), 'authenticated user in navigation');
    report.checks.push({name: 'UI login establishes backend session', ok: true});
    select('sidenav-bankaccounts').click();
    await until(() => select('bankaccount-list')?.textContent.trim(), 'bank accounts fetched and rendered');
    report.checks.push({name: 'original GraphQL bank account UI loads', ok: true});
    select('sidenav-user-settings').click();
    await until(() => select('user-settings-form'), 'user settings form');
    report.checks.push({name: 'authenticated route navigation works', ok: true});
    select('sidenav-signout').click();
    await until(() => select('signin-username'), 'logout returns to sign-in');
    report.checks.push({name: 'UI logout clears authenticated session', ok: true});
    report.ok = true;
  } catch (error) { report.error = String(error); report.stack = error.stack; report.location = location.href; }
  await fetch('/__niva_ui_result', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(report)});
})();

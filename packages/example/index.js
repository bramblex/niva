console.log('hello world');

const log = (ok, title, result) => {
  const content = JSON.stringify(result, null, 2);
  document.getElementById('console-panel').innerText = ok ?
    `TauriLite.api.${title}\nResult: \n${content}` :
    `TauriLite.api.${title}\nError: \n${content}`;
  document.getElementById('console-panel').style.color = ok ? '' : 'red';
}

const testCases = {
  fs: {
    stat: [{ path: 'index.js' }, { path: '/' }],
    exists: [{ path: 'index.js' }, { path: '/' }, { path: 'not-exists' }],

    read: [{ path: 'index.js' }, { path: '/tmp/text2.txt' }, { path: '/tmp/test.txt' }, {
      path: 'logo.png',
      encode: 'base64',
    }],
    write: [{ path: '/tmp/test.txt', content: new Date().toLocaleString() }],

    mv: [{ from: '/tmp/test.txt', to: '/tmp/test2.txt' }, { from: '/tmp/test2.txt', to: '/tmp/test3.txt' }],
    cp: [{ from: '/tmp/test.txt', to: '/tmp/test2.txt' }, { from: '/tmp/test2.txt', to: '/tmp/test3.txt' }],
    rm: [{ path: '/tmp/test.txt' }, { path: '/tmp/test2.txt' }, { path: '/tmp/test3.txt' }],

    ls: [null, { path: './' }, { path: '/tmp/' }],
    mkDir: [{ path: '/tmp/test-dir/test2-dir/test3-dir' },],
    rmDir: [{ path: '/tmp/test-dir' }],
  },
  http: {
    request: [
      { method: 'GET', url: 'http://httpbin.org/ip' },
      {
        method: 'POST', url: location.href + 'api', body: JSON.stringify({
          namespace: 'fs',
          method: 'ls',
          data: {},
        })
      }
    ],
  },
  os: {
    info: [null],
    dirs: [null],
  },
  process: {
    pid: [null],
    cwd: [null],
    chDir: [{ path: '/tmp' }],
    env: [null],
    exec: [
      { command: 'echo', args: ['hello world'] },
      { command: 'open', args: ['https://tauri.studio'] },
      { command: 'ls', args: ['/afwef/awefaaawef/'] }
    ],
    exit: [null],
  }
};

for (const [namespace, methods] of Object.entries(testCases)) {
  for (const [method, cases] of Object.entries(methods)) {
    let group = document.createElement('div');
    group.innerHTML = `<span>${namespace}.${method}:</span>`;
    for (const c of cases) {
      const button = document.createElement('button');
      const title = `${namespace}.${method}(${c ? JSON.stringify(c) : ''})`
      button.innerText = title;
      button.onclick = () => {
        TauriLite.api[namespace][method](c)
          .then(res => log(true, title, res))
          .catch(err => log(false, title, err));
      };
      group.appendChild(button);
    }
    document.getElementById('button-panel').appendChild(group);
  }
}

document.addEventListener('contextmenu', (e) => {
})

TauriLite.addEventListener('*', (event, data) => {
  console.log(event, data);
  document.getElementById('event-panel').innerText = `Event: ${event}\nData: ${JSON.stringify(data, null, 2)}`;
})

document.getElementById('title').addEventListener('drop', (e) => {
  alert('drop');
})

document.getElementById('red-block').addEventListener('mousedown', (e) => {
  TauriLite.api.window.dragWindow();
})

document.getElementById('load-image').addEventListener('click', async (e) => {
  const { content } = await TauriLite.api.fs.read({ path: 'logo.png', encode: 'base64' })
  const image = document.getElementById('image');
  image.src = `data:image/png;base64,${content}`;
  TauriLite.api.fs.write({ path: '/tmp/logo.png', content: content, encode: 'base64' });
})

console.log('hello world');

const log = (ok, title, result) => {
	const content = JSON.stringify(result, null, 2);
	document.getElementById('console-panel').innerHTML = ok ?
		`<code>${title}\nResult: \n${content}</code>` :
		`<code style="color: red;">${title}\nError: \n${content}<code>`;
}

const testCases = {
	fs: {
		stat: [null, {}, { path: 'index.js' }, { path: '/' }, { path: 'not-exists' }],
		exists: [null, {}, { path: 'index.js' }, { path: '/' }, { path: 'not-exists' }],

		read: [null, {}, { path: 'index.js' }, { path: 'not-exists' }, { path: '/tmp/test.txt' }],
		write: [null, {}, { path: '/tmp/test.txt', content: new Date().toLocaleString() }, { path: 'not-exists', content: '' }],

		ls: [null, {}, { path: '/' }, { path: '../' }, { path: 'not-exists' }],
	},
	http: {
		request: [null, {}, { method: 'GET', url: 'https://tauri.studio' }, { method: 'GET', url: 'https://tauri.studio/not-exists' }],
	},
	os: {
		info: [null],
		dirs: [null],
	},
	process: {
		pid: [null],
		cwd: [null],
		chDir: [null, {}, { path: '/' }, { path: 'not-exists' }],
		env: [null],
		exec: [null, {}, { command: 'echo', args: ['hello world'] }, { command: 'not-exists', args: [] }, { command: 'ls', args: ['/afwef/awefaaawef/'] }],
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
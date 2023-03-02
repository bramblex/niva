console.log('hello world');

document.getElementById('list-button').addEventListener('click', () => {
	axios.post('/', { method: 'ls', data: {} }).then(res => {
		document.getElementById('console-panel').innerText += res.data + '\n';
	})
});

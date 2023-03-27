console.log('Hello World!')
Niva.api.window.current().then(id => console.log(`Current window id: ${id}`));
Niva.addEventListener('*', console.log);
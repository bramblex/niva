import { generateConfig } from "./config-template";

export function generateNewProject(name: string): [string, string][] {
	return [
		['niva.json', JSON.stringify(generateConfig("simple", name), null, 2)],
		['index.html', "<h1>Hello World!</h1><script src='./index.js'></script>"],
		['index.js', "console.log('Hello World!')"],
	]
}
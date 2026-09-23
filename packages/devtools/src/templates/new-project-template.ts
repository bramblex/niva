import { generateConfig } from "./config-template";

export function generateNewProject(name: string): [string, string][] {
	return [
		['niva.json', JSON.stringify(generateConfig("simple", name), null, 2)],
		['index.html', `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <!-- Edit this policy when the page needs additional origins. Niva adds its active loopback bridge origins when it serves this document. -->
  <meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'" />
  <title>Hello Niva</title>
  <script type="module" src="./index.js"></script>
</head>
<body>
  <h1>Hello Niva!</h1>
</body>
</html>`],
		['index.js', "console.log('Hello World!')"],
	]
}

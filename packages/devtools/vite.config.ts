import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const nivaBuiltins = [
  "node:fs/promises",
  "node:path",
  "node:os",
  "node:process",
  "node:child_process",
  "node:https",
];

const nivaBuiltinExports: Record<string, { value: string; names: string[] }> = {
  "node:fs/promises": {
    value: "globalThis.Niva.fs.promises",
    names: ["constants", "readFile", "writeFile", "appendFile", "mkdir", "readdir", "stat", "lstat", "realpath", "rename", "copyFile", "access", "rm", "unlink", "cp", "open"],
  },
  "node:path": {
    value: "globalThis.Niva.path",
    names: ["resolve", "normalize", "isAbsolute", "join", "relative", "toNamespacedPath", "dirname", "basename", "extname", "parse", "format", "matchesGlob", "sep", "delimiter", "posix", "win32"],
  },
  "node:os": {
    value: "globalThis.Niva.os",
    names: ["EOL", "arch", "platform", "homedir", "tmpdir", "hostname", "release", "totalmem", "type", "version", "userInfo", "cpus", "freemem", "networkInterfaces", "uptime", "devNull"],
  },
  "node:process": {
    value: "globalThis.Niva.process",
    names: ["arch", "argv", "argv0", "env", "execPath", "pid", "platform", "version", "versions", "cwd", "chdir", "nextTick", "stdin", "stdout", "stderr", "exit", "on"],
  },
  "node:child_process": {
    value: "globalThis.Niva.child_process",
    names: ["spawn", "exec", "execFile", "spawnSync", "execFileSync", "execSync"],
  },
  "node:https": {
    value: "globalThis.Niva.https",
    names: ["request", "get", "createServer", "IncomingMessage", "ServerResponse", "ClientRequest", "METHODS", "STATUS_CODES"],
  },
};

function nivaBuiltinFacade(): Plugin {
  return {
    name: "niva-node-builtins",
    enforce: "pre",
    resolveId(id) {
      return Object.prototype.hasOwnProperty.call(nivaBuiltinExports, id)
        ? `\0niva-builtin:${id}`
        : null;
    },
    load(id) {
      if (!id.startsWith("\0niva-builtin:")) return null;
      const specifier = id.slice("\0niva-builtin:".length);
      const facade = nivaBuiltinExports[specifier];
      if (!facade) return null;
      const named = facade.names
        .map((name) => `export const ${name} = value.${name};`)
        .join("\n");
      return `const value = ${facade.value};\n${named}\nexport default value;`;
    },
  };
}

// Keep the CRA dev-server port so packages/devtools/niva.json
// (`debug.entry: http://localhost:3000`) keeps working.
export default defineConfig({
  plugins: [nivaBuiltinFacade(), react()],
  optimizeDeps: {
    exclude: nivaBuiltins,
  },
  server: {
    port: 3000,
    strictPort: true,
  },
  build: {
    outDir: "build",
    sourcemap: false,
  },
});

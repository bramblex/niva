import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/events.js";
import "./runtime/process.js";
const process = globalThis[Symbol.for("niva.node-compat.runtime")].process;
if (!process) throw new Error("Host process is only available in the trusted main window");
export const { arch, argv, argv0, env, execPath, pid, platform, version, versions, cwd, chdir, nextTick, stdin, stdout, stderr, exit } = process;
export const on = process.on.bind(process);
export default process;

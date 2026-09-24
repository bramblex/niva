import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/events.js";
import "./runtime/path.js";
import "./runtime/buffer.js";
import "./runtime/child_process.js";

const childProcess = globalThis[Symbol.for("niva.node-compat.runtime")].child_process;

export const spawn = childProcess.spawn;
export const exec = childProcess.exec;

export const execFile = childProcess.execFile;
export const spawnSync = childProcess.spawnSync;
export const execFileSync = childProcess.execFileSync;
export const execSync = childProcess.execSync;
export default childProcess;

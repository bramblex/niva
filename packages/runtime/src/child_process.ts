
const childProcess = globalThis.Niva.child_process;

export const spawn = childProcess.spawn;
export const exec = childProcess.exec;

export const execFile = childProcess.execFile;
export const spawnSync = childProcess.spawnSync;
export const execFileSync = childProcess.execFileSync;
export const execSync = childProcess.execSync;
export const execText = childProcess.execText;
export const execFileText = childProcess.execFileText;
export default childProcess;

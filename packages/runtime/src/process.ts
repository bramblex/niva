const process = globalThis.Niva.process;
export const { arch, argv, argv0, env, execPath, pid, platform, version, versions, cwd, chdir, nextTick, hrtime, stdin, stdout, stderr, exit } = process;
export const on = process.on.bind(process);
export default process;

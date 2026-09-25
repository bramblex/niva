const os = globalThis.Niva.os;
// EOL is statically determined from the platform at module load.
export const EOL = globalThis.Niva?.bootstrap?.os?.EOL ?? "\n";
export const { arch, platform, homedir, tmpdir, hostname, release, totalmem, type, version, userInfo, cpus, freemem, networkInterfaces, uptime } = os;
export const info = os.info;
export const dirs = os.dirs;
export default os;

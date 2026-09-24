import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/os.js";
const os = globalThis[Symbol.for("niva.node-compat.runtime")].os;
// EOL is statically determined from the platform at module load.
export const EOL = globalThis.Niva?.bootstrap?.os?.EOL ?? "\n";
export const { arch, platform, homedir, tmpdir, hostname, release, totalmem, type, version, userInfo, cpus, freemem, networkInterfaces, uptime } = os;
export default os;

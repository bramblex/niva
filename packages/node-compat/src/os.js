import "./runtime/bridge.js";
import "./runtime/path.js";
import "./runtime/os.js";

const os = globalThis[Symbol.for("niva.node-compat.runtime")].os;

export const EOL = os.EOL;
export const devNull = os.devNull;
export const sep = os.sep;
export const delimiter = os.delimiter;
export const info = os.info;
export const dirs = os.dirs;
export const platform = os.platform;
export const arch = os.arch;
export const homedir = os.homedir;
export const tmpdir = os.tmpdir;

export default os;

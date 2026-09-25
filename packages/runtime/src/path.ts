
const path = globalThis.Niva.path;

export const resolve = path.resolve;
export const normalize = path.normalize;
export const isAbsolute = path.isAbsolute;
export const join = path.join;
export const relative = path.relative;
export const toNamespacedPath = path.toNamespacedPath;
export const dirname = path.dirname;
export const basename = path.basename;
export const extname = path.extname;
export const parse = path.parse;
export const format = path.format;
export const matchesGlob = path.matchesGlob;
export const sep = path.sep;
export const delimiter = path.delimiter;
export const posix = path.posix;
export const win32 = path.win32;
export const setCwd = path.setCwd;
export const getCwd = path.getCwd;

export default path;

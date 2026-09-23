import "./runtime/bridge.js";
import "./runtime/path.js";
import "./runtime/buffer.js";
import "./runtime/fs.js";

const fsPromises = globalThis[Symbol.for("niva.node-compat.runtime")].fs.promises;

export const constants = fsPromises.constants;
export const readFile = fsPromises.readFile;
export const writeFile = fsPromises.writeFile;
export const appendFile = fsPromises.appendFile;
export const mkdir = fsPromises.mkdir;
export const readdir = fsPromises.readdir;
export const stat = fsPromises.stat;
export const access = fsPromises.access;
export const rename = fsPromises.rename;
export const rm = fsPromises.rm;
export const cp = fsPromises.cp;
export const copyFile = fsPromises.copyFile;
export default fsPromises;

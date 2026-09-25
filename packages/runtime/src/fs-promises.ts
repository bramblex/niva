
const fs = globalThis.Niva.fs.promises;
export const constants = fs.constants;
export const readFile = fs.readFile;
export const writeFile = fs.writeFile;
export const appendFile = fs.appendFile;
export const mkdir = fs.mkdir;
export const readdir = fs.readdir;
export const stat = fs.stat;
export const lstat = fs.lstat;
export const realpath = fs.realpath;
export const rename = fs.rename;
export const copyFile = fs.copyFile;
export const access = fs.access;
export const rm = fs.rm;
export const unlink = fs.unlink;
export const cp = fs.cp;
export const open = fs.open;
export default fs;

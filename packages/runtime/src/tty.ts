const tty = globalThis.Niva.tty;

export const isatty = tty.isatty;
export const ReadStream = tty.ReadStream;
export const WriteStream = tty.WriteStream;
export default tty;

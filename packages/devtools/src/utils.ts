import { modal } from "./modal";

export function uuid() {
  let dt = new Date().getTime();
  let uuid = "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(
    /[xy]/g,
    function (c) {
      let r = (dt + Math.random() * 16) % 16 | 0;
      dt = Math.floor(dt / 16);
      return (c === "x" ? r : (r & 0x3) | 0x8).toString(16);
    }
  );
  return uuid;
}

export function tryOrP<T>(p: Promise<T>, defaultValue: T): Promise<T> {
  return p.catch(() => defaultValue);
}

export function tryOr<T>(p: () => T, defaultValue: T): T {
  try {
    return p();
  } catch (e) {
    return defaultValue;
  }
}

export function withCtxP<T>(p: Promise<T>, context: string): Promise<T> {
  return p.catch((e) =>
    Promise.reject(new Error(context + ": " + e.toString()))
  );
}

export function withCtx<T>(p: () => T, context: string): T {
  try {
    return p();
  } catch (e) {
    throw new Error(context + ": " + (e as any).toString());
  }
}

export function tryOrAlert<T>(p: () => T) {
  try {
    return p();
  } catch (e) {
    modal.alert("错误", (e as any).toString());
    throw e;
  }
}

export async function tryOrAlertAsync<T>(p: () => Promise<T>) {
  try {
    return await p();
  } catch (e) {
    modal.alert("错误", (e as any).toString());
    throw e;
  }
}

let sep: string | null = null;
TauriLite.api.os.sep().then((s: string) => (sep = s));

export function pathJoin(...paths: string[]) {
  return paths.join(sep!);
}

export function dirname(path: string) {
  return path.split(sep!).slice(0, -1).join(sep!);
}

let dirs: { temp: string, home: string } | null = null;
TauriLite.api.os.dirs().then((_dirs: any) => (dirs = _dirs));

export function tempWith(...paths: string[]) {
  return pathJoin(dirs!.temp, ...paths);
}

export function getHome() {
  return dirs!.home;
}

let currentDir: string | null = null;
TauriLite.api.process.currentDir().then((dir: string) => (currentDir = dir));

export function getCurrentDir() {
  return currentDir!;
}

type XPromise<T> = Promise<T> & {
  resolve: (value: T) => void;
};

export function createPromise<T>(): XPromise<T> {
  let resolve: (value: T) => void = (v: T) => { };
  let promise = new Promise<T>(
    (_resolve) => (resolve = _resolve)
  ) as XPromise<T>;
  promise.resolve = resolve;
  return promise;
}

export function base64ToArrayBuffer(base64: string) {
  const binary_string = window.atob(base64);
  const len = binary_string.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binary_string.charCodeAt(i);
  }
  return bytes.buffer;
}

export function arrayBufferToBase64(buffer: ArrayBuffer) {
  let binary = '';
  const bytes = new Uint8Array(buffer);
  const len = bytes.byteLength;
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return window.btoa(binary);
}

export function concatArrayBuffers(buffer1: ArrayBuffer, buffer2: ArrayBuffer) {
  const newBuffer = new ArrayBuffer(buffer1.byteLength + buffer2.byteLength);
  // copy the contents of buffer1 into the new buffer
  new Uint8Array(newBuffer, 0, buffer1.byteLength).set(new Uint8Array(buffer1));
  // copy the contents of buffer2 into the new buffer, starting at the end of buffer1
  new Uint8Array(newBuffer, buffer1.byteLength, buffer2.byteLength).set(new Uint8Array(buffer2));
  return newBuffer;
}
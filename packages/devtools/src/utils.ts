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

type XPromise<T> = Promise<T> & {
  resolve: (value: T) => void;
};

export function createPromise<T>(): XPromise<T> {
  let resolve: (value: T) => void = (v: T) => {};
  let promise = new Promise<T>(
    (_resolve) => (resolve = _resolve)
  ) as XPromise<T>;
  promise.resolve = resolve;
  return promise;
}

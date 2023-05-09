// import { ModalModel } from "../models/modal.model";
// import { AppModel } from "../models/app.model";
// import { AppResult } from "./result";

import { AppModel } from "../models/app.model";
import { AppResult } from "./result";

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

export async function tryOrAlert<T>(app: AppModel, r: Promise<AppResult<T>>) {
  const { locale, modal } = app.state;
  try {
    const result = await r;
    if (result.isErr()) {
      modal.alert(
        locale.t("ERROR"),
        result.error.toLocaleMessage(app)
      );
    }
  } catch (err) {
    modal.alert(
      locale.t("ERROR"),
      `[UNKNOWN ERROR] ${(err as Error).toString()}`
    );
  }
}

export function limitString(str: string, limit: number) {
  if (str.length > limit) {
    const remainingChars = limit - 3;
    return `...${str.slice(-remainingChars)}`;
  } else {
    return str;
  }
}

// export function tryOrP<T>(p: Promise<T>, defaultValue: T): Promise<T> {
//   return p.catch(() => defaultValue);
// }

// export function tryOr<T>(p: () => T, defaultValue: T): T {
//   try {
//     return p();
//   } catch (e) {
//     return defaultValue;
//   }
// }

// export function withCtxP<T>(p: Promise<T>, context: string): Promise<T> {
//   return p.catch((e) =>
//     Promise.reject(new Error(context + ": " + e.toString()))
//   );
// }

// export function withCtx<T>(p: () => T, context: string): T {
//   try {
//     return p();
//   } catch (e) {
//     throw new Error(context + ": " + (e as any).toString());
//   }
// }

// export function tryOrAlert<T>(app: AppModel, p: () => T) {
//   const {locale, modal} = app.state;
//   try {
//     return p();
//   } catch (e) {
//     modal.alert(locale.getText('ERROR'), (e as any).toString());
//     throw e;
//   }
// }

// export async function tryOrAlertAsync<T>(app: AppModel, p: () => Promise<T>) {
//   const {locale, modal} = app.state;
//   try {
//     return await p();
//   } catch (e) {
//     modal.alert(locale.getText('ERROR'), (e as any).toString());
//     throw e;
//   }
// }
let baseFileSystemUrl: string | null = null;
Niva.api.webview
  .baseFileSystemUrl()
  .then((s: string) => (baseFileSystemUrl = s));

export function fileSystemUrl(path: string) {
  return (baseFileSystemUrl + path).replace(/\\/g, "/");
}

let sep: string | null = null;
Niva.api.os.sep().then((s: string) => (sep = s));

export function pathJoin(...paths: string[]) {
  return paths.filter((s) => s).join(sep!);
}

export function pathSplit(path: string): string[] {
  return path.split(sep!);
}

export function dirname(path: string) {
  return path.split(sep!).slice(0, -1).join(sep!);
}

let dirs: { temp: string; home: string; data: string } | null = null;
Niva.api.os.dirs().then((_dirs: any) => (dirs = _dirs));

export function tempDirWith(...paths: string[]) {
  return pathJoin(dirs!.temp, ...paths);
}

export function dataDirWith(...paths: string[]) {
  return pathJoin(dirs!.data, ...paths);
}

export function getHome() {
  return dirs!.home;
}

let currentDir: string | null = null;
Niva.api.process.currentDir().then((dir: string) => (currentDir = dir));

export function getCurrentDir() {
  return currentDir!;
}

export type XPromise<T> = Promise<T> & {
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


export function parseArgs(args: string[]) {
  const result: Record<string, string> = {};
  for (const arg of args.slice(1)) {
    if (arg.startsWith("--")) {
      const [key, value] = arg.slice(2).split("=");
      result[key] = value || "";
    }
  }
  return result;
}

export function isAbsolutePath(path: string) {
  return /^(\/|[A-Z]:\\)/.test(path);
}

export async function resolvePath(path: string) {
  const { process } = Niva.api;
  return isAbsolutePath(path)
    ? path
    : pathJoin(await process.currentDir(), path);
}

export function importAll<T>(resolve: any) {
  const resources: Record<string, T> = {};
  for (const filePath of resolve.keys()) {
    resources[filePath] = resolve(filePath);
  }
  return resources;
}

export function parseVersion(versionString: string): number[] {
  const versionDigits = versionString
    .replace(/[^0-9.]/g, "")
    .split(".")
    .map(Number);
  while (versionDigits.length < 4) {
    versionDigits.push(0);
  }
  return versionDigits.slice(0, 4);
}

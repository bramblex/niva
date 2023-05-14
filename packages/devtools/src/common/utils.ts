import { trimEnd, trimStart } from "lodash";
import { AppModel } from "../models/app.model";
import { AppResult } from "./result";

let baseFileSystemUrl: string | null = null;
let sep: string | null = null;
let dirs: { temp: string; home: string; data: string } | null = null;
let currentDir: string | null = null;

const readPromise = Promise.all([
  Niva.api.webview
    .baseFileSystemUrl()
    .then((s: string) => (baseFileSystemUrl = s)),
  Niva.api.os.sep().then((s: string) => (sep = s)),
  Niva.api.os.dirs().then((_dirs: any) => (dirs = _dirs)),
  Niva.api.process.currentDir().then((dir: string) => (currentDir = dir)),
]);

export function envReady(callback: () => any) {
  readPromise.then(callback)
}

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

export function urlJoin(...paths: string[]) {
  return paths.reduce((l, r) => {
    const left = trimEnd(l, '/');
    const right = trimStart(r, '/');
    return left + '/' + right
  })
}

export function fileSystemUrl(path: string) {
  return urlJoin(baseFileSystemUrl!, path.replace('\\', '/'));
}

export function pathJoin(...paths: string[]) {
  return paths.filter((s) => s).join(sep!);
}

export function pathSplit(path: string): string[] {
  return path.split(sep!);
}

export function dirname(path: string) {
  return path.split(sep!).slice(0, -1).join(sep!);
}

export function tempDirWith(...paths: string[]) {
  return pathJoin(dirs!.temp, ...paths);
}

export function dataDirWith(...paths: string[]) {
  return pathJoin(dirs!.data, ...paths);
}

export function getHome() {
  return dirs!.home;
}

export function getCurrentDir() {
  return currentDir!;
}

export type XPromise<T> = Promise<T> & {
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

export function classNames(classNamesMap: Record<string, boolean>) {
  let classNames = [];
  for (const [className, usage] of Object.entries(classNamesMap)) {
    if (usage) {
      classNames.push(className);
    }
  };
  return classNames.join(' ');
};
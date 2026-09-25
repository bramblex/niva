import { trimEnd, trimStart } from "lodash";
import https from "node:https";
import { os, path, process } from "./node";
import { niva } from "./niva";
import { AppModel } from "../models/app.model";
import { AppResult } from "./result";
import { ErrorCode } from "./error";

let baseFileSystemUrl: string | null = null;
let dirs: { data: string; temp?: string; home?: string } | null = null;

const readPromise = Promise.all([
  niva.webview
    .baseFileSystemUrl()
    .then((s: string) => (baseFileSystemUrl = s)),
  niva.os.dirs().then((_dirs) => (dirs = _dirs)),
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
    if (result.isErr() && result.error.code !== ErrorCode.PROJECT_HAS_UNSAVED_CHANGE) {
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
  return path.join(...paths.filter((s) => s));
}

export function pathSplit(value: string): string[] {
  return value.split(path.sep);
}

export function dirname(filePath: string) {
  return path.dirname(filePath);
}

export function tempDirWith(...paths: string[]) {
  return path.join(os.tmpdir(), ...paths);
}

export function dataDirWith(...paths: string[]) {
  if (!dirs) throw new Error("Niva application data directory is not ready");
  return path.join(dirs.data, ...paths);
}

export function getHome() {
  return os.homedir();
}

export function getCurrentDir() {
  return process.cwd();
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
      const option = arg.slice(2);
      const separator = option.indexOf("=");
      const key = separator < 0 ? option : option.slice(0, separator);
      const value = separator < 0 ? "" : option.slice(separator + 1);
      result[key] = value;
    }
  }
  return result;
}

export function isAbsolutePath(value: string) {
  return path.isAbsolute(value);
}

export async function resolvePath(value: string) {
  return isAbsolutePath(value)
    ? value
    : path.resolve(process.cwd(), value);
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

export async function checkVersion(): Promise<string | null> {
  try {
    const body = await new Promise<string>((resolve, reject) => {
      const request = https.get(
        "https://api.github.com/repos/bramblex/niva/releases/latest",
        { headers: { "User-Agent": "Niva", Accept: "application/vnd.github+json" } },
        (response: any) => {
          let body = "";
          response.setEncoding("utf8");
          response.on("data", (chunk: string) => { body += chunk; });
          response.on("end", () => resolve(body));
          response.on("error", reject);
        },
      );
      request.on("error", reject);
      request.setTimeout(10000, () => request.destroy(new Error("Update check timed out")));
    });
    const remoteVersion = JSON.parse(body)?.tag_name;
    if (typeof remoteVersion !== "string") return null;

    const localVersion = process.version;
    const remote = parseVersion(remoteVersion);
    const local = parseVersion(localVersion);
    for (let i = 0; i < remote.length; i++) {
      if (remote[i] > local[i]) return remoteVersion;
      if (remote[i] < local[i]) return null;
    }
  } catch {
    // Updates are optional; a network failure should not interrupt startup.
  }
  return null;
}

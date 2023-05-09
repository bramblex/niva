import { Result as _Result } from "neverthrow";

declare global {
  var Niva: {
    addEventListener(
      event: string,
      callback: (event: string, data: any) => any
    ): void;

    removeEventListener(
      event: string,
      callback: (event: string, data: any) => any
    ): void;

    call(method: string, data: any): Promise<any>;

    api: {
      clipboard: { [method: string]: Function },
      dialog: { [method: string]: Function },
      extra: { [method: string]: Function },
      fs: { [method: string]: Function },
      http: { [method: string]: Function },
      monitor: { [method: string]: Function },
      os: { [method: string]: Function },
      process: { [method: string]: Function },
      resource: { [method: string]: Function },
      short: { [method: string]: Function },
      tray: { [method: string]: Function },
      webview: { [method: string]: Function },
      window: { [method: string]: Function },
      windowExtra: { [method: string]: Function },
    };
  };
}

export { }
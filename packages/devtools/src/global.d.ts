import { Result as _Result } from "neverthrow";

declare global {
  type NivaApiFunction = (...args: any[]) => Promise<any>
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
      clipboard: { [method: string]: NivaApiFunction },
      dialog: { [method: string]: NivaApiFunction },
      extra: { [method: string]: NivaApiFunction },
      fs: { [method: string]: NivaApiFunction },
      http: { [method: string]: NivaApiFunction },
      monitor: { [method: string]: NivaApiFunction },
      os: { [method: string]: NivaApiFunction },
      process: { [method: string]: NivaApiFunction },
      resource: { [method: string]: NivaApiFunction },
      short: { [method: string]: NivaApiFunction },
      tray: { [method: string]: NivaApiFunction },
      webview: { [method: string]: NivaApiFunction },
      window: { [method: string]: NivaApiFunction },
      windowExtra: { [method: string]: NivaApiFunction },
    };
  };
}

export { }
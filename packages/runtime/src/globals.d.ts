import type { NivaObj } from "./contracts";
export {};

declare global {
  var Niva: NivaObj;
  interface Window { Niva: NivaObj; }

  interface Error {
    code?: string;
    bridgeCode?: number;
    bridgeResult?: unknown;
    context?: unknown;
  }

  interface PropertyDescriptor {
    __proto__?: null | object;
  }

  interface ErrorConstructor {
    captureStackTrace?: (target: object, constructor?: Function) => void;
    stackTraceLimit?: number;
  }
}

declare global {
  var TauriLite: {
    addEventListener(
      event: string,
      callback: (event: string, data: any) => any
    ): void;

    removeEventListener(
      event: string,
      callback: (event: string, data: any) => any
    ): void;

    call(method: string, data: any): Promise<any>;

    api: any;
  };
}

export {}
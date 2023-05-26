import { pathJoin } from "../common/utils";
import type { ProgressModel } from "../models/modal.model";
import type { ProjectModel } from "../models/project.model";

const { fs } = Niva.api;

export const indexesKey = "RESOURCE_INDEXES";
export const dataKey = "RESOURCE_DATA";
type FileIndex = Record<string, [number, number]>;
export interface BuildParams {
  project: ProjectModel,
  progress: ProgressModel,
  file?: any
}

export async function packageResource(
  projectResourcePath: string,
  fileIndex: FileIndex = {},
  buffer: ArrayBuffer = new ArrayBuffer(0)
) {
  for (const name of await fs.readDirAll(projectResourcePath)) {
    const filePath = pathJoin(projectResourcePath, name);
    const fileKey = name.replace(/\\/g, "/");
    const [newFileIndex, newBuffer] = await appendResource(
      filePath,
      fileKey,
      fileIndex,
      buffer
    );
    buffer = newBuffer;
    fileIndex = newFileIndex;
  }
  return [fileIndex, buffer] as const;
}

export async function appendResource(
  filePath: string,
  fileKey: string,
  fileIndex: FileIndex = {},
  buffer: ArrayBuffer = new ArrayBuffer(0)
): Promise<[FileIndex, ArrayBuffer]> {
  const fileBuffer = base64ToArrayBuffer(await fs.read(filePath, "base64"));
  return [
    {
      ...fileIndex,
      [fileKey]: [buffer.byteLength, fileBuffer.byteLength],
    },
    concatArrayBuffers(buffer, fileBuffer),
  ];
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
  let binary = "";
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
  new Uint8Array(newBuffer, buffer1.byteLength, buffer2.byteLength).set(
    new Uint8Array(buffer2)
  );
  return newBuffer;
}

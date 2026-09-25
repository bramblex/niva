import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import process from "node:process";

export { fs, os, path, process };

export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

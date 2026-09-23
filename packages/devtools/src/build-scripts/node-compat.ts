import { dirname, pathJoin } from "../common/utils";
import { resolveNodeCompatAssets } from "./node-compat-assets.mjs";

export type NodeCompatAssetSelection = ReturnType<typeof resolveNodeCompatAssets>;

export interface StagedNodeCompatFiles {
  directory: string;
  files: Array<{ path: string; key: string }>;
}

function nativePath(root: string, relativePath: string) {
  return pathJoin(root, ...relativePath.split("/"));
}

export async function extractNodeCompatFiles(
  option: unknown,
  directory: string,
): Promise<StagedNodeCompatFiles | null> {
  const selection = resolveNodeCompatAssets(option as Parameters<typeof resolveNodeCompatAssets>[0]);
  if (!selection.enabled) return null;

  const { fs, resource } = Niva.api;
  if (await fs.exists(directory)) await fs.remove(directory);
  await fs.createDirAll(directory);

  const files: StagedNodeCompatFiles["files"] = [];
  for (const relativePath of selection.files) {
    const destination = nativePath(directory, relativePath);
    await fs.createDirAll(dirname(destination));
    await resource.extract(`__niva_compat/${relativePath}`, destination);
    files.push({ path: destination, key: `__niva_compat/${relativePath}` });
  }

  return { directory, files };
}

export async function stageProjectResourcesWithNodeCompat(
  projectResourcePath: string,
  stagingPath: string,
  option: unknown,
): Promise<void> {
  const selection = resolveNodeCompatAssets(option as Parameters<typeof resolveNodeCompatAssets>[0]);
  if (!selection.enabled) {
    throw new Error("NodeCompat resource staging requires nodeCompat to be enabled");
  }

  const { fs, resource } = Niva.api;
  if (await fs.exists(stagingPath)) await fs.remove(stagingPath);
  await fs.createDirAll(stagingPath);

  for (const relativePath of await fs.readDirAll(projectResourcePath)) {
    const normalizedPath = relativePath.replace(/\\/g, "/");
    const source = nativePath(projectResourcePath, normalizedPath);
    const destination = nativePath(stagingPath, normalizedPath);
    await fs.createDirAll(dirname(destination));
    await fs.copy(source, destination, { overwrite: true });
  }

  for (const relativePath of selection.files) {
    const destination = nativePath(stagingPath, relativePath);
    await fs.createDirAll(dirname(destination));
    await resource.extract(`__niva_compat/${relativePath}`, destination);
  }
}

export { resolveNodeCompatAssets };

export const niva = Niva;

/** `process.open` remains a Niva-only action; Node's `process` has no equivalent. */
export async function openExternal(uri: string): Promise<void> {
  await niva.bridge.call("process.open", [uri]);
}

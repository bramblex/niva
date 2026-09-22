import {
  useLocalModel as useLocalModelBase,
  useModel as useModelBase,
} from "@bramblex/state-model/react";

/**
 * Subscribe a component to a StateModel.
 *
 * NOTE: @bramblex/state-model v2 ships its .ts sources next to the .d.ts
 * files and has no "exports" map, so under `moduleResolution: bundler` the
 * `StateModel` seen through `@bramblex/state-model/react` (./index.js ->
 * ./index.ts) is a different *declaration* from the one seen through the
 * package root (index.d.ts). The two declarations disagree on private fields
 * and are not mutually assignable, even though they are the same class at
 * runtime. These adapters absorb that packaging quirk in one place; if the
 * library fixes its packaging, delete this file and import from the library
 * directly.
 */
export function useModel(model: unknown): void {
  (useModelBase as (m: unknown) => void)(model);
}

export function useLocalModel<M>(creator: () => M): M {
  return (useLocalModelBase as (c: () => M) => M)(creator);
}

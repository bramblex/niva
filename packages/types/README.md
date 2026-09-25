# @niva/types

`@niva/types` publishes declarations generated from the canonical contracts in
`packages/runtime/src/contracts.ts`. The package root describes the browser
page API and deliberately does not add Node globals or public `node:*` ambient
modules. The pinned `@types/node@22.14.0` declaration closure is AST-transformed
into private modules and private helper types; its snapshot and license are
included in the package.

Import the package root for the `Niva` global and `NivaOptions`:

```ts
import type {} from "@niva/types";

const fs = Niva.fs;
const snapshot = Niva.os.info;
```

Import module contracts from `@niva/types/contracts` when defining helpers or
adapters. Module-specific type facades are available through subpaths such as
`@niva/types/fs`, `@niva/types/http` and `@niva/types/module`.

Choose one declaration mode per TypeScript program:

These are type-only imports so they do not add a runtime package dependency.

- **Browser page (default):** `import type {} from "@niva/types"`. This adds `Niva` and
  `NivaOptions`, but no ambient `process`, `require`, `Buffer`, `setImmediate`,
  `NodeJS`, or `node:*` modules. `@types/node` is an optional peer, not a
  dependency, so it is not installed or auto-discovered by browser consumers.
- **Niva CommonJS page:** `import type {} from "@niva/types/commonjs"` when
  `injectCommonJs` is enabled. This adds the CommonJS globals Niva injects,
  including `process`, `require`, `module`, `exports`, `global`, `__filename`,
  `__dirname`, `Buffer`, and `setImmediate`. This mode is mutually exclusive
  with a TypeScript program that loads standard Node globals through
  `types: ["node"]`.
- **Native Node tooling:** `import type {} from "@niva/types/node"` with
  `@types/node@22.14.0` installed and `types: ["node"]`. It uses the standard
  Node declarations directly and preserves exact Node signatures. Use this
  mode for tools running in Node, not for browser pages.

Runtime global injection is separate from declarations: `injectCommonJs` and
`injectEsm` control runtime globals and import maps. These declarations describe
the supported Niva surface; they do not claim full Node.js compatibility.

Build and validate the generated declarations from the repository root:

```sh
npm run build --workspace=packages/runtime
npm run typecheck --workspace=@niva/types
npm run typecheck:consumers --workspace=@niva/types
npm run test:fresh-pack --workspace=@niva/types
npm pack --dry-run --workspace=@niva/types
```

The runtime build calls the private-type snapshot generator after emitting its
declarations. To run that step directly, pass the runtime declaration source,
runtime facade manifest, and package output directory:

```sh
node packages/types/scripts/build-private-types.mjs \
  --source-dir packages/runtime/dist/types/source \
  --runtime-files packages/runtime/runtime-files.json \
  --out-dir packages/types/dist
```

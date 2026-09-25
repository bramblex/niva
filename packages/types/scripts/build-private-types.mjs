#!/usr/bin/env node
import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import path from "node:path";
import fs from "node:fs";

const require = createRequire(import.meta.url);
const ts = require("typescript");
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptDir, "..");
const workspaceRoot = path.resolve(packageRoot, "../..");

function parseArgs(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith("--")) throw new Error(`Unexpected argument: ${key}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${key}`);
    values[key.slice(2)] = value;
    index += 1;
  }
  for (const required of ["source-dir", "runtime-files", "out-dir"]) {
    if (!values[required]) throw new Error(`Missing required --${required}`);
  }
  return values;
}

const args = parseArgs(process.argv.slice(2));
const fromWorkspace = (value) => path.resolve(workspaceRoot, value);
const sourceDir = fromWorkspace(args["source-dir"]);
const runtimeFilesPath = fromWorkspace(args["runtime-files"]);
const outDir = fromWorkspace(args["out-dir"]);

function isWithin(parent, candidate) {
  const relative = path.relative(parent, candidate);
  return relative === "" || (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative));
}

function walk(dir, callback) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(absolute, callback);
    else if (entry.isFile()) callback(absolute);
  }
}

function sourceDeclaration(source) {
  if (!source.startsWith("src/") || source.includes("..")) {
    throw new Error(`Invalid runtime declaration source: ${source}`);
  }
  const relative = source.slice("src/".length).replace(/\.ts$/, ".d.ts");
  const absolute = path.resolve(sourceDir, relative);
  if (!isWithin(sourceDir, absolute)) throw new Error(`Runtime declaration escaped its source root: ${source}`);
  if (!fs.existsSync(absolute)) throw new Error(`Missing emitted declaration: ${absolute}`);
  return absolute;
}

const runtimeFiles = JSON.parse(fs.readFileSync(runtimeFilesPath, "utf8"));
if (!runtimeFiles.packageRoot || !runtimeFiles.entries || Object.keys(runtimeFiles.entries).length === 0) {
  throw new Error("runtime-files.json must declare packageRoot and ESM entries");
}
const typesPackage = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"));
const missingExports = Object.keys(runtimeFiles.entries).filter((specifier) => !typesPackage.exports?.[`./${specifier}`]);
if (missingExports.length > 0) {
  throw new Error(`@niva/types package exports are missing runtime facades: ${missingExports.join(", ")}`);
}
const facadeInputs = [[".", runtimeFiles.packageRoot], ...Object.entries(runtimeFiles.entries)].map(([specifier, source]) => ({
  specifier,
  source,
  declaration: sourceDeclaration(source),
}));
const contractsSource = path.join(sourceDir, "contracts.d.ts");
if (!fs.existsSync(contractsSource)) throw new Error(`Missing emitted contracts declaration: ${contractsSource}`);

// @types/node is an optional peer for consumers, but the build must read the
// exact pinned snapshot from the workspace's development install.
const nodePackagePath = require.resolve("@types/node/package.json", { paths: [packageRoot, workspaceRoot] });
const nodePackage = JSON.parse(fs.readFileSync(nodePackagePath, "utf8"));
if (nodePackage.version !== "22.14.0") {
  throw new Error(`Expected pinned @types/node@22.14.0; found @types/node@${nodePackage.version}`);
}
const nodeRoot = path.dirname(nodePackagePath);
const nodeIndex = path.join(nodeRoot, "index.d.ts");
const undiciPackagePath = createRequire(nodePackagePath).resolve("undici-types/package.json");
const undiciPackage = JSON.parse(fs.readFileSync(undiciPackagePath, "utf8"));
const undiciRoot = path.dirname(undiciPackagePath);

const compilerOptions = {
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  strict: true,
  skipLibCheck: false,
  noEmit: true,
  types: [],
  lib: ["lib.esnext.d.ts", "lib.dom.d.ts", "lib.dom.iterable.d.ts"],
};

// The source undici-types files refer to the package-level "node" type
// library. Resolve those references to the already loaded pinned Node index so
// the generator checks one declaration graph instead of a second TS-version
// specific copy from @types/node/ts5.6.
const compilerHost = ts.createCompilerHost(compilerOptions);
const baseGetSourceFile = compilerHost.getSourceFile.bind(compilerHost);
compilerHost.resolveModuleNames = (moduleNames, containingFile, _reusedNames, redirectedReference, options) => moduleNames.map((specifier) => {
  // The runtime build deliberately rewrites declarations to the public package
  // subpath before this snapshot pass. Resolve that workspace self-reference
  // to the exact emitted contracts source while checking the declaration graph.
  if (specifier === "@niva/types/contracts") {
    return {
      resolvedFileName: contractsSource,
      extension: ts.Extension.Dts,
      isExternalLibraryImport: false,
    };
  }
  return ts.resolveModuleName(
    specifier,
    containingFile,
    options ?? compilerOptions,
    compilerHost,
    redirectedReference,
  ).resolvedModule;
});
compilerHost.getSourceFile = (fileName, languageVersion, onError, shouldCreateNewSourceFile) => {
  const absolute = path.resolve(fileName);
  if (isWithin(undiciRoot, absolute) && absolute.endsWith(".d.ts")) {
    let source = fs.readFileSync(absolute, "utf8");
    const reference = path.relative(path.dirname(absolute), nodeIndex).split(path.sep).join("/");
    source = source.replace(
      /\/\/\/\s*<reference\s+types=["']node["']\s*\/>/g,
      `/// <reference path="${reference}" />`,
    );
    return ts.createSourceFile(absolute, source, languageVersion, true);
  }
  return baseGetSourceFile(fileName, languageVersion, onError, shouldCreateNewSourceFile);
};

const publicDeclarationInputs = [contractsSource, ...facadeInputs.map((entry) => entry.declaration)];
const program = ts.createProgram([nodeIndex, ...publicDeclarationInputs], compilerOptions, compilerHost);
const checker = program.getTypeChecker();
const nodeSourceFiles = program.getSourceFiles().filter((source) => isWithin(nodeRoot, source.fileName) && source.fileName.endsWith(".d.ts"));
const undiciSourceFiles = program.getSourceFiles().filter((source) => isWithin(undiciRoot, source.fileName) && source.fileName.endsWith(".d.ts"));

const allModuleNames = new Set();
for (const source of nodeSourceFiles) {
  const collect = (node) => {
    if (ts.isModuleDeclaration(node) && ts.isStringLiteral(node.name)) allModuleNames.add(node.name.text);
    ts.forEachChild(node, collect);
  };
  collect(source);
}
const nodeBases = new Set([...allModuleNames].filter((name) => name.startsWith("node:")).map((name) => name.slice(5)));

const stableGlobals = new Set([
  "Array", "ArrayLike", "ReadonlyArray", "ArrayBuffer", "ArrayBufferLike", "ArrayBufferView", "DataView",
  "Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array", "Int32Array", "Uint32Array",
  "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array", "Object", "Function", "String", "Number",
  "Boolean", "RegExp", "Date", "Error", "ErrorConstructor", "Symbol", "SymbolConstructor", "Promise", "PromiseLike",
  "Iterable", "Iterator", "IteratorObject", "AsyncIterable", "AsyncIterator", "AsyncIteratorObject", "Map", "ReadonlyMap",
  "Set", "ReadonlySet", "WeakMap", "WeakSet", "Disposable", "AsyncDisposable", "ReadonlyMapIterator", "MapIterator",
  "SetIterator", "ArrayIterator", "Generator", "AsyncGenerator", "AbortController", "AbortSignal", "Blob", "File", "Event",
  "EventTarget", "CloseEvent", "MessageEvent", "DOMException", "URL", "URLSearchParams", "Request", "RequestInit", "Response",
  "ResponseInit", "FormData", "Headers", "WebSocket", "EventSource", "ReadableStream", "WritableStream", "ReadableStreamDefaultReader",
  "ReadableStreamBYOBReader", "ReadableStreamDefaultController", "ReadableStreamBYOBRequest", "ReadableByteStreamController",
  "WritableStreamDefaultWriter", "WritableStreamDefaultController", "ByteLengthQueuingStrategy", "CountQueuingStrategy", "CompressionStream",
  "DecompressionStream", "TextDecoder", "TextEncoder", "TextDecoderStream", "TextEncoderStream", "TransformStream",
  "TransformStreamDefaultController", "Performance", "PerformanceEntry", "PerformanceMark", "PerformanceMeasure", "PerformanceObserver",
  "PerformanceObserverEntryList", "PerformanceResourceTiming", "Console", "console", "Storage", "localStorage", "sessionStorage",
  "structuredClone", "fetch", "atob", "btoa", "setTimeout", "clearTimeout", "setInterval", "clearInterval", "queueMicrotask",
]);

function isTypeScriptLib(fileName) {
  return /[/\\]typescript[/\\]lib[/\\]lib\.[^/\\]+\.d\.ts$/.test(fileName);
}

function hasLibraryMerge(statement) {
  if (ts.isVariableStatement(statement)) {
    return statement.declarationList.declarations.length > 0 && statement.declarationList.declarations.every((declaration) => {
      const symbol = checker.getSymbolAtLocation(declaration.name);
      return !!symbol?.declarations?.some((entry) => isTypeScriptLib(entry.getSourceFile().fileName));
    });
  }
  if (!statement.name) return false;
  const symbol = checker.getSymbolAtLocation(statement.name);
  return !!symbol?.declarations?.some((entry) => isTypeScriptLib(entry.getSourceFile().fileName));
}

function statementNames(statement) {
  if (statement.name && ts.isIdentifier(statement.name)) return [statement.name.text];
  if (ts.isVariableStatement(statement)) {
    return statement.declarationList.declarations.flatMap((declaration) => ts.isIdentifier(declaration.name) ? [declaration.name.text] : []);
  }
  return [];
}

function keepPrivateStatement(statement) {
  if (ts.isInterfaceDeclaration(statement) && statement.name.text === "ErrorConstructor") return false;
  const names = statementNames(statement);
  const timerNames = new Set(["setTimeout", "clearTimeout", "setInterval", "clearInterval", "queueMicrotask"]);
  if (names.length && names.every((name) => stableGlobals.has(name)) && hasLibraryMerge(statement) && !names.some((name) => timerNames.has(name))) return false;
  return true;
}

function isNodeOrUndici(source) {
  return isWithin(nodeRoot, source.fileName) || isWithin(undiciRoot, source.fileName);
}

function nearestGlobalBlock(declaration) {
  let node = declaration.parent;
  while (node) {
    if (ts.isModuleDeclaration(node) && ts.isIdentifier(node.name) && node.name.text === "global") return node;
    if (ts.isModuleDeclaration(node) && ts.isStringLiteral(node.name)) return undefined;
    node = node.parent;
  }
  return undefined;
}

function isDirectGlobalDeclaration(declaration) {
  if (!declaration || !isWithin(nodeRoot, declaration.getSourceFile().fileName)) return false;
  const globalBlock = nearestGlobalBlock(declaration);
  if (globalBlock && ts.isModuleBlock(globalBlock.body)) {
    let top = declaration;
    while (top.parent && top.parent !== globalBlock.body) top = top.parent;
    if (top === declaration) return true;
    return ts.isVariableDeclaration(declaration) && ts.isVariableStatement(top) && top.declarationList.declarations.includes(declaration);
  }
  const source = declaration.getSourceFile();
  return !ts.isExternalModule(source) && declaration.parent === source;
}

function isPrivateGlobalSymbol(symbol) {
  return !!(symbol?.declarations?.some(isDirectGlobalDeclaration) && !stableGlobals.has(symbol.getName()));
}

function isDeclarationName(identifier) {
  const parent = identifier.parent;
  return !!(parent && parent.name === identifier && (
    ts.isDeclaration(parent) || ts.isModuleDeclaration(parent) || ts.isImportSpecifier(parent) || ts.isExportSpecifier(parent) ||
    ts.isTypeParameterDeclaration(parent) || ts.isParameter(parent) || ts.isPropertySignature(parent) || ts.isMethodSignature(parent) ||
    ts.isPropertyDeclaration(parent) || ts.isMethodDeclaration(parent) || ts.isEnumMember(parent)
  ));
}

function shouldPrivate(identifier) {
  if (!ts.isIdentifier(identifier) || isDeclarationName(identifier)) return false;
  const symbol = checker.getSymbolAtLocation(identifier);
  return isPrivateGlobalSymbol(symbol) || (
    identifier.text === "BufferEncoding" && !!symbol?.declarations?.some((declaration) => nearestGlobalBlock(declaration))
  );
}

function rootEntity(entity) {
  return ts.isIdentifier(entity) ? entity : ts.isQualifiedName(entity) ? rootEntity(entity.left) : undefined;
}

function qualifyEntity(entity) {
  const root = rootEntity(entity);
  if (!root || !shouldPrivate(root)) return entity;
  const privateRoot = ts.factory.createQualifiedName(ts.factory.createIdentifier("NivaInternal"), root);
  function replace(node) {
    return ts.isIdentifier(node) ? privateRoot : ts.factory.updateQualifiedName(node, replace(node.left), node.right);
  }
  return replace(entity);
}

function qualifyExpression(expression) {
  if (ts.isPropertyAccessExpression(expression) && ts.isIdentifier(expression.expression) && expression.expression.text === "globalThis" && privateGlobalNames.has(expression.name.text)) {
    return ts.factory.createPropertyAccessExpression(ts.factory.createIdentifier("NivaInternal"), expression.name.text);
  }
  const root = ts.isIdentifier(expression) ? expression : ts.isPropertyAccessExpression(expression)
    ? (function findRoot(node) { return ts.isIdentifier(node.expression) ? node.expression : findRoot(node.expression); })(expression)
    : undefined;
  if (!root || !shouldPrivate(root)) return expression;
  const privateRoot = ts.factory.createPropertyAccessExpression(ts.factory.createIdentifier("NivaInternal"), root.text);
  function replace(node) {
    if (ts.isIdentifier(node)) return privateRoot;
    if (ts.isPropertyAccessExpression(node)) return ts.factory.updatePropertyAccessExpression(node, replace(node.expression), node.name);
    return node;
  }
  return replace(expression);
}

const privateGlobalNames = new Set();
function declarationNames(statement) {
  if (statement.name && ts.isIdentifier(statement.name)) return [statement.name];
  if (ts.isVariableStatement(statement)) return statement.declarationList.declarations.map((entry) => entry.name).filter(ts.isIdentifier);
  return [];
}

function collectMovedGlobalStatements(source) {
  const moved = [];
  function visit(node) {
    if (ts.isModuleDeclaration(node) && ts.isIdentifier(node.name) && node.name.text === "global" && ts.isModuleBlock(node.body)) {
      for (const statement of node.body.statements) if (isDeclarationStatement(statement) && keepPrivateStatement(statement)) moved.push(statement);
    }
    ts.forEachChild(node, visit);
  }
  visit(source);
  if (!ts.isExternalModule(source)) {
    for (const statement of source.statements) {
      if (ts.isModuleDeclaration(statement) && ts.isStringLiteral(statement.name)) continue;
      if (isDeclarationStatement(statement) && keepPrivateStatement(statement)) moved.push(statement);
    }
  }
  return moved;
}

for (const source of nodeSourceFiles) {
  for (const statement of collectMovedGlobalStatements(source)) {
    for (const name of declarationNames(statement)) privateGlobalNames.add(name.text);
  }
}

function mapModuleSpecifier(specifier) {
  if (specifier === "undici-types") return "niva-internal-undici-types";
  if (specifier === "sys") return "niva-internal:util";
  if (specifier === "node:sys") return "niva-internal:node:util";
  if (specifier.startsWith("node:")) return `niva-internal:${specifier}`;
  if (specifier === "./contracts") return "@niva/types/contracts";
  if (allModuleNames.has(specifier) || nodeBases.has(specifier)) return `niva-internal:${specifier}`;
  return specifier;
}

function mapLiteral(literal) {
  return ts.factory.createStringLiteral(mapModuleSpecifier(literal.text));
}

function modifiersOf(node) {
  return ts.canHaveModifiers(node) ? (ts.getModifiers(node) || []) : [];
}

function addExport(node) {
  const modifiers = modifiersOf(node).filter((modifier) => modifier.kind !== ts.SyntaxKind.DeclareKeyword);
  if (modifiers.some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword)) return ts.factory.replaceModifiers(node, modifiers);
  return ts.factory.replaceModifiers(node, [ts.factory.createModifier(ts.SyntaxKind.ExportKeyword), ...modifiers]);
}

function makePrivateNamespace(statements, topLevel) {
  const body = ts.factory.createModuleBlock(statements.map(addExport));
  const modifiers = topLevel ? [ts.factory.createModifier(ts.SyntaxKind.DeclareKeyword)] : undefined;
  return ts.factory.createModuleDeclaration(modifiers, ts.factory.createIdentifier("NivaInternal"), body, ts.NodeFlags.Namespace);
}

function isDeclarationStatement(statement) {
  return ts.isInterfaceDeclaration(statement) || ts.isTypeAliasDeclaration(statement) || ts.isModuleDeclaration(statement) ||
    ts.isVariableStatement(statement) || ts.isFunctionDeclaration(statement) || ts.isClassDeclaration(statement) || ts.isEnumDeclaration(statement) ||
    ts.isImportEqualsDeclaration(statement) || ts.isExportDeclaration(statement) || ts.isExportAssignment(statement);
}

function transformSource(source) {
  return (context) => {
    const factory = ts.factory;
    function visit(node) {
      if (ts.isModuleDeclaration(node)) {
        if (ts.isStringLiteral(node.name)) {
          const name = mapLiteral(node.name);
          const body = node.body ? ts.visitNode(node.body, visit) : node.body;
          return factory.updateModuleDeclaration(node, node.modifiers, name, body);
        }
        if (isNodeOrUndici(source) && ts.isIdentifier(node.name) && node.name.text === "global" && ts.isModuleBlock(node.body)) {
          const moved = [];
          for (const statement of node.body.statements) {
            if (isDeclarationStatement(statement) && keepPrivateStatement(statement)) moved.push(ts.visitEachChild(statement, visit, context));
          }
          return moved.length
            ? factory.updateModuleDeclaration(node, node.modifiers, node.name, factory.createModuleBlock([makePrivateNamespace(moved, false)]))
            : undefined;
        }
        const body = node.body ? ts.visitNode(node.body, visit) : node.body;
        return factory.updateModuleDeclaration(node, node.modifiers, node.name, body);
      }
      if (ts.isSourceFile(node) && isNodeOrUndici(source) && !ts.isExternalModule(node)) {
        const kept = [];
        const moved = [];
        for (const statement of node.statements) {
          if (ts.isModuleDeclaration(statement) && ts.isStringLiteral(statement.name)) kept.push(ts.visitNode(statement, visit));
          else if (ts.isModuleDeclaration(statement) && ts.isIdentifier(statement.name) && statement.name.text === "global") kept.push(ts.visitNode(statement, visit));
          else if (isDeclarationStatement(statement) && keepPrivateStatement(statement)) moved.push(ts.visitNode(statement, visit));
          else if (!isDeclarationStatement(statement)) kept.push(ts.visitNode(statement, visit));
        }
        if (moved.length) kept.push(makePrivateNamespace(moved, true));
        return factory.updateSourceFile(node, kept);
      }
      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
        return factory.updateImportDeclaration(node, node.modifiers, node.importClause, mapLiteral(node.moduleSpecifier), node.attributes);
      }
      if (ts.isExportDeclaration(node)) {
        if (!node.moduleSpecifier && node.exportClause && ts.isNamedExports(node.exportClause)) {
          const names = node.exportClause.elements.map((element) => element.name.text);
          if (names.includes("Buffer") && isNodeOrUndici(source) && path.basename(source.fileName) === "buffer.d.ts") {
            return factory.createImportEqualsDeclaration(
              [factory.createModifier(ts.SyntaxKind.ExportKeyword)], false, factory.createIdentifier("Buffer"),
              factory.createQualifiedName(factory.createIdentifier("NivaInternal"), factory.createIdentifier("Buffer")),
            );
          }
          if (names.length === 1 && names[0] === "BufferEncoding" && isNodeOrUndici(source) && path.basename(source.fileName) === "buffer.d.ts") {
            return factory.createTypeAliasDeclaration(
              [factory.createModifier(ts.SyntaxKind.ExportKeyword)], factory.createIdentifier("BufferEncoding"), undefined,
              factory.createQualifiedName(factory.createIdentifier("NivaInternal"), factory.createIdentifier("BufferEncoding")),
            );
          }
        }
        const specifier = node.moduleSpecifier && ts.isStringLiteral(node.moduleSpecifier) ? mapLiteral(node.moduleSpecifier) : node.moduleSpecifier;
        const clause = node.exportClause ? ts.visitNode(node.exportClause, visit) : node.exportClause;
        return factory.updateExportDeclaration(node, node.modifiers, node.isTypeOnly, clause, specifier, node.attributes);
      }
      if (ts.isImportTypeNode(node)) {
        let argument = node.argument;
        if (ts.isLiteralTypeNode(argument) && ts.isStringLiteral(argument.literal)) argument = factory.updateLiteralTypeNode(argument, mapLiteral(argument.literal));
        else argument = ts.visitNode(argument, visit);
        const qualifier = node.qualifier ? qualifyEntity(node.qualifier) : node.qualifier;
        return factory.updateImportTypeNode(node, argument, node.attributes, qualifier, node.typeArguments ? ts.visitNodes(node.typeArguments, visit) : node.typeArguments, node.isTypeOf);
      }
      if (ts.isTypeReferenceNode(node)) {
        return factory.updateTypeReferenceNode(node, qualifyEntity(node.typeName), node.typeArguments ? ts.visitNodes(node.typeArguments, visit) : node.typeArguments);
      }
      if (ts.isTypeQueryNode(node)) return factory.updateTypeQueryNode(node, qualifyEntity(node.exprName), node.typeArguments ? ts.visitNodes(node.typeArguments, visit) : node.typeArguments);
      if (ts.isExpressionWithTypeArguments(node)) return factory.updateExpressionWithTypeArguments(node, qualifyExpression(node.expression), node.typeArguments ? ts.visitNodes(node.typeArguments, visit) : node.typeArguments);
      if (ts.isExportAssignment(node)) return factory.updateExportAssignment(node, node.modifiers, qualifyExpression(node.expression));
      if (ts.isImportEqualsDeclaration(node) && ts.isQualifiedName(node.moduleReference) && ts.isIdentifier(node.moduleReference.left) && node.moduleReference.left.text === "globalThis" && privateGlobalNames.has(node.moduleReference.right.text)) {
        return factory.updateImportEqualsDeclaration(node, node.modifiers, node.isTypeOnly, node.name, factory.createQualifiedName(factory.createIdentifier("NivaInternal"), node.moduleReference.right));
      }
      if (ts.isExternalModuleReference(node) && node.expression && ts.isStringLiteral(node.expression)) {
        return factory.updateExternalModuleReference(node, mapLiteral(node.expression));
      }
      return ts.visitEachChild(node, visit, context);
    }
    return (sourceFile) => ts.visitNode(sourceFile, visit);
  };
}

function writeTransformed(source, relativeRoot, destinationRoot, outputRelative) {
  const relative = outputRelative || path.relative(relativeRoot, source.fileName);
  const destination = path.join(destinationRoot, relative);
  fs.mkdirSync(path.dirname(destination), { recursive: true });
  const result = ts.transform(source, [transformSource(source)]);
  const text = ts.createPrinter({ newLine: ts.NewLineKind.LineFeed, removeComments: false }).printFile(result.transformed[0]);
  result.dispose();
  fs.writeFileSync(destination, text);
}

const diagnostics = ts.getPreEmitDiagnostics(program);
if (diagnostics.length > 0) {
  throw new Error(`Declaration snapshot source check failed:\n${ts.formatDiagnosticsWithColorAndContext(diagnostics, {
    getCurrentDirectory: () => workspaceRoot,
    getCanonicalFileName: (fileName) => fileName,
    getNewLine: () => "\n",
  })}`);
}
if (nodeSourceFiles.length === 0 || undiciSourceFiles.length === 0) throw new Error("Pinned Node declaration closure was not resolved");
if (nodeSourceFiles.some((source) => source.fileName.includes(`${path.sep}ts5.6${path.sep}`))) {
  throw new Error("Unexpected TypeScript 5.6 fallback declarations in the pinned TypeScript 5.9 build");
}

fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });
const nodeTypesOut = path.join(outDir, "node-types");
const undiciOut = path.join(nodeTypesOut, "node_modules/niva-internal-undici-types");
for (const source of nodeSourceFiles) writeTransformed(source, nodeRoot, nodeTypesOut);
for (const source of undiciSourceFiles) writeTransformed(source, undiciRoot, undiciOut);
const contractSourceFile = program.getSourceFile(contractsSource);
if (!contractSourceFile) throw new Error(`Missing parsed contracts declaration: ${contractsSource}`);
writeTransformed(contractSourceFile, sourceDir, outDir, "contracts.d.ts");
for (const entry of facadeInputs) {
  const source = program.getSourceFile(entry.declaration);
  if (!source) throw new Error(`Missing parsed facade declaration: ${entry.declaration}`);
  const route = entry.specifier === "." ? "index.d.ts" : `${entry.specifier}.d.ts`;
  writeTransformed(source, sourceDir, path.join(outDir, "modules"), route);
}

for (const fileName of ["LICENSE", "README.md"]) {
  const nodeFile = path.join(nodeRoot, fileName);
  if (fs.existsSync(nodeFile)) fs.copyFileSync(nodeFile, path.join(nodeTypesOut, fileName));
  const undiciFile = path.join(undiciRoot, fileName);
  if (fs.existsSync(undiciFile)) fs.copyFileSync(undiciFile, path.join(undiciOut, fileName));
}
const undiciOutputPackage = JSON.parse(fs.readFileSync(undiciPackagePath, "utf8"));
undiciOutputPackage.name = "niva-internal-undici-types";
fs.writeFileSync(path.join(undiciOut, "package.json"), `${JSON.stringify(undiciOutputPackage, null, 2)}\n`);
walk(undiciOut, (file) => {
  if (!file.endsWith(".d.ts")) return;
  let text = fs.readFileSync(file, "utf8");
  text = text
    .replace(/\/\/\/\s*<reference\s+path=["'][^"']*@types\/node\/index\.d\.ts["']\s*\/>/g, '/// <reference path="../../index.d.ts" />')
    .replace(/\/\/\/\s*<reference\s+types=["']node["']\s*\/>/g, '/// <reference path="../../index.d.ts" />');
  fs.writeFileSync(file, text);
});

const contractsNodePath = path.join(outDir, "contracts-node.d.ts");
fs.copyFileSync(contractsSource, contractsNodePath);
const browserFacade = path.join(outDir, "Niva_zh.d.ts");
fs.writeFileSync(browserFacade, [
  '/// <reference path="./node-types/index.d.ts" />',
  'import type { NivaObj, NivaOptions as NivaOptionsContract } from "./contracts";',
  "declare global {",
  "    var Niva: NivaObj;",
  "    interface Window { Niva: NivaObj; }",
  "    type NivaOptions = NivaOptionsContract;",
  "}",
  'export type * from "./contracts";',
  "export {};",
  "",
].join("\n"));
const nodeFacade = path.join(outDir, "Niva_node.d.ts");
fs.writeFileSync(nodeFacade, [
  '/// <reference types="node" />',
  'import type { NivaObj, NivaOptions as NivaOptionsContract } from "./contracts-node";',
  "declare global {",
  "    var Niva: NivaObj;",
  "    interface Window { Niva: NivaObj; }",
  "    type NivaOptions = NivaOptionsContract;",
  "}",
  'export type * from "./contracts-node";',
  "export {};",
  "",
].join("\n"));
const commonjsFacade = path.join(outDir, "Niva_commonjs.d.ts");
fs.writeFileSync(commonjsFacade, [
  '/// <reference path="./Niva_zh.d.ts" />',
  "declare global {",
  "    var process: NivaInternal.NodeJS.Process;",
  "    var require: NivaInternal.NodeJS.Require;",
  "    var module: NivaInternal.NodeJS.Module;",
  '    var exports: NivaInternal.NodeJS.Module["exports"];',
  "    var global: typeof globalThis;",
  "    var __filename: string;",
  "    var __dirname: string;",
  "    var Buffer: NivaInternal.BufferConstructor;",
  "    type Buffer = NivaInternal.Buffer;",
  "    type BufferEncoding = NivaInternal.BufferEncoding;",
  '    var setImmediate: typeof import("niva-internal:node:timers").setImmediate;',
  "}",
  "export {};",
  "",
].join("\n"));

const hash = (value) => createHash("sha256").update(value).digest("hex");
const snapshotFiles = [...nodeSourceFiles, ...undiciSourceFiles].map((source) => ({
  path: isWithin(nodeRoot, source.fileName)
    ? `@types/node/${path.relative(nodeRoot, source.fileName).split(path.sep).join("/")}`
    : `undici-types/${path.relative(undiciRoot, source.fileName).split(path.sep).join("/")}`,
  sha256: hash(fs.readFileSync(source.fileName)),
})).sort((left, right) => left.path.localeCompare(right.path));
const snapshot = {
  format: 1,
  nodeTypes: { package: "@types/node", version: nodePackage.version, license: nodePackage.license },
  undiciTypes: { package: "undici-types", version: undiciPackage.version, license: undiciPackage.license },
  sourceFiles: snapshotFiles,
  digest: hash(snapshotFiles.map((file) => `${file.path}:${file.sha256}`).join("\n")),
};
fs.writeFileSync(path.join(nodeTypesOut, "SNAPSHOT.json"), `${JSON.stringify(snapshot, null, 2)}\n`);
console.log(JSON.stringify({
  output: outDir,
  nodeTypesVersion: nodePackage.version,
  undiciTypesVersion: undiciPackage.version,
  privateNodeModules: allModuleNames.size,
  snapshotFiles: snapshotFiles.length,
  snapshotDigest: snapshot.digest,
  sourceDiagnostics: diagnostics.length,
}, null, 2));

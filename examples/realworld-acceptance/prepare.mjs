// Prepare a pinned upstream application's original TS sources for execution.
// This is a build tool, not the application's runtime or a Node fallback.
import { readFile, writeFile, mkdir, cp, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { transform } from 'esbuild';

const here = path.dirname(fileURLToPath(import.meta.url));
const manifest = JSON.parse(await readFile(path.join(here, 'manifest.json'), 'utf8'));
const source = process.argv[2] && path.resolve(process.argv[2]);
if (!source) throw new Error('Usage: node examples/realworld-acceptance/prepare.mjs <upstream-checkout>');
const commit = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: source, encoding: 'utf8' }).trim();
if (commit !== manifest.commit) throw new Error(`Expected upstream ${manifest.commit}, got ${commit}`);
const diff = execFileSync('git', ['diff', '--name-only', 'HEAD'], { cwd: source, encoding: 'utf8' }).trim();
if (diff) throw new Error(`Upstream tracked files must remain unchanged: ${diff}`);
const output = path.join(source, '.niva-compiled');
await mkdir(output, { recursive: true });
const inputs = {};
async function compileTree(relative) {
  const root = path.join(source, relative);
  for (const item of await readdir(root, { withFileTypes: true })) {
    const rel = path.join(relative, item.name);
    if (item.isDirectory()) { await compileTree(rel); continue; }
    if (!item.isFile()) throw new Error(`Unexpected upstream entry ${rel}`);
    const data = await readFile(path.join(source, rel));
    inputs[rel.replaceAll(path.sep, '/')] = createHash('sha256').update(data).digest('hex');
    if (rel.endsWith('.d.ts')) continue;
    let destination = path.join(output, rel);
    if (/\.tsx?$/.test(rel)) {
      destination = destination.replace(/\.tsx?$/, '.js');
      const result = await transform(data.toString('utf8'), {
        loader: rel.endsWith('.tsx') ? 'tsx' : 'ts',
        format: 'cjs', target: 'es2020', sourcefile: rel,
        sourcemap: 'inline',
      });
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, result.code);
    } else {
      await mkdir(path.dirname(destination), { recursive: true });
      await writeFile(destination, data);
    }
  }
}
for (const root of ['backend', 'src/models', 'src/utils']) await compileTree(root);
// Upstream's CI supplies this same mock configuration. Local auth remains the
// real application's auth; optional Cognito is not exercised or replaced.
const mockConfig = await readFile(path.join(source, 'scripts/mock-aws-exports.js'), 'utf8');
inputs['scripts/mock-aws-exports.js'] = createHash('sha256').update(mockConfig).digest('hex');
await writeFile(path.join(output, 'src/aws-exports.js'), (await transform(mockConfig, {
  loader: 'js', format: 'cjs', target: 'es2020', sourcefile: 'scripts/mock-aws-exports.js',
})).code);
await cp(path.join(source, 'data'), path.join(output, 'data'), { recursive: true });
await cp(path.join(source, 'public'), path.join(output, 'public'), { recursive: true });
await cp(path.join(source, '.env'), path.join(output, '.env'));
const lockSha256 = createHash('sha256').update(await readFile(path.join(source, manifest.dependencyLock))).digest('hex');
await writeFile(path.join(output, 'build-manifest.json'), JSON.stringify({
  ...manifest, actualCommit: commit, dependencyLockSha256: lockSha256,
  buildNode: process.version, sourceRoot: source, outputRoot: output,
  transformed: 'esbuild transform only; imports preserved as require', inputs,
}, null, 2) + '\n');
console.log(output);

import "./setup-runtime.js";
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const dist=fileURLToPath(new URL('../dist/',import.meta.url));
const manifest=JSON.parse(readFileSync(path.join(dist,'runtime-assets.json'),'utf8'));
test('each selected builtin serves the full relative ESM import closure',()=>{
  for(const [module,assets] of Object.entries(manifest.modules)){
    const allowed=new Set([...manifest.shared,...assets]);
    for(const file of assets){
      const source=readFileSync(path.join(dist,file),'utf8');
      for(const match of source.matchAll(/(?:import\s*|from\s*)['"](\.[^'"]+)['"]/g)){
        const dependency=path.posix.normalize(path.posix.join(path.posix.dirname(file),match[1]));
        assert(allowed.has(dependency),`${module}: ${file} imports unserved ${dependency}`);
      }
    }
  }
});

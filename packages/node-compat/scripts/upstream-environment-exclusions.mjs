// Test adaptation only. The pinned upstream snapshot is never edited.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
export const policySha256 = '607ac4076ac99a8915c452071f37b3429db78d5cecb8eb0353cb05a5cd9092f1';
const hash = value => createHash('sha256').update(value).digest('hex');
const policyBytes=readFileSync(new URL('../upstream/environment-exclusions.json',import.meta.url));
assert.equal(hash(policyBytes),policySha256,'Environment exclusions changed; explicit scope review required');
export const environmentPolicy=JSON.parse(policyBytes);
assert.equal(environmentPolicy.schemaVersion,1);
assert.equal(environmentPolicy.upstream,'v22.14.0');
assert.equal(environmentPolicy.entries.length,2);
assert.equal(new Set(environmentPolicy.entries.map(row=>row.id)).size,2);

export function prepareEnvironmentExclusions(file,original,enabled=true) {
  const entries=enabled?environmentPolicy.entries.filter(row=>row.file===file):[];
  assert(!original.includes('__nivaSkip'),'Reserved exclusion marker exists in original source');
  let source=original;
  for(let index=entries.length-1;index>=0;index--) {
    const row=entries[index];
    assert.equal(hash(original),row.sourceSha256,'Exclusion source checksum mismatch');
    assert.equal(original.slice(row.start,row.end),row.statement,'Exclusion no longer points to the approved statement');
    const marker=`__nivaSkip(${index});`;
    const blank=row.statement.replace(/[^\r\n]/g,' ');
    assert(marker.length<=(blank.indexOf('\n')<0?blank.length:blank.indexOf('\n')),'Marker cannot preserve source locations');
    source=source.slice(0,row.start)+marker+blank.slice(marker.length)+source.slice(row.end);
  }
  assert.equal(source.length,original.length);
  assert.equal(source.split('\n').length,original.split('\n').length);
  return {source,entries,executedSourceSha256:hash(source)};
}

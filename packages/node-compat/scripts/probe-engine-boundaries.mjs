// Diagnostic only, not shipped: demonstrate what the public Web Crypto/JS APIs expose.
import assert from 'node:assert/strict';
import util from 'node:util';
import { writeFileSync } from 'node:fs';
assert.equal(process.version, 'v22.14.0');
const importKey = (byte, extractable = true) => crypto.subtle.importKey('raw', Uint8Array.of(byte), {name:'HMAC',hash:'SHA-256'}, extractable, ['sign']);
const [a,b,c,locked] = await Promise.all([importKey(0),importKey(1),importKey(0),importKey(0,false)]);
// Node's own Symbol(kKeyObject) is not part of Web Crypto; deliberately do not read it.
const publicKeyShape = key => ({ownNames:Object.getOwnPropertyNames(key),enumerableNames:Object.keys(key),tag:Object.prototype.toString.call(key),type:key.type,extractable:key.extractable,algorithm:key.algorithm,usages:key.usages});
assert.deepEqual(publicKeyShape(a),publicKeyShape(b));
assert.equal(Object.getPrototypeOf(a),Object.getPrototypeOf(b));
let nonextractableExportError;
try { await crypto.subtle.exportKey('raw',locked);assert.fail('Nonextractable key was exported'); }
catch(error) { nonextractableExportError=error.name; }
class Foo {}
const removed=Object.setPrototypeOf(new Foo(),null), plain=Object.create(null);
const publicShape=value=>({keys:Reflect.ownKeys(value),prototype:Object.getPrototypeOf(value),tag:Object.prototype.toString.call(value),constructor:value.constructor});
assert.deepEqual(publicShape(removed),publicShape(plain));
const result={node:process.version,scope:'Public-observation counterexamples in the pinned Node oracle; not WebView runtime acceptance',cryptoKey:{samePublicShape:true,samePrototype:true,nodeDifferentMaterialEqual:util.isDeepStrictEqual(a,b),nodeSameMaterialEqual:util.isDeepStrictEqual(a,c),exportReturnsPromise:crypto.subtle.exportKey('raw',a) instanceof Promise,nonextractableExportError},removedPrototype:{samePublicShape:true,nodeClassLabel:util.format('%s',removed),nodePlainLabel:util.format('%s',plain)}};
assert.equal(result.cryptoKey.nodeDifferentMaterialEqual,false);
assert.equal(result.cryptoKey.nodeSameMaterialEqual,true);
assert.notEqual(result.removedPrototype.nodeClassLabel,result.removedPrototype.nodePlainLabel);
const output=JSON.stringify(result,null,2)+'\n';
if(process.argv[2])writeFileSync(process.argv[2],output);
console.log(output);

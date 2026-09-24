import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import '../src/runtime/bridge.js';
import '../src/runtime/vendor.js';
const source = readFileSync(new URL('../src/runtime/source.js', import.meta.url),'utf8');
const vendor = globalThis[Symbol.for('niva.node-compat.runtime')].vendor;

function browserFixture(filename, sourceText) {
  const requests=[];
  class StackError extends Error {
    constructor() {super();this.stack='callerFrame@niva://app/__niva_compat/node-compat.js:50:3\ngetAssertionExpression@niva://app/__niva_compat/node-compat.js:90:2\nassertValue@niva://app/__niva_compat/node-compat.js:150:8\n@'+filename+':1:1';}
    static captureStackTrace(target) {target.stack='';}
  }
  class XHR {
    open(method,url,async) {requests.push({method,url,async});}
    send() {this.status=200;this.responseText=sourceText;}
  }
  const context=vm.createContext({Error:StackError,Map,URL,XMLHttpRequest:XHR,location:{href:'niva://app/index.html'},[Symbol.for('niva.node-compat.runtime')]:{vendor}});
  vm.runInContext(source,context);
  return {api:context[Symbol.for('niva.node-compat.runtime')].source,requests};
}

test('assert source recovers JSC tail-eliminated frames and reads only same-origin scripts',()=>{
  const page=browserFixture('niva://app/app.js','assert.ok(missingFeature);');
  assert.equal(page.api.getAssertionExpression(function ok(){}),'assert.ok(missingFeature)');
  assert.deepEqual(page.requests,[{method:'GET',url:'niva://app/app.js',async:false}]);
  assert.equal(page.api.getAssertionExpression(function ok(){}),'assert.ok(missingFeature)');
  assert.equal(page.requests.length,1);
  const remote=browserFixture('https://untrusted.test/app.js','assert.ok(missingFeature);');
  assert.equal(remote.api.getAssertionExpression(function ok(){}),undefined);
  assert.equal(remote.requests.length,0);
});

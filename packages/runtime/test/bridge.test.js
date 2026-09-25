import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import {
  assert as esmAssert,
  buffer as esmBuffer,
  child_process as esmChildProcess,
  crypto as esmCrypto,
  events as esmEvents,
  fs as esmFs,
  os as esmOs,
  path as esmPath,
  querystring as esmQuerystring,
  module as esmModule,
  url as esmUrl,
  util as esmUtil,
  zlib as esmZlib,
} from "@niva/runtime";
import classicPath from "@niva/runtime/path";
import { Buffer as esmBufferClass } from "@niva/runtime/buffer";
import { spawn as esmSpawn } from "@niva/runtime/child_process";
import fsPromisesDefault from "@niva/runtime/fs/promises";
import { randomBytes } from "@niva/runtime/crypto";
import { EventEmitter } from "@niva/runtime/events";
import { readFile as esmReadFile } from "@niva/runtime/fs";
import { platform as esmPlatform } from "@niva/runtime/os";
import { parse as parseQuery } from "@niva/runtime/querystring";
import { URL as esmURL } from "@niva/runtime/url";
import { format as esmFormat } from "@niva/runtime/util";
import { gzip as esmGzip } from "@niva/runtime/zlib";
import strictAssertDefault from "@niva/runtime/assert/strict";
import moduleDefault, { Module as moduleConstructor } from "@niva/runtime/module";
import "../dist/source/index.js";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];
const hostGlobals = { process: globalThis.process, require: globalThis.require, module: globalThis.module, Buffer: globalThis.Buffer };

test("package ESM exports resolve and registration enforces the bridge version", () => {
  assert.equal(classicPath, esmPath);
  assert.equal(typeof esmFs.readFile, "function");
  assert.equal(typeof esmOs.platform, "function");
  assert.equal(typeof esmChildProcess.spawn, "function");
  assert.equal(typeof esmEvents.EventEmitter, "function");
  assert.equal(typeof esmUtil.format, "function");
  assert.equal(typeof esmQuerystring.parse, "function");
  assert.equal(typeof esmBuffer.Buffer, "function");
  assert.equal(typeof esmUrl.URL, "function");
  assert.equal(typeof esmCrypto.randomUUID, "function");
  assert.equal(typeof esmZlib.gzip, "function");
  assert.equal(esmBufferClass, esmBuffer.Buffer);
  assert.equal(EventEmitter, esmEvents.EventEmitter);
  assert.equal(typeof parseQuery, "function");
  assert.equal(typeof randomBytes, "function");
  assert.equal(typeof esmSpawn, "function");
  assert.equal(typeof esmReadFile, "function");
  assert.equal(typeof esmPlatform, "function");
  assert.equal(typeof esmURL, "function");
  assert.equal(typeof esmFormat, "function");
  assert.equal(typeof esmGzip, "function");
  assert.equal(fsPromisesDefault, esmFs.promises);
  assert.equal(strictAssertDefault, esmAssert.strict);
  assert.equal(esmModule, globalThis.Niva.module);
  assert.equal(moduleDefault, esmModule);
  assert.equal(moduleConstructor, esmModule.Module);
  assert.equal(typeof globalThis.Niva.bridge.call, "function");
  assert.equal(globalThis.Niva.api, undefined);
  assert.equal(globalThis.Niva.call, undefined);
});


test('fs uses native operations for callbacks, promises and synchronous binary data', async () => {
  const files=new Map(),calls=[];
  function dispatch(method,[op,args]){
    assert.equal(method,'fs.node');calls.push([op,args]);
    if(op==='writeFile'){files.set(args.path,args.data);return null;}
    if(op==='readFile'){if(!files.has(args.path))throw {code:-1,message:'missing',data:{code:'ENOENT'}};return files.get(args.path);}
    if(op==='stat')return {isFile:true,size:3,mtimeMs:20,mode:0o100600};
    if(op==='access'){if(!files.has(args.path))throw {code:-1,message:'missing',data:{code:'ENOENT'}};return null;}
    if(op==='copyFile'){files.set(args.destination,files.get(args.path));return null;}
    return null;
  }
  const niva={bridge:{call:(...args)=>Promise.resolve().then(()=>dispatch(...args)),callSync:dispatch}};
  const fs=runtime.createFsModule(niva);
  fs.writeFileSync('/bytes',Uint8Array.of(0,255,65),{mode:0o600,flag:'wx'});
  assert.deepEqual([...fs.readFileSync('/bytes')],[0,255,65]);
  assert.deepEqual([...await fs.promises.readFile('/bytes')],[0,255,65]);
  assert.equal(await new Promise((resolve,reject)=>fs.readFile('/bytes','hex',(error,data)=>error?reject(error):resolve(data))),'00ff41');
  assert.equal(fs.statSync('/bytes').mtimeMs,20);
  assert.equal(fs.existsSync('/absent'),false);
  await assert.rejects(fs.promises.readFile('/absent'),{code:'ENOENT'});
  assert.throws(()=>fs.readFile('/bytes'),TypeError);
  await fs.promises.copyFile('/bytes','/copy',fs.constants.COPYFILE_EXCL);
  assert.deepEqual(calls.find(([op])=>op==='copyFile')[1],{path:'/bytes',destination:'/copy',flags:1});
  assert.equal(calls[0][1].mode,0o600);assert.equal(calls[0][1].flag,'wx');
});

test('OS static information avoids bridge calls and dynamic values query each time',()=>{
  let queries=0;
  const os=runtime.createOsModule({bootstrap:{os:{platform:'darwin',arch:'arm64',homedir:'/home/u',tmpdir:'/tmp',EOL:'\n'}},bridge:{callSync(method,args){assert.equal(method,'os.freemem');assert.deepEqual(args,[]);return ++queries;}}});
  assert.equal(os.platform(),'darwin');assert.equal(os.arch(),'arm64');assert.equal(os.EOL,'\n');assert.equal(queries,0);
  assert.equal(os.freemem(),1);assert.equal(os.freemem(),2);
});

function processBridge(status=0){
  const calls=[],sent=[];
  const bridge={stream(method,args,handlers){calls.push([method,args]);if(method==='process.signal')return {promise:Promise.resolve(true)};return {id:7,cancel(){},promise:Promise.resolve().then(()=>{
    handlers.onEvent('spawn',{pid:42});handlers.onChunk(Uint8Array.from([0xe4,0xbd]),false);handlers.onChunk(Uint8Array.from([0xa0]),false);handlers.onChunk(new TextEncoder().encode('warn'),true);return {status};
  })};},streamSend(id,bytes,end){sent.push([id,[...bytes],end]);return true;}};
  return {calls,sent,bridge,bootstrap:{os:{platform:'linux'}}};
}
test('child output uses real streams, decodes split UTF-8, and signals the owned call',async()=>{
  const bridge=processBridge(),cp=runtime.createChildProcessModule(bridge),child=cp.spawn('cat',[]);let text='';
  child.stdout.setEncoding('utf8');child.stdout.on('data',chunk=>text+=chunk);
  const ended=new Promise(resolve=>child.stdout.once('end',resolve));
  child.on('spawn',()=>{assert.equal(child.pid,42);assert.equal(child.kill('SIGTERM'),true);});
  child.stdin.end('hello');await child.completion;await ended;
  assert.equal(text,'你');assert.deepEqual(bridge.calls[1],['process.signal',[7,'SIGTERM']]);assert.deepEqual(bridge.sent[0],[7,[104,101,108,108,111],false]);
});
test('exec callbacks receive buffered stderr and nonzero exit errors',async()=>{
  const cp=runtime.createChildProcessModule(processBridge(7));
  const result=await new Promise(resolve=>cp.exec('printf ignored',(error,stdout,stderr)=>resolve({error,stdout,stderr})));
  assert.equal(result.error.status,7);assert.equal(result.stdout,'你');assert.equal(result.stderr,'warn');
});
test('base runtime exposes Niva namespaces without injecting Node globals',()=>{
  assert.equal(globalThis.Niva.runtimeConfig.injectCommonJs,false);
  assert.equal(globalThis.Niva.runtimeConfig.injectEsm,false);
  assert.equal(typeof globalThis.Niva.fs.readFile,'function');
  assert.equal(globalThis.process,hostGlobals.process);
  assert.equal(globalThis.require,hostGlobals.require);
  assert.equal(globalThis.module,hostGlobals.module);
  assert.equal(globalThis.Buffer,hostGlobals.Buffer);
});

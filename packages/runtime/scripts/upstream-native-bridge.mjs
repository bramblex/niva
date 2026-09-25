// Test infrastructure, never shipped. Real WebView bridge behind a disposable relay.
import { spawn, spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import http from 'node:http';
const hostBuffer = Buffer;
const directory = path.dirname(fileURLToPath(import.meta.url));
const python = process.platform === 'win32' ? 'python' : 'python3';
export function startNativeBridge(binary) {
  const temporary = mkdtempSync(path.join(tmpdir(), 'niva-official-relay-'));
  const ready = path.join(temporary, 'ready.json');
  const relay = spawn(python, [path.join(directory, 'upstream-native-relay.py'), binary, ready], { stdio: ['ignore', 'ignore', 'ignore'] });
  relay.unref();
  function cleanup() { relay.kill(); rmSync(temporary, { recursive: true, force: true }); }
  process.once('exit', cleanup);
  process.once('SIGTERM', () => process.exit(143));
  process.once('SIGINT', () => process.exit(130));
  const deadline = Date.now() + 25000;
  while (!existsSync(ready) && Date.now() < deadline) spawnSync(python, ['-c', 'import time;time.sleep(.05)']);
  if (!existsSync(ready)) throw new Error('Real Native relay failed to start');
  const config = JSON.parse(readFileSync(ready));
  let sequence = 0;
  const streams = new Map();
  const cancelled = new Set();
  function decode(result) {
    if (result.error) throw Object.assign(new Error(result.error.message), result.error);
    return result.value;
  }
  function sync(data) {
    const script = 'import sys,json,urllib.request\nc=json.load(sys.stdin)\nr=urllib.request.Request(c["url"]+"/rpc",data=json.dumps(c["data"]).encode(),headers={"Authorization":"Bearer "+c["token"]})\nprint(urllib.request.urlopen(r,timeout=15).read().decode())';
    const result = spawnSync(python, ['-c', script], { input: JSON.stringify({ ...config, data: { id: ++sequence, ...data } }), encoding: 'utf8', timeout: 17000, maxBuffer: 8*1024*1024 });
    if (result.error || result.status !== 0) throw new Error('Native relay RPC failed: ' + (result.error?.message || result.stderr));
    return decode(JSON.parse(result.stdout));
  }
  function request(endpoint, data) {
    return new Promise((resolve, reject) => {
      const body=JSON.stringify(data);
      const req=http.request(config.url+endpoint,{method:'POST',agent:false,headers:{Authorization:'Bearer '+config.token,'Content-Length':hostBuffer.byteLength(body)}},res=>{
        const chunks=[];res.on('data',c=>chunks.push(c));res.on('end',()=>{
          try { if(res.statusCode!==200)throw new Error('Relay HTTP '+res.statusCode);resolve(JSON.parse(hostBuffer.concat(chunks))); }catch(e){reject(e);}
        });
      });req.on('error',reject);req.setTimeout(17000,()=>req.destroy(new Error('Relay timeout')));req.end(body);
    });
  }
  let polling=false;
  async function poll() {
    if(polling)return;polling=true;
    try {
      while(streams.size){
        for(const event of await request('/events',{})){
          if(event.fatal)throw new Error(event.fatal);
          if(cancelled.has(event.stream))continue;
          const s=streams.get(event.stream);if(!s)throw new Error('Unknown Native stream event '+event.stream);
          if(event.kind==='event')s.handlers.onEvent?.(...event.values);
          else if(event.kind==='chunk')s.handlers.onChunk?.(new Uint8Array(event.values[0]),event.values[1]);
          else if(event.kind==='blob')s.handlers.onBlob?.(new Blob([new Uint8Array(event.values[0])]),event.values[1]);
          else {streams.delete(event.stream);if(event.kind==='resolve')s.resolve(event.values[0]);else s.reject(event.values[0]);}
        }
      }
    }catch(error){for(const s of streams.values())s.reject(error);streams.clear();}
    finally{polling=false;}
  }
  const bridge={bootstrap:config.bootstrap,
    callSync(method,args=[]){return sync({op:'sync',method,args});},
    call(method,args=[]){return request('/rpc',{id:++sequence,op:'call',method,args}).then(decode);},
    stream(method,args=[],handlers={}){
      const key=++sequence;
      let resolve,reject;const promise=new Promise((a,b)=>{resolve=a;reject=b;});
      const id=sync({id:key,op:'stream',method,args,chunk:!!handlers.onChunk,blob:!!handlers.onBlob});
      streams.set(key,{handlers,resolve,reject});queueMicrotask(poll);
      return {id,promise,cancel(){const result=sync({op:'cancel',stream:id});cancelled.add(key);streams.delete(key);return result;}};
    },
    streamSend(id,bytes,end){return sync({op:'send',stream:id,bytes:Array.from(bytes),end});}
  };
  return {bridge, directory:config.directory, cleanup};
}

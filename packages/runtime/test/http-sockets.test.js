import "./setup-runtime.js";
import test from 'node:test';
import assert from 'node:assert/strict';
import net from 'node:net';
import nodeHttp from 'node:http';
import '../dist/source/runtime/bridge.js';
import '../dist/source/runtime/vendor.js';
import '../dist/source/runtime/events.js';
import '../dist/source/runtime/http.js';
const runtime=globalThis[Symbol.for('niva.node-compat.runtime')];
let connected=0;
runtime.createNetModule=()=>({connect(options){connected++;return net.connect(options);},createServer:(...args)=>net.createServer(...args)});
const http=runtime.createHttpModule('http');
const listen=server=>new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const close=server=>new Promise(resolve=>server.close(resolve));
const collect=response=>new Promise((resolve,reject)=>{let chunks=[];response.on('data',chunk=>chunks.push(chunk));response.on('end',()=>resolve(Buffer.concat(chunks).toString()));response.on('error',reject);});
test('JS HTTP client streams chunked requests to a localhost HTTP peer',async()=>{
  let received;
  const server=nodeHttp.createServer(async(req,res)=>{received=await collect(req);res.setHeader('X-Protocol','native-peer');res.write('hel');res.end('lo');});
  await listen(server);
  try{
    const result=await new Promise((resolve,reject)=>{const req=http.request(`http://127.0.0.1:${server.address().port}/test`,{method:'POST'},async response=>{try{resolve({body:await collect(response),header:response.headers['x-protocol']});}catch(error){reject(error);}});req.on('error',reject);req.write('a');req.end('bc');});
    assert.deepEqual(result,{body:'hello',header:'native-peer'});assert.equal(received,'abc');assert.ok(connected>0);
  }finally{await close(server);}
});
test('JS HTTP server handles Node HTTP request, binary body, and response trailers',async()=>{
  const server=http.createServer(async(req,res)=>{assert.equal(req.url,'/echo');assert.equal(req.method,'POST');const body=await collect(req);res.setHeader('X-Test','server');res.addTrailers({'X-Trailer':'complete'});res.end(body);});
  await listen(server);
  try{
    const result=await new Promise((resolve,reject)=>{const req=nodeHttp.request({host:'127.0.0.1',port:server.address().port,path:'/echo',method:'POST'},async response=>{try{const body=await collect(response);resolve({body,trailers:response.trailers});}catch(error){reject(error);}});req.on('error',reject);req.end('hello');});
    assert.deepEqual(result,{body:'hello',trailers:{'x-trailer':'complete'}});
  }finally{await close(server);}
});
test('JS HTTP server rejects duplicate length and TE/CL before dispatch',async()=>{
  let requests=0,errors=0;const server=http.createServer(()=>requests++);
  server.on('clientError',(_error,socket)=>{errors++;socket.destroy();});await listen(server);
  try{
    for(const header of ['Content-Length: 1\r\nContent-Length: 1','Transfer-Encoding: chunked\r\nContent-Length: 1']){
      await new Promise((resolve,reject)=>{const socket=net.connect(server.address().port,'127.0.0.1',()=>socket.end('POST / HTTP/1.1\r\nHost: local\r\n'+header+'\r\n\r\n0\r\n\r\n'));socket.on('close',resolve);socket.on('error',reject);});
    }
    assert.equal(requests,0);assert.equal(errors,2);
  }finally{await close(server);}
});

test('invalid outbound HTTP input fails before allocating a native connection',()=>{
  const before=connected;
  assert.throws(()=>http.request('http://127.0.0.1/',{method:'GET\r\nInjected'}));
  assert.throws(()=>http.request('http://127.0.0.1/',{headers:{'X-Test':'bad\r\nHeader: value'}}));
  assert.throws(()=>http.request('http://@127.0.0.1/'));
  assert.throws(()=>http.request('http://127.0.0.1/',{protocol:'https:'}));
  assert.equal(connected,before);
});

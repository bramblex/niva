#!/usr/bin/env python3
"""Test-only bridge relay. Every operation executes in an isolated real Niva page.
No host filesystem/socket implementation is substituted for the product API.
The random HTTP capability belongs only to this disposable test relay.
"""
import http.server,json,os,queue,secrets,signal,subprocess,sys,tempfile,threading,time,uuid
from pathlib import Path

PAGE = '''<!doctype html><script>
(async()=>{
 await NivaNodeCompatReady;
 const streams=new Map();
 let outbound=Promise.resolve();
 const send=data=>{outbound=outbound.then(()=>typeof data==='function'?data():data).then(value=>Niva.api.host.send('relay',value));return outbound;};
 Niva.addEventListener('host:message',async(_,message)=>{
  if(message.name!=='relay')return;
  const r=message.data; let value;
  try {
   if(r.op==='sync')value=Niva.callSync(r.method,r.args);
   else if(r.op==='call')value=await Niva.call(r.method,r.args);
   else if(r.op==='stream'){
    const event=(kind,values)=>send({stream:r.id,kind,values});
    let blobs=Promise.resolve();
    const s=Niva.stream(r.method,r.args,{
     onEvent:(...v)=>event('event',v),
     onChunk:r.chunk?((b,e)=>event('chunk',[Array.from(b),e])):undefined,
     onBlob:r.blob?((b,e)=>{blobs=send(async()=>({stream:r.id,kind:'blob',values:[Array.from(new Uint8Array(await b.arrayBuffer())),e]}));}):undefined
    });streams.set(s.id,s);value=s.id;
    s.promise.then(v=>blobs.then(()=>event('resolve',[v])),e=>blobs.then(()=>event('reject',[e]))).finally(()=>streams.delete(s.id));
   }else if(r.op==='send')value=Niva.streamSend(r.stream,new Uint8Array(r.bytes),r.end);
   else if(r.op==='cancel'){streams.get(r.stream)?.cancel();value=true;}
   else throw new Error('Unknown relay operation');
   await send({reply:r.id,value:value===undefined?null:value});
  }catch(e){await send({reply:r.id,error:{message:e.message||String(e),code:e.code,data:e.data}});}
 });
 await Niva.api.host.send('relay-ready',Niva.bootstrap);
})().catch(e=>Niva.api.host.send('relay-fatal',{message:String(e)}));
</script>'''

def main():
 binary,ready_file=sys.argv[1:3]
 parent=os.getppid()
 def watch_parent():
  while True:
   time.sleep(1)
   if os.getppid()!=parent or parent==1:
    os.kill(os.getpid(),signal.SIGTERM);return
 threading.Thread(target=watch_parent,daemon=True).start()
 with tempfile.TemporaryDirectory(prefix='niva-upstream-native-') as directory:
  root=Path(directory).resolve();directory=str(root); (root/'index.html').write_text(PAGE)
  (root/'niva.json').write_text(json.dumps({'name':'OfficialContracts','uuid':str(uuid.uuid4()),'window':{'entry':'index.html','visible':False}}))
  env=os.environ.copy()
  for key in ['HOME','TMPDIR']:
   p=root/key;p.mkdir();env[key]=str(p)
  stderr=(root/'stderr.log').open('w+')
  native=subprocess.Popen([binary,'--stdio','--debug-resource='+directory],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=stderr,env=env,text=True)
  lock=threading.Lock(); write_lock=threading.Lock(); responses={}; events=queue.Queue(); boot=queue.Queue(); token=secrets.token_hex(32)
  def read():
   for line in native.stdout:
    try:
     frame=json.loads(line)
     if frame.get('name')=='relay-ready':boot.put(frame['data'])
     if frame.get('name')=='relay':
      d=frame['data']
      if 'reply' in d:
       with lock:q=responses.get(d['reply'])
       if q:q.put(d)
      else:events.put(d)
    except Exception as e:events.put({'fatal':str(e)})
  threading.Thread(target=read,daemon=True).start()
  try:
   bootstrap=boot.get(timeout=20)
   class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self,*args):pass
    def do_POST(self):
     if self.headers.get('Authorization')!='Bearer '+token:self.send_error(403);return
     try:
      data=json.loads(self.rfile.read(int(self.headers['Content-Length'])))
      if self.path=='/events':
       value=[]
       try:value.append(events.get(timeout=.1))
       except queue.Empty:pass
       while not events.empty():value.append(events.get_nowait())
      elif self.path=='/rpc':
       q=queue.Queue()
       with lock:
        responses[data['id']]=q
       with write_lock:
        native.stdin.write(json.dumps({'t':'msg','name':'relay','data':data})+'\n');native.stdin.flush()
       try:value=q.get(timeout=12)
       finally:
        with lock:responses.pop(data['id'],None)
      else:raise ValueError('Unknown endpoint')
      payload=json.dumps(value).encode();self.send_response(200);self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)
     except Exception as e:self.send_error(500,str(e))
   server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
   Path(ready_file).write_text(json.dumps({'url':'http://127.0.0.1:'+str(server.server_port),'token':token,'bootstrap':bootstrap,'directory':directory}))
   server.serve_forever()
  finally:
   native.stdin.close()
   try:native.wait(timeout=3)
   except subprocess.TimeoutExpired:native.terminate();native.wait(timeout=5)
   stderr.close()

if __name__=='__main__':
 signal.signal(signal.SIGTERM,lambda *_:sys.exit(0))
 main()

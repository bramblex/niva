#!/usr/bin/env python3
"""Test-only bridge relay. Every operation executes in an isolated real Niva page.
No host filesystem/socket implementation is substituted for the product API.
The random HTTP capability belongs only to this disposable test relay.
"""
import http.server,json,os,queue,secrets,signal,subprocess,sys,tempfile,threading,time,uuid
from pathlib import Path

PAGE = '''<!doctype html><meta charset="utf-8"><script>
(() => {
  const protocol = "niva-fixture";
  const streams = new Map();
  let input = "";
  let outbound = Promise.resolve();
  const errorValue = error => ({message: error && error.message || String(error), code: error && error.code, data: error && error.data});
  function send(name, data) {
    outbound = outbound.then(async () => {
      const payload = typeof data === "function" ? await data() : data;
      const frame = {protocol, version: 1, event: name === "relay-ready" ? "ready" : "message", name, data: payload};
      await new Promise((resolve, reject) => process.stdout.write(JSON.stringify(frame) + "\\n", error => error ? reject(error) : resolve()));
    });
    return outbound;
  }
  function dispatch(line) {
    let frame;
    try { frame = JSON.parse(line); } catch (error) { void send("relay-fatal", {message: String(error)}); return; }
    if (!frame || frame.protocol !== protocol || frame.version !== 1 || frame.event !== "command" || frame.name !== "relay") return;
    void handle(frame.data);
  }
  async function handle(r) {
    let value;
    try {
      if (r.op === "sync") value = Niva.bridge.callSync(r.method, r.args);
      else if (r.op === "call") value = await Niva.bridge.call(r.method, r.args);
      else if (r.op === "stream") {
        const event = (kind, values) => send("relay", {stream: r.id, kind, values});
        let blobs = Promise.resolve();
        const stream = Niva.bridge.stream(r.method, r.args, {
          onEvent: (...values) => event("event", values),
          onChunk: r.chunk ? ((bytes, end) => event("chunk", [Array.from(bytes), end])) : undefined,
          onBlob: r.blob ? ((blob, end) => {
            blobs = send("relay", async () => ({stream: r.id, kind: "blob", values: [Array.from(new Uint8Array(await blob.arrayBuffer())), end]}));
          }) : undefined
        });
        streams.set(stream.id, stream);
        value = stream.id;
        stream.promise.then(
          result => blobs.then(() => event("resolve", [result])),
          error => blobs.then(() => event("reject", [errorValue(error)]))
        ).finally(() => streams.delete(stream.id));
      } else if (r.op === "send") value = Niva.bridge.streamSend(r.stream, new Uint8Array(r.bytes), r.end);
      else if (r.op === "cancel") { streams.get(r.stream)?.cancel(); value = true; }
      else throw new Error("Unknown relay operation");
      await send("relay", {reply: r.id, value: value === undefined ? null : value});
    } catch (error) {
      await send("relay", {reply: r.id, error: errorValue(error)});
    }
  }
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", chunk => {
    input += chunk;
    let newline;
    while ((newline = input.indexOf("\\n")) !== -1) {
      const line = input.slice(0, newline).replace(/\\r$/, "");
      input = input.slice(newline + 1);
      if (line) dispatch(line);
    }
  });
  process.stdin.on("end", () => { if (input.trim()) dispatch(input.trim()); });
  void send("relay-ready", Niva.bootstrap);
})()
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
  config=root/'niva.json'
  config.write_text(json.dumps({'name':'OfficialContracts','uuid':str(uuid.uuid4()),'injectCommonJs':True,'window':{'entry':'index.html','visible':False}}))
  env=os.environ.copy()
  for key in ['HOME','TMPDIR']:
   p=root/key;p.mkdir();env[key]=str(p)
  stderr=(root/'stderr.log').open('w+')
  native=subprocess.Popen([binary,'--config='+str(config),'--resource='+directory],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=stderr,env=env)
  lock=threading.Lock(); write_lock=threading.Lock(); responses={}; events=queue.Queue(); boot=queue.Queue(); token=secrets.token_hex(32)
  def read():
   for raw in native.stdout:
    try:
     line=raw.decode('utf-8').rstrip('\r\n')
     frame=json.loads(line)
     if frame.get('protocol')!='niva-fixture' or frame.get('version')!=1:raise RuntimeError('invalid fixture protocol frame')
     if frame.get('event')=='ready' and frame.get('name')=='relay-ready':boot.put(frame['data'])
     elif frame.get('event')=='message' and frame.get('name')=='relay':
      d=frame['data']
      if 'reply' in d:
       with lock:q=responses.get(d['reply'])
       if q:q.put(d)
      else:events.put(d)
     elif frame.get('event')=='message' and frame.get('name')=='relay-fatal':events.put({'fatal':frame.get('data',{}).get('message','relay failed')})
    except Exception as e:events.put({'fatal':str(e)})
   if boot.empty():events.put({'fatal':'native process closed stdout before the page relay became ready'})
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
        if native.poll() is not None:raise RuntimeError('Niva exited before completing the fixture call')
        native.stdin.write((json.dumps({'protocol':'niva-fixture','version':1,'event':'command','name':'relay','data':data})+'\n').encode('utf-8'));native.stdin.flush()
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

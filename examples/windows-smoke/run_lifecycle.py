"""Real Windows regression: API deadlines, long dialog, and closing active windows."""
import argparse, ctypes, hashlib, json, subprocess, sys, threading, time, uuid
from ctypes import wintypes
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def dismiss_dialog(pid, title):
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
    user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
    user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
    user32.EnumChildWindows.argtypes = [wintypes.HWND, callback_type, wintypes.LPARAM]
    user32.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.GetDlgCtrlID.argtypes = [wintypes.HWND]
    found = []
    def visit(hwnd, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == pid:
            text = ctypes.create_unicode_buffer(256)
            user32.GetWindowTextW(hwnd, text, 256)
            if text.value == title:
                buttons = []
                def child(control, _):
                    kind = ctypes.create_unicode_buffer(64)
                    user32.GetClassNameW(control, kind, 64)
                    if kind.value == "Button": buttons.append(control)
                    return True
                user32.EnumChildWindows(hwnd, callback_type(child), 0)
                # Rfd/MessageBox can assign IDCANCEL to its sole confirmation
                # button. Resolve the actual control, never guess an ID.
                if len(buttons) == 1:
                    button = buttons[0]
                    control_id = user32.GetDlgCtrlID(button)
                    user32.PostMessageW(button, 0xF5, 0, 0)
                    user32.PostMessageW(hwnd, 0x466, control_id, 0)
                    user32.PostMessageW(hwnd, 0x111, control_id, button)
                    found.append(hwnd)
        return True
    user32.EnumWindows(callback_type(visit), 0)
    return bool(found)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--dialog-seconds", type=float, default=40)
    args = parser.parse_args()
    if sys.platform != "win32": raise SystemExit("Windows desktop required")
    binary, output = Path(args.binary).resolve(), Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    result, done, dialog = {}, threading.Event(), threading.Event()
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_): pass
        def do_POST(self):
            data = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
            if self.path == "/dialog": dialog.set()
            else: result.update(data); done.set()
            self.send_response(200); self.send_header("Access-Control-Allow-Origin", "*"); self.end_headers()
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    endpoint = f"http://127.0.0.1:{server.server_port}"
    title = "Niva lifecycle " + str(uuid.uuid4())
    fixture = "const PY=" + json.dumps(sys.executable) + ";const ENDPOINT=" + json.dumps(endpoint) + ";const TITLE=" + json.dumps(title) + ";const WAIT_MS=" + str(args.dialog_seconds*1000) + ";"
    script = fixture + r"""
(async()=>{
 const checks=[];
 const post=(path,data)=>fetch(ENDPOINT+path,{method:'POST',headers:{'Content-Type':'text/plain'},body:JSON.stringify(data)});
 try {
  const start=performance.now(); let timedOut=false;
  try {await Niva.bridge.call('process.execFileText',[PY,['-c','import time; time.sleep(5)']]);}
  catch(e){timedOut=e.code==='ETIMEDOUT'||String(e.message).includes('ETIMEDOUT');}
  if(!timedOut||performance.now()-start>3000)throw Error('configured 500 ms IPC deadline was not enforced');
  checks.push('configured IPC deadline cancels a running process');
  for(let i=0;i<4;i++){
   const id=await Niva.window.open({entry:'child.html',visible:true});
   await new Promise(r=>setTimeout(r,200));
   await Niva.window.close(id);
   await Niva.window.title();
  }
  checks.push('close four windows with active Native calls');
  await post('/dialog',{});
  const opened=performance.now();
  await Niva.dialog.showMessage(TITLE,'Automated regression: this dialog closes after 40 seconds.');
  if(performance.now()-opened<WAIT_MS-1000)throw Error('dialog did not outlive the old 35 second limit');
  const dialogElapsedMs=Math.round(performance.now()-opened);
  checks.push('dialog remains live until explicit confirmation');
  await Niva.window.title();
  await post('/result',{ok:true,checks,dialogElapsedMs});
 }catch(e){await post('/result',{ok:false,checks,error:String(e),stack:e.stack});}
})();
"""
    (output / "index.html").write_text('<!doctype html><script src="probe.js"></script>')
    (output / "probe.js").write_text(script)
    (output / "child.html").write_text('<!doctype html><script src="child.js"></script>')
    (output / "child.js").write_text(fixture + "Niva.bridge.call('process.execFileText',[PY,['-c','import time;time.sleep(5)']]).catch(()=>{});")
    (output / "niva.json").write_text(json.dumps({"name":"Lifecycle", "uuid":str(uuid.uuid4()), "api":{"timeoutMs":500}, "window":{"entry":"index.html", "visible":True}}))
    process = None
    try:
        with (output / "stdio.log").open('wb') as log:
            process = subprocess.Popen([str(binary), '--resource='+str(output)], stdout=log, stderr=subprocess.STDOUT)
            deadline=time.monotonic()+80; shown=None; dismissed=False
            while not done.wait(.1) and time.monotonic()<deadline:
                if process.poll() is not None: break
                if dialog.is_set() and shown is None: shown=time.monotonic()
                if shown and not dismissed and time.monotonic()-shown>=args.dialog_seconds:
                    dismissed=dismiss_dialog(process.pid,title)
            if not done.is_set(): result.update(ok=False,error='No lifecycle result within deadline')
            result['dialogDismissedByHarness']=dismissed
            result['dialogWaitSeconds']=args.dialog_seconds
    finally:
        if process and process.poll() is None:
            dismiss_dialog(process.pid,title)
            process.terminate()
            try: process.wait(timeout=5)
            except subprocess.TimeoutExpired: process.kill(); process.wait()
        server.shutdown(); server.server_close()
    result['binarySha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
    (output/'result.json').write_text(json.dumps(result,indent=2))
    print(json.dumps(result),flush=True)
    raise SystemExit(0 if result.get('ok') else 1)

if __name__ == '__main__': main()

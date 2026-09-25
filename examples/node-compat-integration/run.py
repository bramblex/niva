#!/usr/bin/env python3
"""Exercise the unified runtime through a real Niva WebView and native bridge."""
import argparse,json,os,selectors,subprocess,tempfile,time,uuid,shutil,zlib
from pathlib import Path

def run(binary,packaged=False,trace=False):
    with tempfile.TemporaryDirectory(prefix='niva-node-integration-') as directory:
        root=Path(directory)
        def openssl(*args):
            subprocess.run(['openssl',*args],cwd=root,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        openssl('req','-x509','-newkey','rsa:2048','-nodes','-keyout','ca.key','-out','ca.crt','-days','2','-subj','/CN=NivaIntegrationCA','-addext','basicConstraints=critical,CA:TRUE','-addext','keyUsage=critical,keyCertSign,cRLSign')
        openssl('req','-newkey','rsa:2048','-nodes','-keyout','server.key','-out','server.csr','-subj','/CN=localhost')
        (root/'server.ext').write_text('basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost,IP:127.0.0.1\n')
        openssl('x509','-req','-in','server.csr','-CA','ca.crt','-CAkey','ca.key','-CAcreateserial','-out','server.crt','-days','2','-extfile','server.ext')

        openssl('rsa','-in','server.key','-traditional','-out','server-pkcs1.key')

        inventory=json.loads((Path(__file__).resolve().parents[2]/'docs/node-api-inventory.json').read_text())
        specs=[{'api':item['api'],'module':item['module']} for item in inventory['apis']]
        html=Path(__file__).with_name('index.html').read_text().replace('__SMOKE_DIR__',json.dumps(directory)).replace('__API_SPECS__',json.dumps(specs)).replace('__TRACE__',json.dumps(trace))
        (root/'index.html').write_text(html)
        shutil.copy2(Path(__file__).with_name('fixture-protocol.js'), root/'fixture-protocol.js')
        (root/'niva.json').write_text(json.dumps({'name':'NivaNodeIntegration','uuid':str(uuid.uuid4()),'injectCommonJs':True,'injectEsm':True,'window':{'entry':'index.html','title':'Niva Node integration','size':{'width':600,'height':400},'visible':False}}))
        command=[str(binary),'--config='+str(root/'niva.json'),'--resource='+directory]
        if packaged:
            executable=root/'Smoke.app/Contents/MacOS/niva';executable.parent.mkdir(parents=True)
            resources=executable.parent.parent/'Resources';resources.mkdir()
            shutil.copy2(binary,executable)
            data=b'';indexes={}
            for name in ['niva.json','index.html','fixture-protocol.js']:
                content=(root/name).read_bytes();indexes[name]=[len(data),len(content)];data+=content
            compressor=zlib.compressobj(wbits=-15)
            (resources/'RESOURCE_INDEXES').write_text(json.dumps(indexes))
            (resources/'RESOURCE_DATA').write_bytes(compressor.compress(data)+compressor.flush())
            command=[str(executable)]
        child_env=os.environ.copy()
        for name,relative in [('HOME','home'),('TMPDIR','tmp')]:
            (root/relative).mkdir()
            child_env[name]=str(root/relative)
        with (root/'stderr.log').open('wb') as errors:
            process=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=errors,env=child_env)
            selector=selectors.DefaultSelector();selector.register(process.stdout,selectors.EVENT_READ)
            buffer=b'';deadline=time.monotonic()+60;fixture_ready=False;result=None;progress=[]
            try:
                while result is None and time.monotonic()<deadline:
                    for _,_ in selector.select(1):
                        chunk=os.read(process.stdout.fileno(),65536)
                        if not chunk:raise RuntimeError('Niva exited early: '+str(process.poll()))
                        buffer+=chunk
                        while b'\n' in buffer:
                            line,buffer=buffer.split(b'\n',1);message=json.loads(line)
                            if message.get('event')=='ready' and message.get('name')=='node-compat-integration':fixture_ready=True
                            if message.get('name')=='implementation-progress' and trace:
                                label=message['data']['label'];progress.append(label);print('[node-compat] '+label,flush=True)
                            if message.get('event')=='message' and message.get('name')=='implementation-result':result=message['data']
                if result is None:raise TimeoutError('No WebView result; progress='+repr(progress)+'; '+(root/'stderr.log').read_text())
                result['fixtureReady']=fixture_ready;result['packaged']=packaged
                print(json.dumps(result,ensure_ascii=False,indent=2))
                if not result.get('ok') or not fixture_ready:raise RuntimeError('Runtime integration checks failed')
            finally:
                selector.close();process.stdin.close()
                try:process.wait(timeout=5)
                except subprocess.TimeoutExpired:process.terminate();process.wait(timeout=5)

if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('binary',nargs='?',default=str(Path(__file__).resolve().parents[2]/'target/debug/niva'))
    parser.add_argument('--packaged',action='store_true');parser.add_argument('--trace',action='store_true');args=parser.parse_args()
    run(Path(args.binary).resolve(),args.packaged,args.trace)

#!/usr/bin/env python3
"""Full wasm-pack release path experiment; dispatched only on research ref."""
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import time

ROOT=Path.cwd()
OUT=ROOT/'output/wasm-wrapper-integration'
SELF=Path(__file__).resolve()

def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p,d): Path(p).write_text(json.dumps(d,indent=2)+'\n')
def capture(cmd): return subprocess.check_output(cmd,text=True).strip()
def timed(cmd, log, cwd=ROOT, env=None):
    start=time.perf_counter()
    with log.open('w') as f:
        code=subprocess.call(cmd,cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT)
    result={'command':cmd,'seconds':time.perf_counter()-start,'exit_code':code}
    print(json.dumps(result),flush=True)
    if code: print(log.read_text()[-6000:],flush=True)
    return result

def record_rustc(args):
    start=time.perf_counter()
    code=subprocess.call(args)
    dest=Path(os.environ['RHWP_COMPILER_RECORDS'])
    if '--crate-name' in args:
        name=args[args.index('--crate-name')+1]
        save(dest/f'{name}-{os.getpid()}.json',{'command':args,'seconds':time.perf_counter()-start,'exit_code':code,'name':name})
    return code

def main():
    mode=sys.argv[1]
    assert mode in ['dual','cdylib']
    OUT.mkdir(parents=True,exist_ok=True)
    assert not (ROOT/'target/pr-review').exists(), 'Requires a fresh hosted runner target, never deletes caches'
    assert not (ROOT/'pkg').exists(), 'Requires fresh package output'
    save(OUT/'environment.json',{'head':capture(['git','rev-parse','HEAD']),'mode':mode,'rustc':capture(['rustc','-vV']),'wasm_pack':capture(['wasm-pack','--version']),'cpu_count':os.cpu_count(),'cpu':Path('/proc/cpuinfo').read_text(),'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip(),'image':os.environ.get('ImageVersion'),'cache_policy':'No Cargo or package restore/save; target initially absent'})
    wrapper=OUT/'rustc-record'
    wrapper.write_text('#!/bin/sh\nexec '+shlex.quote(sys.executable)+' '+shlex.quote(str(SELF))+' --rustc "$@"\n'); wrapper.chmod(0o755)
    src=ROOT/'src/lib.rs'; original=src.read_bytes()
    before='env!("CARGO_PKG_VERSION").to_string()'
    after='concat!(env!("CARGO_PKG_VERSION"), "-phase-probe-1").to_string()'
    assert original.decode().count(before)==1
    checks={}
    try:
        for stage in ['cold','changed']:
            if stage=='changed': src.write_bytes(original.decode().replace(before,after).encode())
            dest=OUT/stage; dest.mkdir()
            records=dest/'compiler'; records.mkdir()
            env=dict(os.environ,RHWP_WASM_CDYLIB_ONLY=str(int(mode=='cdylib')),RUSTC_WRAPPER=str(wrapper),RHWP_COMPILER_RECORDS=str(records),CARGO_TARGET_DIR='target/pr-review',CARGO_INCREMENTAL='0')
            result=timed(['python3','scripts/measure_render_diff_wasm.py','--profile','release','--output',str(dest)],dest/'command.log',env=env)
            save(dest/'command.json',result)
            assert result['exit_code']==0
            manifest=json.loads((dest/'build.json').read_text())
            manifest['mode']=mode; manifest['stage']=stage
            manifest['source_edit']={'original_sha256':hashlib.sha256(original).hexdigest(),'compiled_sha256':sha(src),'before':before,'after':after if stage=='changed' else before}
            manifest['artifacts']={p.name:{'sha256':sha(p),'bytes':p.stat().st_size} for p in (ROOT/'pkg').iterdir() if p.is_file()}
            manifest['studio_sync']={name:sha(ROOT/'pkg'/name)==sha(ROOT/'rhwp-studio/public'/name) for name in ['rhwp.js','rhwp_bg.wasm']}
            assert all(manifest['studio_sync'].values())
            invocations=[json.loads(p.read_text()) for p in records.glob('*.json')]
            root=[r for r in invocations if r['name']=='rhwp' and 'wasm32-unknown-unknown' in r['command']]
            assert len(root)==1
            manifest['compiler_invocations']=len(invocations)
            manifest['root_rustc']=root[0]
            if stage=='changed': assert len(invocations)==1, 'Warm edit must rebuild root only'
            save(dest/'build.json',manifest)
            shutil.copytree(ROOT/'pkg',dest/'pkg')
            node="""import fs from 'node:fs'; import init,* as api from './pkg/rhwp.js'; const bytes=fs.readFileSync('./pkg/rhwp_bg.wasm'); const m=new WebAssembly.Module(bytes); await init({module_or_path:bytes}); console.log(JSON.stringify({version:api.version(),jsExports:Object.keys(api).sort(),imports:WebAssembly.Module.imports(m).sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b))),exports:WebAssembly.Module.exports(m).sort((a,b)=>a.name.localeCompare(b.name))},null,2));"""
            api=json.loads(capture(['node','--input-type=module','-e',node]))
            assert api['version'].endswith('-phase-probe-1') == (stage=='changed')
            save(dest/'api.json',api)
            checks[stage]=True
    finally:
        src.write_bytes(original)
        save(OUT/'source-restoration.json',{'restored':src.read_bytes()==original,'diff':capture(['git','diff','--','src/lib.rs','Cargo.toml','Cargo.lock'])})
    native=timed(['cargo','build','--locked','--features','native-skia','--bin','rhwp','--target-dir','target/pr-review'],OUT/'native.log')
    checks['native']=native
    assert native['exit_code']==0
    env=dict(os.environ,RHWP_WASM_BUILD_MANIFEST=str(OUT/'changed/build.json'),RHWP_RENDER_DIFF_MAX_PAGES='1',RHWP_RENDER_DIFF_WRITE_IMAGES='1',RHWP_RENDER_DIFF_PDF='1',RHWP_RENDER_DIFF_PDF_WRITE_IMAGES='1',RHWP_RENDER_DIFF_DIRECT_PDF='1',RHWP_RENDER_DIFF_DIRECT_PDF_GATE='1',RHWP_RENDER_DIFF_DIRECT_PDF_MAX_RATIO='0.02',RHWP_RENDER_DIFF_DIRECT_PDF_RASTER_DPI='144',RHWP_RENDER_DIFF_RHWP_BIN=str(ROOT/'target/pr-review/debug/rhwp'))
    for name,cmd,cwd in [
        ('renderer-contract',['npm','run','e2e:renderer-contract'],ROOT/'rhwp-studio'),
        ('font-coverage',['npm','run','e2e:canvaskit-font-coverage'],ROOT/'rhwp-studio'),
        ('canvas-pdf',['npm','run','e2e:render-diff:ci'],ROOT/'rhwp-studio'),
        ('readiness',['python3','scripts/renderer_baseline.py','--profiles','screen','--browser-mode','headless','--readiness-only','--output',str(OUT/'readiness')],ROOT),
    ]:
        checks[name]=timed(cmd,OUT/(name+'.log'),cwd,env)
        save(OUT/'checks.json',checks)
    shots=ROOT/'rhwp-studio/e2e/screenshots/render-diff'
    if shots.exists(): shutil.copytree(shots,OUT/'screenshots')
    save(OUT/'checks.json',checks)
    assert all(c is True or c['exit_code']==0 for c in checks.values()), 'One or more release validation gates failed; see checks.json'

if __name__=='__main__':
    if sys.argv[1]=='--rustc': sys.exit(record_rustc(sys.argv[2:]))
    main()

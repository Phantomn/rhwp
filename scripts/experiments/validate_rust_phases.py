#!/usr/bin/env python3
"""Validate release raw artifacts from the fixed #7473 phase experiment."""
import hashlib,json,os,pathlib,shutil,subprocess,time
ROOT=pathlib.Path.cwd()
OUT=ROOT/'output/rust-phase-validation'
INPUT=ROOT/'output/phase-input'
TOOLS=ROOT/'output/phase-tools'
RAW_HEAD='99473fbc7d80f237e2ec3e9f952752431232bb9d'

def sha(p): return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def save(p,x): pathlib.Path(p).write_text(json.dumps(x,indent=2)+'\n')
def output(cmd): return subprocess.check_output(cmd,text=True).strip()
def run_timed(cmd,log,cwd=ROOT,env=None,check=True):
 start=time.perf_counter()
 with pathlib.Path(log).open('w') as f: r=subprocess.run(cmd,cwd=cwd,env=env,stdout=f,stderr=subprocess.STDOUT)
 duration=time.perf_counter()-start
 if r.returncode and check: raise RuntimeError(f'{cmd} failed; see {log}')
 return {'command':cmd,'seconds':duration,'exit_code':r.returncode}

def main():
 OUT.mkdir(parents=True,exist_ok=True)
 original=(ROOT/'src/lib.rs').read_bytes()
 environment=json.loads((INPUT/'environment.json').read_text())
 assert environment['head']==RAW_HEAD
 bindgen=TOOLS/'wasm-bindgen-0.2.127-x86_64-unknown-linux-musl/wasm-bindgen'
 opt=TOOLS/'binaryen-version_117/bin/wasm-opt'
 expected={bindgen:'f36dbe1938e78a79ce34d0ab3700907555161e293954f6f35bf6785a2f0cf916',opt:'621b5a984f16d0323ea7aa8b389ac9ccda0950ddee5de0e99292970129f571ed'}
 tools={}
 for binary,digest in expected.items():
  assert sha(binary)==digest, f'unexpected {binary}'
  tools[binary.name]={'sha256':digest,'version':output([str(binary),'--version'])}
 save(OUT/'tools.json',tools)
 all_results={}
 for mode,label in [('dual','4-dual'),('cdylib-baseline-lto','3-cdylib-baseline-lto')]:
  sample=INPUT/label
  build=json.loads((sample/'build.json').read_text())
  source=json.loads((sample/'source.json').read_text())
  assert build['diagnostic'] is False and build['crate_type']==mode and build['marker']=='1' and build['exit_code']==0
  raw=sample/'rhwp.wasm'
  assert sha(raw)==build['wasm_sha256']
  # Consumer source identity is checked against the exact producer commit.
  raw_source=subprocess.check_output(['git','show','HEAD:src/lib.rs'])
  assert hashlib.sha256(raw_source).hexdigest()==source['original_sha256']
  changed=raw_source.decode().replace(source['before'],source['after']).encode()
  assert hashlib.sha256(changed).hexdigest()==source['changed_sha256']
  dest=OUT/mode; dest.mkdir()
  pkg=ROOT/'pkg'; pkg.mkdir(exist_ok=True)
  # These are the exact postprocessing operations observed in wasm-pack 0.15.0 release.
  # Cargo was already measured; this step does not pretend to run it again.
  binding=run_timed([str(bindgen),str(raw),'--out-dir',str(pkg),'--typescript','--target','web'],dest/'bindgen.log')
  optimization=run_timed([str(opt),str(pkg/'rhwp_bg.wasm'),'-o',str(pkg/'rhwp_bg.wasm-opt.wasm'),'-O'],dest/'wasm-opt.log')
  (pkg/'rhwp_bg.wasm-opt.wasm').replace(pkg/'rhwp_bg.wasm')
  for name in ['rhwp.js','rhwp_bg.wasm']:
   shutil.copy2(pkg/name,ROOT/'rhwp-studio/public'/name)
  files={p.name:{'sha256':sha(p),'bytes':p.stat().st_size} for p in pkg.iterdir() if p.is_file()}
  manifest={'source_sha':RAW_HEAD,'source_dirty':True,'source_edit':source,'profile':'release','success':True,
   'artifact_producer_run':36514652526,'validation_head':output(['git','rev-parse','HEAD']),
   'compiler_sample':label,'cargo_build':build,'binding':binding,'optimization':optimization,'tools':tools,'artifacts':files}
  save(dest/'build.json',manifest)
  shutil.copytree(pkg,dest/'pkg')
  # Public JS/TS surface, Wasm exports/imports, and actual edited function result.
  node="""
import fs from 'node:fs';
import init, * as api from './pkg/rhwp.js';
const bytes=fs.readFileSync('./pkg/rhwp_bg.wasm');
const module=new WebAssembly.Module(bytes);
await init({module_or_path:bytes});
if (!api.version().endsWith('-phase-probe-1')) throw new Error('edited version not observed');
console.log(JSON.stringify({version:api.version(),jsExports:Object.keys(api).sort(),imports:WebAssembly.Module.imports(module).sort((a,b)=>JSON.stringify(a).localeCompare(JSON.stringify(b))),exports:WebAssembly.Module.exports(module).sort((a,b)=>a.name.localeCompare(b.name))},null,2));
"""
  (dest/'api.json').write_text(output(['node','--input-type=module','-e',node])+'\n')
  env=dict(os.environ,RHWP_WASM_BUILD_MANIFEST=str(dest/'build.json'),RHWP_RENDER_DIFF_ALL='1',RHWP_RENDER_DIFF_MAX_PAGES='1',RHWP_RENDER_DIFF_WRITE_IMAGES='1')
  browser=run_timed(['npm','run','e2e:render-diff:ci'],dest/'browser.log',ROOT/'rhwp-studio',env,check=False)
  shutil.copytree(ROOT/'rhwp-studio/e2e/screenshots/render-diff',dest/'screenshots')
  all_results[mode]={'files':files,'binding':binding,'optimization':optimization,'browser':browser}
 a=OUT/'dual'; b=OUT/'cdylib-baseline-lto'
 same_png=[]; changed_png=[]
 left={p.name:sha(p) for p in (a/'screenshots').glob('*.png')}
 right={p.name:sha(p) for p in (b/'screenshots').glob('*.png')}
 assert left and left.keys()==right.keys()
 for name in left:
  (same_png if left[name]==right[name] else changed_png).append(name)
 checks={
  'same_png':same_png,'changed_png':changed_png,
  'browser_passed':all(r['browser']['exit_code']==0 for r in all_results.values()),
  'api_identical':(a/'api.json').read_bytes()==(b/'api.json').read_bytes(),
  'types_identical':(a/'pkg/rhwp.d.ts').read_bytes()==(b/'pkg/rhwp.d.ts').read_bytes(),
  'canvas_results_identical':(a/'screenshots/results.json').read_bytes()==(b/'screenshots/results.json').read_bytes(),
  'source_untouched':(ROOT/'src/lib.rs').read_bytes()==original}
 save(OUT/'comparison.json',{'results':all_results,'checks':checks})
 assert not changed_png and checks['api_identical'] and checks['types_identical'] and checks['canvas_results_identical'] and checks['source_untouched'] and checks['browser_passed']
 print(json.dumps(checks,indent=2))
if __name__=='__main__': main()

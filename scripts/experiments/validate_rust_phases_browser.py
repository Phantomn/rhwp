#!/usr/bin/env python3
"""Repeat only browser validation of exact previously optimized artifacts."""
import json,os,pathlib,shutil
from validate_rust_phases import ROOT,OUT,sha,save,output,run_timed,RAW_HEAD

INCOMING=ROOT/'output/optimized-input'
MODES=['dual','cdylib-baseline-lto']
EXPECTED={'dual':'db615689b2d67ae143e21a4ef5ce14e3c436ff9a81ae8c929f0752fef9afcbb2',
 'cdylib-baseline-lto':'70223af2670ac316c0ecaa51149291b7c67052d2f32615aacaa1877645d3c782'}
FILES=['basic/KTX.hwp','biz_plan.hwp','tac-case-001.hwp','BlogForm_BookReview.hwp',
 'footnote-01.hwp','form-002.hwpx','kps-ai.hwp','number-bullet.hwp','oullim-01.hwp','para-head-num-2.hwp','shift-return.hwp']
PKG={'rhwp.js','rhwp_bg.wasm','rhwp.d.ts','rhwp_bg.wasm.d.ts'}

def main():
 OUT.mkdir(parents=True)
 fixtures=[]
 for name in FILES:
  public=ROOT/'rhwp-studio/public/samples'/name; served=ROOT/'samples'/name
  digest=sha(public)
  exists=served.exists()
  if exists: assert sha(served)==digest, f'existing fixture mismatch: {name}'
  else:
   served.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(public,served)
  fixtures.append({'name':name,'sha256':digest,'copied_missing_route':not exists})
 save(OUT/'fixtures.json',fixtures)
 source_digest=sha(ROOT/'src/lib.rs')
 all_results={}
 for mode in MODES:
  bundle=INCOMING/mode; dest=OUT/mode;dest.mkdir()
  manifest=json.loads((bundle/'build.json').read_text())
  assert manifest['success'] and manifest['source_sha']==RAW_HEAD and manifest['artifact_producer_run']==36514652526
  assert manifest['source_edit']['original_sha256']==source_digest
  assert set(manifest['artifacts'])==PKG
  assert manifest['artifacts']['rhwp_bg.wasm']['sha256']==EXPECTED[mode]
  for name in PKG:
   file=bundle/'pkg'/name
   assert not file.is_symlink() and sha(file)==manifest['artifacts'][name]['sha256']
   assert file.stat().st_size==manifest['artifacts'][name]['bytes']
  pkg=ROOT/'pkg';pkg.mkdir(exist_ok=True)
  for name in PKG: shutil.copy2(bundle/'pkg'/name,pkg/name)
  for name in ['rhwp.js','rhwp_bg.wasm']: shutil.copy2(pkg/name,ROOT/'rhwp-studio/public'/name)
  manifest['postprocessing_head']=manifest['validation_head']
  manifest['validation_head']=output(['git','rev-parse','HEAD'])
  manifest['optimized_producer_run']=36516437863
  manifest['postprocessing_reused']=True
  save(dest/'build.json',manifest)
  shutil.copytree(pkg,dest/'pkg');shutil.copy2(bundle/'api.json',dest/'api.json')
  env=dict(os.environ,RHWP_WASM_BUILD_MANIFEST=str(dest/'build.json'),RHWP_RENDER_DIFF_FILES=','.join(FILES),
   RHWP_RENDER_DIFF_MAX_PAGES='1',RHWP_RENDER_DIFF_WRITE_IMAGES='1')
  screenshots=ROOT/'rhwp-studio/e2e/screenshots/render-diff'
  assert not screenshots.exists()
  browser=run_timed(['npm','run','e2e:render-diff:ci'],dest/'browser.log',ROOT/'rhwp-studio',env,check=False)
  shutil.move(str(screenshots),str(dest/'screenshots'))
  all_results[mode]={'files':manifest['artifacts'],'binding':manifest['binding'],
   'optimization':manifest['optimization'],'postprocessing_reused':True,'browser':browser}
 a=OUT/'dual';b=OUT/'cdylib-baseline-lto'
 left={p.name:sha(p) for p in (a/'screenshots').glob('*.png')}
 right={p.name:sha(p) for p in (b/'screenshots').glob('*.png')}
 checks={'same_png':[name for name in left if right.get(name)==left[name]],
  'changed_png':[name for name in left.keys()|right.keys() if left.get(name)!=right.get(name)],
  'browser_passed':all(r['browser']['exit_code']==0 for r in all_results.values()),
  'api_identical':(a/'api.json').read_bytes()==(b/'api.json').read_bytes(),
  'types_identical':(a/'pkg/rhwp.d.ts').read_bytes()==(b/'pkg/rhwp.d.ts').read_bytes(),
  'canvas_results_identical':(a/'screenshots/results.json').read_bytes()==(b/'screenshots/results.json').read_bytes(),
  'source_untouched':sha(ROOT/'src/lib.rs')==source_digest}
 save(OUT/'comparison.json',{'results':all_results,'checks':checks,'optimized_producer_run':36516437863})
 print(json.dumps(checks,indent=2),flush=True)
 assert checks['browser_passed'] and checks['api_identical'] and checks['types_identical'] and checks['canvas_results_identical']
 assert len(left)==33 and not checks['changed_png'] and checks['source_untouched']
if __name__=='__main__': main()

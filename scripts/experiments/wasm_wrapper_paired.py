#!/usr/bin/env python3
"""One within-runner changed-source wrapper pair after both Cargo configurations are seeded."""
import hashlib,json,os
from pathlib import Path
import shlex,subprocess,sys
from wasm_wrapper_integration import sha,save,capture,timed,record_rustc
ROOT=Path.cwd(); OUT=ROOT/'output/wasm-wrapper-paired'

def main():
    OUT.mkdir(parents=True)
    assert not (ROOT/'target/pr-review').exists()
    assert not (ROOT/'pkg').exists()
    save(OUT/'environment.json',{'head':capture(['git','rev-parse','HEAD']),'rustc':capture(['rustc','-vV']),'cpu':Path('/proc/cpuinfo').read_text(),'cpu_count':os.cpu_count(),'image':os.environ.get('ImageVersion'),'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip()})
    wrapper=OUT/'rustc-record'
    wrapper.write_text('#!/bin/sh\nexec '+shlex.quote(sys.executable)+' '+shlex.quote(str(Path(__file__).resolve()))+' --rustc "$@"\n');wrapper.chmod(0o755)
    src=ROOT/'src/lib.rs'; original=src.read_bytes()
    before='env!("CARGO_PKG_VERSION").to_string()'; after='concat!(env!("CARGO_PKG_VERSION"), "-phase-probe-1").to_string()'
    assert original.decode().count(before)==1
    try:
        for label,mode,seed in [('seed-dual','dual',True),('seed-cdylib','cdylib',True),('changed-cdylib','cdylib',False),('changed-dual','dual',False)]:
            dest=OUT/label;dest.mkdir();records=dest/'compiler';records.mkdir()
            env=dict(os.environ,RHWP_WASM_CDYLIB_ONLY=str(int(mode=='cdylib')),RUSTC_WRAPPER=str(wrapper),RHWP_COMPILER_RECORDS=str(records),CARGO_TARGET_DIR='target/pr-review',CARGO_INCREMENTAL='0')
            if not seed: src.write_bytes(original.decode().replace(before,after).encode())
            # First seed warms wasm-pack tools too. Second seed needs only the alternate Cargo configuration.
            if label=='seed-cdylib':
                result=timed(['sh','scripts/wasm-pack-locked.sh','--target','web','--release','--no-opt'],dest/'command.log',env=env)
            else:
                result=timed(['python3','scripts/measure_render_diff_wasm.py','--profile','release','--output',str(dest)],dest/'command.log',env=env)
            save(dest/'command.json',result);assert result['exit_code']==0
            calls=[json.loads(p.read_text()) for p in records.glob('*.json')]
            roots=[r for r in calls if r['name']=='rhwp' and 'wasm32-unknown-unknown' in r['command']]
            assert len(roots)==1
            save(dest/'compiler-summary.json',{'count':len(calls),'root':roots[0],'source_sha256':sha(src),'seed':seed,'mode':mode})
            if not seed:
                assert len(calls)==1,'Only root may recompile in the measured pair'
                manifest=json.loads((dest/'build.json').read_text())
                expected={'dual':'db615689b2d67ae143e21a4ef5ce14e3c436ff9a81ae8c929f0752fef9afcbb2','cdylib':'70223af2670ac316c0ecaa51149291b7c67052d2f32615aacaa1877645d3c782'}
                assert manifest['artifacts']['rhwp_bg.wasm']['sha256']==expected[mode], 'Must match the already validated package exactly'
                assert sha(ROOT/'pkg/rhwp_bg.wasm')==sha(ROOT/'rhwp-studio/public/rhwp_bg.wasm')
                assert sha(ROOT/'pkg/rhwp.js')==sha(ROOT/'rhwp-studio/public/rhwp.js')
                assert sha(ROOT/'pkg/rhwp.js')=='2b7e7bb01cbbff0cb0d3c9a3222c6cb187f9d9710045013bcfb077d7abef4133'
    finally:
        src.write_bytes(original)
        save(OUT/'source-restoration.json',{'restored':src.read_bytes()==original,'diff':capture(['git','diff','--','src/lib.rs','Cargo.toml','Cargo.lock'])})
    a=json.loads((OUT/'changed-dual/build.json').read_text());b=json.loads((OUT/'changed-cdylib/build.json').read_text())
    assert a['tools']==b['tools']
    save(OUT/'comparison.json',{'dual_seconds':a['wall_seconds'],'cdylib_seconds':b['wall_seconds'],'difference_seconds':b['wall_seconds']-a['wall_seconds'],'percent':(b['wall_seconds']/a['wall_seconds']-1)*100,'same_runner':True,'n':1,'measured_order':['cdylib','dual'],'package_hashes_match_validated':True})

if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='--rustc': sys.exit(record_rustc(sys.argv[2:]))
    main()

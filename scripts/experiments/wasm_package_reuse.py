#!/usr/bin/env python3
"""Same-run Linux release package experiment, not a production cache policy."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT/'output/package-reuse'
FILES = {'LICENSE','README.md','package.json','rhwp.d.ts','rhwp.js','rhwp_bg.wasm','rhwp_bg.wasm.d.ts'}
MAX_BYTES = 64*1024*1024
TOOLS = {
    'wasm-bindgen': {'version':'wasm-bindgen 0.2.127','sha256':'f36dbe1938e78a79ce34d0ab3700907555161e293954f6f35bf6785a2f0cf916'},
    'wasm-opt': {'version':'wasm-opt version 117 (version_117)','sha256':'621b5a984f16d0323ea7aa8b389ac9ccda0950ddee5de0e99292970129f571ed'},
}


def capture(*args):
    return subprocess.check_output(args,cwd=ROOT,text=True).strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def metadata(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > MAX_BYTES:
        raise ValueError('unsafe or oversized package file')
    return dict(bytes=path.stat().st_size,sha256=sha(path))


def contract():
    if capture('git','status','--porcelain','--untracked-files=no'):
        raise ValueError('dirty tracked source')
    if platform.system() != 'Linux' or platform.machine() != 'x86_64':
        raise ValueError('experiment requires Linux x86_64')
    if os.environ.get('CARGO_TARGET_DIR') != 'target/pr-review':
        raise ValueError('unexpected target directory')
    return dict(schema=1,repository=os.environ['GITHUB_REPOSITORY'],run_id=os.environ['GITHUB_RUN_ID'],
        workflow_ref=os.environ['GITHUB_WORKFLOW_REF'],event=os.environ['GITHUB_EVENT_NAME'],
        source_sha=capture('git','rev-parse','HEAD'),source_tree=capture('git','rev-parse','HEAD^{tree}'),
        lock_sha256=sha(ROOT/'Cargo.lock'),profile='release',target='web',features='default',
        command=['sh','scripts/wasm-pack-locked.sh','--target','web','--release'],
        rustc=capture('rustc','-Vv'),cargo=capture('cargo','-V'),
        wasm_pack=capture('wasm-pack','--version'),wasm_pack_sha256=sha(Path(shutil.which('wasm-pack'))),
        tools=TOOLS,platform=platform.system(),architecture=platform.machine(),
        image_version=os.environ.get('ImageVersion'),
        build_env={k:v for k,v in sorted(os.environ.items()) if k.startswith(('CARGO_','RUST'))})


def validate_build(build, identity):
    if (not isinstance(build, dict) or build.get('success') is not True or build.get('source_dirty') is not False
        or build.get('source_sha') != identity['source_sha'] or build.get('profile') != 'release'
        or build.get('command') != identity['command'] or build.get('tools') != TOOLS
        or build.get('cargo_lock_sha256') != identity['lock_sha256']
        or build.get('wasm_pack') != identity['wasm_pack']):
        raise ValueError('build provenance or tool mismatch')
    commands=build['tool_commands']
    expected={
        'wasm-bindgen':[str(ROOT/'target/pr-review/wasm32-unknown-unknown/release/rhwp.wasm'),'--out-dir',str(ROOT/'pkg'),'--typescript','--target','web'],
        'wasm-opt':[str(ROOT/'pkg/rhwp_bg.wasm'),'-o',str(ROOT/'pkg/rhwp_bg.wasm-opt.wasm'),'-O'],
    }
    if set(commands) != set(expected) or any(commands[k][1:] != v for k,v in expected.items()):
        raise ValueError('tool options mismatch')


def publish(bundle):
    identity=contract()
    build=json.loads((ROOT/'output/render-diff-build/build.json').read_text())
    validate_build(build,identity)
    pkg=ROOT/'pkg'
    # wasm-pack may also write .gitignore; it is not part of the consumed package.
    hashes={name:metadata(pkg/name) for name in sorted(FILES)}
    if sum(v['bytes'] for v in hashes.values()) > MAX_BYTES:raise ValueError('package size limit')
    if any(build['artifacts'][name] != hashes[name] for name in ('rhwp.js','rhwp_bg.wasm')):
        raise ValueError('build output changed before publication')
    bundle.mkdir(parents=True,exist_ok=False)
    (bundle/'pkg').mkdir()
    for name in FILES:shutil.copyfile(pkg/name,bundle/'pkg'/name)
    manifest=dict(identity=identity,files=hashes,build=build,producer_boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip())
    (bundle/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')


def verify(bundle,identity):
    if bundle.is_symlink() or (bundle/'pkg').is_symlink():raise ValueError('symlink directory')
    metadata(bundle/'manifest.json')
    manifest=json.loads((bundle/'manifest.json').read_text())
    if manifest['identity'] != identity:raise ValueError('identity mismatch')
    validate_build(manifest['build'],identity)
    if set(manifest['files']) != FILES or {p.name for p in (bundle/'pkg').iterdir()} != FILES:
        raise ValueError('package file set mismatch')
    hashes={name:metadata(bundle/'pkg'/name) for name in FILES}
    if sum(v['bytes'] for v in hashes.values()) > MAX_BYTES or hashes != manifest['files']:
        raise ValueError('package checksum or size mismatch')
    if any(manifest['build']['artifacts'][n] != hashes[n] for n in ('rhwp.js','rhwp_bg.wasm')):
        raise ValueError('manifest artifact mismatch')
    return manifest


def restore(bundle,identity):
    # Validate the complete bundle before touching either consumer location.
    manifest=verify(bundle,identity)
    pkg=ROOT/'pkg';public=ROOT/'rhwp-studio/public'
    if pkg.is_symlink() or public.is_symlink():raise ValueError('symlink destination')
    pkg.mkdir(exist_ok=True)
    for name in FILES:
        if (pkg/name).is_symlink() or (public/name).is_symlink():raise ValueError('symlink destination file')
    for name in FILES:shutil.copyfile(bundle/'pkg'/name,pkg/name)
    for name in ('rhwp.js','rhwp_bg.wasm'):shutil.copyfile(pkg/name,public/name)
    build=ROOT/'output/render-diff-build';build.mkdir(parents=True,exist_ok=True)
    (build/'build.json').write_text(json.dumps(manifest['build'],indent=2)+'\n')
    return manifest


def consume(bundle):
    OUT.mkdir(parents=True,exist_ok=True);started=time.monotonic()
    result=dict(reused=False,consumer_boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip())
    try:
        manifest=restore(bundle,contract())
        result.update(reused=True,producer_boot_id=manifest['producer_boot_id'])
        if result['producer_boot_id']==result['consumer_boot_id']:raise RuntimeError('consumer is not a new runner')
    except (OSError,ValueError,KeyError,TypeError) as error:
        result['reason']=str(error)
    result['verify_install_seconds']=time.monotonic()-started
    (OUT/'reuse.json').write_text(json.dumps(result,indent=2)+'\n')
    with open(os.environ['GITHUB_OUTPUT'],'a') as out:out.write(f"reused={str(result['reused']).lower()}\n")
    print(json.dumps(result))


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('mode',choices=['publish','consume']);parser.add_argument('--bundle',type=Path,default=OUT/'bundle');args=parser.parse_args()
    if args.mode=='publish':publish(args.bundle)
    else:consume(args.bundle)

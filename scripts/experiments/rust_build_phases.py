#!/usr/bin/env python3
"""Bounded #7473 experiment; never invoked by production builds."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import time

ROOT = Path.cwd()
OUT = ROOT / 'output/rust-build-phases'
SELF = Path(__file__).resolve()

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def save(path, obj):
    Path(path).write_text(json.dumps(obj, indent=2) + '\n')

def checked(args, **kwargs):
    return subprocess.check_output(args, text=True, **kwargs).strip()

def wrap_rustc(args):
    # Cargo probes and dependencies are forwarded unchanged. Only root WASM is observed.
    if '--crate-name' not in args or args[args.index('--crate-name')+1] != 'rhwp' or 'wasm32-unknown-unknown' not in args:
        os.execv(args[0], args)
    dest = Path(os.environ['RHWP_PHASE_SAMPLE'])
    original = list(args)
    if os.environ.get('RHWP_PHASE_DIAGNOSTIC') == '1':
        args += ['-Ztime-passes', '-Zunstable-options', '-Csymbol-mangling-version=legacy', '-Clinker=' + str(OUT/'linker'), '-Clinker-flavor=wasm-lld']
    env = dict(os.environ)
    if os.environ.get('RHWP_PHASE_DIAGNOSTIC') == '1':
        env['RUSTC_BOOTSTRAP'] = '1'
    start = time.perf_counter()
    with (dest/'rustc.stderr').open('wb') as log:
        proc = subprocess.Popen(args, stderr=subprocess.PIPE, env=env)
        for line in proc.stderr:
            log.write(line)
            sys.stderr.buffer.write(line)
            sys.stderr.buffer.flush()
        code = proc.wait()
    save(dest/'rustc.json', {'original_command': original, 'executed_command': args,
         'seconds': time.perf_counter()-start, 'exit_code': code})
    return code

def wrap_linker(args):
    dest = Path(os.environ['RHWP_PHASE_SAMPLE'])
    real = os.environ['RHWP_REAL_LINKER']
    start = time.perf_counter()
    code = subprocess.call([real, *args])
    save(dest/'linker.json', {'command': [real, *args], 'seconds': time.perf_counter()-start, 'exit_code': code})
    return code

def measure(label, diagnostic, crate_type, marker, source):
    dest = OUT/label
    dest.mkdir(parents=True)
    src = ROOT/'src/lib.rs'
    original_expr = 'env!("CARGO_PKG_VERSION").to_string()'
    changed_expr = 'concat!(env!("CARGO_PKG_VERSION"), "-phase-probe-' + marker + '").to_string()'
    assert source.count(original_expr) == 1
    src.write_text(source.replace(original_expr, changed_expr))
    save(dest/'source.json', {'original_sha256': hashlib.sha256(source.encode()).hexdigest(),
        'changed_sha256': sha(src), 'before': original_expr, 'after': changed_expr})
    env = dict(os.environ, RUSTC_WRAPPER=str(OUT/'rustc-wrapper'),
        RHWP_PHASE_SAMPLE=str(dest), RHWP_PHASE_DIAGNOSTIC=str(int(diagnostic)),
        CARGO_TARGET_DIR='target/pr-review', CARGO_INCREMENTAL='0', CARGO_TERM_COLOR='never')
    sysroot = checked(['rustc','--print','sysroot'])
    host = re.search(r'^host: (.+)$', checked(['rustc','-vV']), re.M)[1]
    env['RHWP_REAL_LINKER'] = str(Path(sysroot)/'lib/rustlib'/host/'bin/rust-lld')
    command = ['cargo','rustc','--locked','-p','rhwp','--lib','--release','--target','wasm32-unknown-unknown','--message-format=json']
    if crate_type.startswith('cdylib'):
        command += ['--crate-type', 'cdylib']
    if crate_type == 'cdylib-baseline-lto':
        command += ['--config', 'profile.release.lto=false']
    start = time.perf_counter()
    with (dest/'cargo.jsonl').open('w') as stdout, (dest/'cargo.stderr').open('w') as stderr:
        code = subprocess.call(command, env=env, stdout=stdout, stderr=stderr)
    elapsed = time.perf_counter()-start
    artifacts=[]
    for line in (dest/'cargo.jsonl').read_text().splitlines():
        row=json.loads(line)
        if row.get('reason')=='compiler-artifact':
            artifacts.append({'name':row['target']['name'],'kind':row['target']['kind'],'fresh':row['fresh']})
    result={'label':label,'diagnostic':diagnostic,'crate_type':crate_type,'marker':marker,
        'command':command,'seconds':elapsed,'exit_code':code,'artifacts':artifacts}
    wasm = ROOT/'target/pr-review/wasm32-unknown-unknown/release/rhwp.wasm'
    if code == 0:
        result['wasm_sha256']=sha(wasm)
        result['wasm_bytes']=wasm.stat().st_size
        shutil.copy2(wasm,dest/'rhwp.wasm')
    save(dest/'build.json',result)
    print(json.dumps({k:v for k,v in result.items() if k!='artifacts'}),flush=True)
    if code:
        print((dest/'cargo.stderr').read_text()[-12000:],flush=True)
        raise RuntimeError(f'build {label} failed')
    assert any(not x['fresh'] and x['name']=='rhwp' for x in artifacts), 'root must recompile'
    return result

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--order',choices=['abba','baab'],default='abba')
    parser.add_argument('--candidate', choices=['fat-lto', 'baseline-lto'], default='fat-lto')
    args=parser.parse_args()
    OUT.mkdir(parents=True,exist_ok=True)
    # Shell executable entrypoints keep subprocess invocations independent of Python shebang PATH.
    for name,mode in [('rustc-wrapper','--rustc'),('linker','--linker')]:
        p=OUT/name
        import shlex
        p.write_text('#!/bin/sh\nexec '+shlex.quote(sys.executable)+' '+shlex.quote(str(SELF))+' '+mode+' "$@"\n')
        p.chmod(0o755)
    save(OUT/'environment.json',{'head':checked(['git','rev-parse','HEAD']),
        'rustc':checked(['rustc','-vV']),'cargo':checked(['cargo','-V']),
        'platform':platform.platform(),'cpu_count':os.cpu_count(),
        'cpu':Path('/proc/cpuinfo').read_text() if Path('/proc/cpuinfo').exists() else '',
        'boot_id':Path('/proc/sys/kernel/random/boot_id').read_text().strip() if Path('/proc/sys/kernel/random/boot_id').exists() else '',
        'image':{k:os.environ.get(k) for k in ['ImageOS','ImageVersion']}})
    src=ROOT/'src/lib.rs'; source=src.read_text()
    try:
        # Seed dependencies once; do not include it in warm comparisons.
        measure('seed',False,'dual','seed',source)
        candidate = 'cdylib' if args.candidate == 'fat-lto' else 'cdylib-baseline-lto'
        modes=['dual',candidate,candidate,'dual'] if args.order=='abba' else [candidate,'dual','dual',candidate]
        for i,mode in enumerate(modes):
            # First two diagnostic / last two plain; equal marker for A/B comparison.
            measure(f'{i+1}-{mode}',i<2,mode,'1',source)
    finally:
        src.write_text(source)
    assert subprocess.call(['git', 'diff', '--exit-code', '--', 'src/lib.rs', 'Cargo.lock', 'Cargo.toml']) == 0
    save(OUT/'complete.json',{'source_restored':src.read_text()==source,'order':args.order})

if __name__=='__main__':
    if len(sys.argv)>1 and sys.argv[1]=='--rustc':
        sys.exit(wrap_rustc(sys.argv[2:]))
    elif len(sys.argv)>1 and sys.argv[1]=='--linker':
        sys.exit(wrap_linker(sys.argv[2:]))
    else:
        main()

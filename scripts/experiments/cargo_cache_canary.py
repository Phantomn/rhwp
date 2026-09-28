#!/usr/bin/env python3
"""Temporary #7473 Cargo-only cache experiment; no product changes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'output/cargo-cache-canary'
TARGET = ROOT / 'target/pr-review'


def capture(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def key(profile, lock, rust, generation):
    if profile not in ('dev', 'release'):
        raise ValueError('unsupported profile')
    return f'canary-7473-v1-Linux-X64-wasm32-{profile}-{rust[:16]}-{lock[:16]}-{generation}'


def measure(profile, label):
    OUT.mkdir(parents=True, exist_ok=True)
    command = ['cargo', 'build', '--locked', '-p', 'rhwp', '--lib', '--target',
               'wasm32-unknown-unknown', '--profile', profile, '--target-dir',
               'target/pr-review', '--message-format=json-render-diagnostics', '-vv']
    env = {**os.environ, 'CARGO_LOG': 'cargo::core::compiler::fingerprint=info'}
    started = time.monotonic()
    with (OUT / f'{label}.jsonl').open('w') as stdout, (OUT / f'{label}.log').open('w') as stderr:
        proc = subprocess.run(command, cwd=ROOT, env=env, stdout=stdout, stderr=stderr)
    elapsed = time.monotonic() - started
    units = []
    for line in (OUT / f'{label}.jsonl').read_text().splitlines():
        event = json.loads(line)
        if event.get('reason') == 'compiler-artifact':
            units.append({k: event[k] for k in ('package_id', 'target', 'profile', 'fresh')})
    wasm = TARGET / 'wasm32-unknown-unknown' / ('debug' if profile == 'dev' else 'release') / 'rhwp.wasm'
    result = dict(label=label, profile=profile, command=command, wall_seconds=elapsed,
                  returncode=proc.returncode, source_sha=capture('git', 'rev-parse', 'HEAD'),
                  rustc=capture('rustc', '-Vv'), cargo=capture('cargo', '-V'),
                  cargo_lock_sha256=sha(ROOT / 'Cargo.lock'),
                  platform=platform.platform(), image_version=os.environ.get('ImageVersion'),
                  runner_name=os.environ.get('RUNNER_NAME'), hostname=platform.node(),
                  boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip() if Path('/proc/sys/kernel/random/boot_id').exists() else None,
                  cargo_incremental=os.environ.get('CARGO_INCREMENTAL'), units=units,
                  fresh_units=sum(x['fresh'] for x in units), rebuilt_units=sum(not x['fresh'] for x in units))
    if proc.returncode == 0:
        result['wasm'] = {'bytes': wasm.stat().st_size, 'sha256': sha(wasm)}
    (OUT / f'{label}.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k:v for k,v in result.items() if k not in ('units','rustc','command')}, indent=2))
    if proc.returncode:
        print((OUT / f'{label}.log').read_text()[-8000:])
        raise SystemExit(proc.returncode)
    if not units:
        raise RuntimeError('Cargo emitted no artifact evidence')


def identity(profile):
    OUT.mkdir(parents=True, exist_ok=True)
    lock = sha(ROOT / 'Cargo.lock')
    rust = hashlib.sha256(capture('rustc', '-Vv').encode()).hexdigest()
    generation = f"{os.environ['GITHUB_RUN_ID']}-{os.environ['GITHUB_RUN_ATTEMPT']}"
    own = key(profile, lock, rust, generation)
    other = key('release' if profile == 'dev' else 'dev', lock, rust, generation)
    with open(os.environ['GITHUB_OUTPUT'], 'a') as f:
        f.write(f'key={own}\nother_key={other}\n')
    (OUT / 'identity.json').write_text(json.dumps(dict(key=own, other_key=other, lock=lock,
        rust=rust, generation=generation, source_sha=capture('git','rev-parse','HEAD')), indent=2)+'\n')


def snapshot():
    # Estimate the archive before writing a shared Actions cache. The production action
    # uses its own archive; this bounded probe is recorded as seed overhead separately.
    paths = [TARGET, Path.home()/'.cargo/registry', Path.home()/'.cargo/git']
    paths = [str(p) for p in paths if p.exists()]
    archive = OUT / 'size-probe.tar.zst'
    started = time.monotonic()
    subprocess.run(['tar', '--use-compress-program', 'zstd -T0 -3', '-cf', str(archive), *paths], check=True)
    size = archive.stat().st_size
    result = dict(compressed_probe_bytes=size, wall_seconds=time.monotonic()-started,
                  max_bytes=512*1024*1024, paths=paths)
    (OUT/'cache-size.json').write_text(json.dumps(result,indent=2)+'\n')
    archive.unlink()
    print(json.dumps(result))
    if size > result['max_bytes']:
        raise RuntimeError('cache size guard: refusing save above 512 MiB')


def verify():
    seed = json.loads((ROOT/'output/canary-seed/seed.json').read_text())
    restored = json.loads((OUT/'restored.json').read_text())
    noop = json.loads((OUT/'same-runner-noop.json').read_text())
    assert seed['boot_id'] and restored['boot_id'] and seed['boot_id'] != restored['boot_id'], 'not a new runner'
    for field in ('source_sha','rustc','cargo_lock_sha256','profile','image_version','cargo_incremental'):
        assert seed[field] == restored[field] == noop[field], field
    assert seed['wasm'] == restored['wasm'] == noop['wasm'], 'artifact differs'
    assert noop['rebuilt_units'] == 0, 'no-op rebuilt crates'
    (OUT/'verification.json').write_text(json.dumps(dict(success=True,new_runner=True,wasm_identical=True),indent=2)+'\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=['identity','build','snapshot','verify'])
    parser.add_argument('--profile', choices=['dev','release'], default='dev')
    parser.add_argument('--label', default='seed')
    a=parser.parse_args()
    if a.mode=='identity': identity(a.profile)
    elif a.mode=='build': measure(a.profile,a.label)
    elif a.mode=='snapshot': snapshot()
    else: verify()

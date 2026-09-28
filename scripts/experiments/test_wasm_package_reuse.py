import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec=importlib.util.spec_from_file_location('reuse',Path(__file__).with_name('wasm_package_reuse.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)


class PackageReuse(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.old=m.ROOT;m.ROOT=Path(self.tmp.name);self.addCleanup(setattr,m,'ROOT',self.old)
        self.bundle=m.ROOT/'bundle';(self.bundle/'pkg').mkdir(parents=True)
        (m.ROOT/'rhwp-studio/public').mkdir(parents=True)
        self.identity=dict(source_sha='a'*40,profile='release',run_id='11',repository='edwardkim/rhwp',
            source_tree='b'*40,lock_sha256='c'*64,wasm_pack='wasm-pack 0.15.0',rustc='rustc pinned',
            command=['sh','scripts/wasm-pack-locked.sh','--target','web','--release'])
        for name in m.FILES:(self.bundle/'pkg'/name).write_bytes(('test '+name).encode())
        files={n:m.metadata(self.bundle/'pkg'/n) for n in m.FILES}
        build=dict(success=True,source_dirty=False,source_sha='a'*40,profile='release',
            command=self.identity['command'],tools=m.TOOLS,cargo_lock_sha256='c'*64,wasm_pack='wasm-pack 0.15.0',
            artifacts={n:files[n] for n in ('rhwp.js','rhwp_bg.wasm')},tool_commands={
                'wasm-bindgen':['/bin/wasm-bindgen',str(m.ROOT/'target/pr-review/wasm32-unknown-unknown/release/rhwp.wasm'),'--out-dir',str(m.ROOT/'pkg'),'--typescript','--target','web'],
                'wasm-opt':['/bin/wasm-opt',str(m.ROOT/'pkg/rhwp_bg.wasm'),'-o',str(m.ROOT/'pkg/rhwp_bg.wasm-opt.wasm'),'-O']})
        self.manifest=dict(identity=self.identity,files=files,build=build,producer_boot_id='producer')
        self.write(self.manifest)

    def write(self,value):
        (self.bundle/'manifest.json').write_text(json.dumps(value))

    def rejected_without_install(self):
        with self.assertRaises((ValueError,KeyError,OSError)):m.restore(self.bundle,self.identity)
        self.assertFalse((m.ROOT/'pkg').exists())
        self.assertEqual(list((m.ROOT/'rhwp-studio/public').iterdir()),[])

    def test_valid_installs_both_paths_and_browser_manifest(self):
        m.restore(self.bundle,self.identity)
        for name in m.FILES:self.assertEqual((m.ROOT/'pkg'/name).read_bytes(),(self.bundle/'pkg'/name).read_bytes())
        for name in ('rhwp.js','rhwp_bg.wasm'):self.assertEqual((m.ROOT/'pkg'/name).read_bytes(),(m.ROOT/'rhwp-studio/public'/name).read_bytes())
        self.assertTrue(json.loads((m.ROOT/'output/render-diff-build/build.json').read_text())['success'])

    def test_each_identity_boundary_rejects_before_install(self):
        for name in ('source_sha','source_tree','profile','run_id','repository','rustc','wasm_pack','lock_sha256','command'):
            with self.subTest(field=name):
                changed=copy.deepcopy(self.manifest);changed['identity'][name]='different';self.write(changed);self.rejected_without_install()

    def test_build_provenance_and_tools_reject(self):
        for field,value in [('success',False),('source_dirty',True),('source_sha','z'*40),('profile','dev'),('tools',{}),('tool_commands',{})]:
            with self.subTest(field=field):
                changed=copy.deepcopy(self.manifest);changed['build'][field]=value;self.write(changed);self.rejected_without_install()

    def test_changed_optimizer_options_reject(self):
        changed=copy.deepcopy(self.manifest);changed['build']['tool_commands']['wasm-opt'][-1]='-Oz';self.write(changed);self.rejected_without_install()

    def test_corrupt_wasm_rejects(self):
        (self.bundle/'pkg/rhwp_bg.wasm').write_bytes(b'corrupt');self.rejected_without_install()

    def test_corrupt_js_rejects(self):
        (self.bundle/'pkg/rhwp.js').write_bytes(b'corrupt');self.rejected_without_install()

    def test_missing_file_rejects(self):
        (self.bundle/'pkg/rhwp.d.ts').unlink();self.rejected_without_install()

    def test_extra_file_rejects(self):
        (self.bundle/'pkg/extra.js').write_text('extra');self.rejected_without_install()

    def test_symlink_file_rejects(self):
        p=self.bundle/'pkg/rhwp.js';p.unlink();p.symlink_to('/etc/hosts');self.rejected_without_install()

    def test_symlink_directory_rejects(self):
        (self.bundle/'pkg').rename(self.bundle/'actual');(self.bundle/'pkg').symlink_to(self.bundle/'actual');self.rejected_without_install()

    def test_size_guard_rejects(self):
        p=self.bundle/'pkg/rhwp_bg.wasm'
        with p.open('wb') as f:f.truncate(m.MAX_BYTES+1)
        self.rejected_without_install()

    def test_corrupt_manifest_rejects(self):
        (self.bundle/'manifest.json').write_text('{');self.rejected_without_install()

    def test_missing_bundle_rejects(self):
        (self.bundle/'manifest.json').unlink();self.rejected_without_install()


if __name__=='__main__':unittest.main()

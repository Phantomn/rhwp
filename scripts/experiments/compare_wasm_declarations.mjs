import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync } from 'node:child_process';
const require = createRequire(new URL('../../rhwp-studio/package.json', import.meta.url));
const tsPackage = require.resolve('typescript/package.json');
const tsVersion = JSON.parse(readFileSync(tsPackage, 'utf8')).version;
const tsc = join(dirname(tsPackage), 'bin/tsc');
const [leftPath, rightPath] = process.argv.slice(2);
const left = readFileSync(leftPath, 'utf8');
const right = readFileSync(rightPath, 'utf8');
const first = '    readonly version: () => [number, number];\n    readonly init_panic_hook: () => void;';
const second = '    readonly init_panic_hook: () => void;\n    readonly version: () => [number, number];';
// Strictly prove that the sole text difference is the observed property permutation.
const onlyObservedOrderDiff = left.split(first).length === 2 && left.replace(first, second) === right;
const temp = mkdtempSync(join(tmpdir(), 'rhwp-types-'));
try {
  writeFileSync(join(temp, 'a.d.ts'), left);
  writeFileSync(join(temp, 'b.d.ts'), right);
  writeFileSync(join(temp, 'check.ts'), `import type { InitOutput as A } from './a';\nimport type { InitOutput as B } from './b';\ndeclare const a:A; declare const b:B;\nconst toB:B=a; const toA:A=b;\nvoid toB; void toA;\n`);
  writeFileSync(join(temp, 'tsconfig.json'), JSON.stringify({compilerOptions:{strict:true,noEmit:true,target:'ES2022',module:'ESNext',moduleResolution:'Bundler',lib:['ESNext','DOM'],types:[]},files:['check.ts']}));
  const run = () => spawnSync(process.execPath, [tsc, '--project', join(temp,'tsconfig.json')], {encoding:'utf8'});
  const positive = run();
  const needle = 'readonly version: () => [number, number];';
  writeFileSync(join(temp,'b.d.ts'),right.replace(needle,'readonly version: () => boolean;'));
  const negative = run();
  const result = {byteIdentical:left===right,onlyObservedOrderDiff,mutuallyAssignable:positive.status===0,
    changedReturnTypeRejected:negative.status!==0 && /version/.test(negative.stdout+negative.stderr),
    typescriptVersion:tsVersion,positiveDiagnostics:positive.stdout+positive.stderr,negativeDiagnostics:negative.stdout+negative.stderr};
  console.log(JSON.stringify(result,null,2));
  if (!onlyObservedOrderDiff || !result.mutuallyAssignable || !result.changedReturnTypeRejected) process.exitCode=1;
} finally { rmSync(temp,{recursive:true,force:true}); }

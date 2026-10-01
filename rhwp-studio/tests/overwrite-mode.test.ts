import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as nodeModule from 'node:module';

// [#7489] Insert 키로 수정 모드를 켜면 입력이 캐럿 뒤 글자를 덮어써야 한다(한컴과 같다).
// 종전에는 상태 표시줄 문구만 바뀌고 입력은 항상 끼워 넣었다. 실제 입력 핸들러와
// InsertTextCommand·CommandHistory 를 러너에서 실행해 문서 결과와 되돌리기를 검증한다.

const here = dirname(fileURLToPath(import.meta.url));
const runner = join(here, 'support', 'overwrite-mode.runner.mjs');

function transformTypesSupported(): boolean {
  return process.allowedNodeEnvironmentFlags.has('--experimental-transform-types')
    && typeof (nodeModule as { registerHooks?: unknown }).registerHooks === 'function';
}

test('수정 모드 입력은 캐럿 뒤 일반 글자를 덮어쓰고 한 번에 되돌린다 (자식 프로세스)', (t) => {
  if (!transformTypesSupported()) {
    t.skip('현재 Node 가 --experimental-transform-types / registerHooks 미지원 — 행위 테스트 skip');
    return;
  }
  const res = spawnSync(
    process.execPath,
    ['--experimental-transform-types', '--no-warnings', runner],
    { encoding: 'utf8' },
  );
  assert.equal(res.status, 0,
    `러너가 비정상 종료했습니다.\n--- stdout ---\n${res.stdout}\n--- stderr ---\n${res.stderr}`);
  assert.match(res.stdout, /OVERWRITE_MODE_OK/, '행위 검증 성공 마커가 있어야 함');
});

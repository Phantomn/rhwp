// [#7489] 수정(덮어쓰기) 모드 행위 러너. 실제 onInput/onCompositionStart/onCompositionEnd,
// InsertTextCommand, CommandHistory 를 mock this 와 문단 모델 wasm 에 대고 실행한다.
// command.ts 의 파라미터 프로퍼티 때문에 부모 테스트가 --experimental-transform-types 로 spawn 한다
// (composition-hf-fn-reanchor.runner.mjs 와 같은 패턴).
import { registerHooks } from 'node:module';
import { pathToFileURL, fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import assert from 'node:assert/strict';

const studioDir = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const srcDir = join(studioDir, 'src');

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier.startsWith('@/')) {
      const abs = join(srcDir, specifier.slice(2));
      const withTs = abs.endsWith('.ts') ? abs : abs + '.ts';
      return { url: pathToFileURL(withTs).href, shortCircuit: true };
    }
    if ((specifier.startsWith('./') || specifier.startsWith('../')) && !/\.[cm]?[tj]s$/.test(specifier)) {
      const parent = context.parentURL ? dirname(fileURLToPath(context.parentURL)) : srcDir;
      return { url: pathToFileURL(join(parent, specifier + '.ts')).href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
});

const text = await import(pathToFileURL(join(srcDir, 'engine', 'input-handler-text.ts')).href);
const { CommandHistory } = await import(pathToFileURL(join(srcDir, 'engine', 'history.ts')).href);

/**
 * 본문 문단 하나(sec 0, para 0)와 셀 문단 하나를 담은 모델. 글자처럼 취급한 개체는 텍스트에
 * 없고 `objects`(텍스트 위치)로만 있다 — rhwp 의 text/char_offsets 분리와 같다.
 *
 * 본문에는 링크·누름틀 범위(`ranges`)와 글자별 굵게(`bold`)가 있다. 범위와 글자 모양은 실제
 * rhwp 처럼 움직인다(실제 WASM 으로 확인): 지우면 범위가 줄고, 범위 시작에 넣은 글자는 범위
 * 밖(앞)으로, 빈 누름틀에 넣은 글자는 누름틀 안으로 간다. 빈 링크는 그대로 남는다. 새 글자는
 * 앞 글자의 모양을 따른다. 그래서 지운 글자를 다시 넣기만 하면 범위·모양이 원래대로 안 돌아온다.
 * 문단 조각(captureDeleteRange)은 문단을 통째로 복사해 두었다가 되돌린다.
 *
 * 캐럿이 들어간 누름틀은 활성(setActiveField)이 된다. 실제 rhwp 처럼 활성 누름틀은 시작·끝에
 * 넣은 글자도 안으로 받는다. 활성 상태는 문서 데이터가 아니어서 조각 복원과 무관하다.
 * 셀 문단도 같은 규칙의 누름틀 범위(`cellRanges`)를 가진다.
 */
function makeWasm({ body = '', cell = '', objects = [], ranges = [], cellRanges = [], bold = [] } = {}) {
  const toRanges = (list) => list.map(([kind, start, end]) => ({ kind, start, end }));
  const doc = {
    body,
    cell,
    ranges: toRanges(ranges),
    cellRanges: toRanges(cellRanges),
    bold: [...body].map((_, i) => i >= bold[0] && i < bold[1]),
  };
  const fragments = new Map();
  let nextFragmentId = 1;
  let active = null; // { inCell, index }
  const rangesAt = (pos) => (pos.parentParaIndex === undefined ? doc.ranges : doc.cellRanges);
  const fieldIndexAt = (pos) =>
    rangesAt(pos).findIndex((r) => r.kind === 'field' && r.start <= pos.charOffset && pos.charOffset <= r.end);
  const moveRanges = (ranges, inCell, off, del, n) => {
    ranges.forEach((r, i) => {
      const shrink = (p) => (p >= off + del ? p - del : Math.min(p, off));
      r.start = shrink(r.start);
      r.end = shrink(r.end);
      if (n === 0) return;
      const inside = r.kind === 'field' && active?.inCell === inCell && active.index === i;
      if (r.start > off || (r.start === off && r.start < r.end && !inside)) {
        r.start += n;
        r.end += n;
      } else if (r.start === off ? r.kind === 'field' : off < r.end || (off === r.end && inside)) {
        r.end += n;
      }
    });
  };
  const editBody = (off, del, ins) => {
    const chars = [...doc.body];
    const added = [...ins];
    chars.splice(off, del, ...added);
    doc.body = chars.join('');
    doc.bold.splice(off, del, ...added.map(() => off > 0 && doc.bold[off - 1]));
    moveRanges(doc.ranges, false, off, del, added.length);
  };
  const editCell = (off, del, ins) => {
    const chars = [...doc.cell];
    chars.splice(off, del, ...ins);
    doc.cell = chars.join('');
    moveRanges(doc.cellRanges, true, off, del, ins.length);
  };
  const cellResult = (off) => ({ ok: true, charOffset: off, paginationDeferred: false, cellFlowChanged: false });
  return {
    doc,
    fragments,
    getTextRange: (_s, _p, off, n) => [...doc.body].slice(off, off + n).join(''),
    getTextInCell: (_s, _pp, _ci, _cei, _cpi, off, n) => [...doc.cell].slice(off, off + n).join(''),
    insertText: (_s, _p, off, t) => editBody(off, 0, [...t]),
    deleteText: (_s, _p, off, n) => editBody(off, n, []),
    replaceBodyTextLocal: (_s, _p, off, del, t) => {
      editBody(off, del, [...t]);
      return { ok: true, charOffset: off + [...t].length, documentPaginationPending: false, flowChanged: false };
    },
    insertTextInCell: (_s, _pp, _ci, _cei, _cpi, off, t) => editCell(off, 0, [...t]),
    deleteTextInCell: (_s, _pp, _ci, _cei, _cpi, off, n) => editCell(off, n, []),
    insertTextInCellDeferredPagination: (_s, _pp, _ci, _cei, _cpi, off, t) => {
      editCell(off, 0, [...t]);
      return cellResult(off + [...t].length);
    },
    replaceTextInCellDeferredPagination: (_s, _pp, _ci, _cei, _cpi, off, del, t) => {
      editCell(off, del, [...t]);
      return cellResult(off + [...t].length);
    },
    textToLogicalOffset: (_s, _p, off) => off + objects.filter((p) => p < off).length,
    getFieldInfoAt: (pos) => {
      const fieldId = fieldIndexAt(pos);
      if (fieldId < 0) return { inField: false };
      const { start, end } = rangesAt(pos)[fieldId];
      return { inField: true, fieldType: 'clickhere', fieldId, startCharIdx: start, endCharIdx: end, editableInForm: true };
    },
    // 실제 rhwp 처럼 누름틀이 없는 자리면 활성 상태를 그대로 둔다.
    setActiveField: (pos) => {
      const index = fieldIndexAt(pos);
      const inCell = pos.parentParaIndex !== undefined;
      if (index < 0 || (active?.inCell === inCell && active.index === index)) return false;
      active = { inCell, index };
      return true;
    },
    clearActiveField: () => { active = null; },
    captureDeleteRange: () => {
      fragments.set(nextFragmentId, structuredClone(doc));
      return nextFragmentId++;
    },
    restoreDeleteFragment: (id) => {
      if (!fragments.has(id)) throw new Error(`삭제 조각 ${id} 없음`);
      Object.assign(doc, structuredClone(fragments.get(id)));
      fragments.delete(id);
      return '{"ok":true}';
    },
    discardDeleteFragment: (id) => { fragments.delete(id); },
    snapshotCapacity: () => 100,
  };
}

const bodyPos = (charOffset) => ({ sectionIndex: 0, paragraphIndex: 0, charOffset });
const cellPos = (charOffset) => ({
  sectionIndex: 0, paragraphIndex: 0, charOffset,
  parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex: 0,
});
const rangesOf = (wasm, key = 'ranges') => wasm.doc[key].map((r) => `${r.kind} ${r.start}-${r.end}`).join(', ');
const boldOf = (wasm) => [...wasm.doc.body].map((ch, i) => (wasm.doc.bold[i] ? ch.toUpperCase() : ch)).join('');

/** InputHandler 가 이 경로에서 실제로 쓰는 필드·메서드만 채운 mock this. */
function makeHandler(wasm, start, { insertMode = false, editMode = 'normal', exitedFieldEnd = false } = {}) {
  let position = start;
  const history = new CommandHistory();
  // InputHandler 의 양식 모드 판정과 같다: 편집 가능한 누름틀 안에서만 넣고 지운다.
  const fieldAt = (pos) => {
    const fi = wasm.getFieldInfoAt(pos);
    return editMode === 'form' && fi.inField && fi.editableInForm ? fi : null;
  };
  // InputHandler.updateFieldMarkers 와 같다: 캐럿이 누름틀 안이면 활성화하고, 밖이나 빠져나온 끝이면 해제한다.
  const syncActiveField = () => {
    const fi = wasm.getFieldInfoAt(position);
    if (fi.inField && !h.isAtExitedFieldEnd(position, fi)) wasm.setActiveField(position);
    else wasm.clearActiveField();
  };
  const h = {
    active: true,
    insertMode,
    wasm,
    history,
    textarea: { value: '' },
    isComposing: false,
    compositionAnchor: null,
    compositionLength: 0,
    _compositionCovered: '',
    _compositionFragment: null,
    _lastCompositionText: '',
    _lastComposedText: '',
    caret: { hideComposition() {} },
    cursor: {
      isInHeaderFooter: () => false,
      isInFootnote: () => false,
      hasSelection: () => false,
      getPosition: () => ({ ...position }),
      getRect: () => ({ pageIndex: 0 }),
      moveTo: (pos) => { position = { ...pos }; },
    },
    canInsertTextInFormMode(pos) {
      if (editMode !== 'form') return true;
      const fi = fieldAt(pos);
      return !!fi && pos.charOffset >= fi.startCharIdx && pos.charOffset <= fi.endCharIdx;
    },
    canDeleteTextInFormMode(pos, count) {
      if (editMode !== 'form') return true;
      const fi = fieldAt(pos);
      return !!fi && pos.charOffset >= fi.startCharIdx && pos.charOffset + count <= fi.endCharIdx;
    },
    // 오른쪽 화살표로 누름틀 끝을 빠져나온 상태
    isAtExitedFieldEnd: (pos, fi) => exitedFieldEnd && pos.charOffset === fi?.endCharIdx,
    resetRawTextMutationEffects() {},
    consumeRawTextMutationBeforeCursor: () => false,
    prepareTextMutationBeforeCursor: () => false,
    flushDeferredPaginationIfNeeded() {},
    // 실제 화면 갱신은 모두 updateCaret → updateFieldMarkers 를 거친다.
    afterTextInputEdit: syncActiveField,
    afterEdit: syncActiveField,
    updateCaret: syncActiveField,
    executeOperation(desc) {
      // InputHandler.isOperationAllowedInEditMode 와 같다: 기록은 늘 통과, 입력은 넣을 수 있는 자리만.
      if (desc.kind === 'command') {
        if (!this.canInsertTextInFormMode(desc.command.position)) return;
        position = history.execute(desc.command, wasm);
        syncActiveField();
      } else {
        history.recordWithoutExecute(desc.command, wasm);
      }
    },
    undo() { position = history.undo(wasm); syncActiveField(); },
    redo() { position = history.redo(wasm); syncActiveField(); },
    type(s) {
      for (const ch of s) {
        this.textarea.value = ch;
        text.onInput.call(this);
      }
    },
    /** 한 음절 조합: 갱신 문자열들을 차례로 넣고 끝낸다. */
    compose(...updates) {
      text.onCompositionStart.call(this);
      for (const u of updates) {
        this.textarea.value = u;
        text.onInput.call(this);
      }
      text.onCompositionEnd.call(this);
    },
  };
  h.getTextAt = (pos, n) => text.getTextAt.call(h, pos, n);
  h.replaceTextAtRaw = (pos, del, t) => text.replaceTextAtRaw.call(h, pos, del, t);
  h.insertTextAtRaw = (pos, t) => text.insertTextAtRaw.call(h, pos, t);
  syncActiveField();
  return h;
}

// 시나리오마다 실패를 모아 한꺼번에 알린다 — 앞 시나리오가 깨져도 나머지 결과를 볼 수 있다.
const failures = [];
function scenario(name, fn) {
  try {
    fn();
  } catch (err) {
    failures.push(`${name}: ${err.message}`);
  }
}

scenario('1. 이슈 재현: abcd, Home, 수정 모드, XY → XYcd, 한 번에 되돌린다', () => {
  const wasm = makeWasm({ body: 'abcd' });
  const h = makeHandler(wasm, bodyPos(0));
  h.type('XY');
  assert.equal(wasm.doc.body, 'XYcd', '수정 모드는 캐럿 뒤 글자를 입력한 만큼 덮어써야 한다');
  assert.equal(h.cursor.getPosition().charOffset, 2, '캐럿은 입력 글자 뒤에 있어야 한다');
  h.undo();
  assert.equal(wasm.doc.body, 'abcd', '연속 입력은 한 번의 되돌리기로 원문이 돼야 한다');
  assert.equal(h.history.canUndo(), false, '되돌릴 편집이 하나만 기록돼야 한다');
  assert.equal(wasm.fragments.size, 0, '되돌린 뒤 문단 조각이 남으면 안 된다');
  h.redo();
  assert.equal(wasm.doc.body, 'XYcd', '다시 실행은 같은 덮어쓰기를 재현해야 한다');
});

scenario('2. 삽입 모드는 그대로 끼워 넣는다', () => {
  const wasm = makeWasm({ body: 'abcd' });
  makeHandler(wasm, bodyPos(0), { insertMode: true }).type('XY');
  assert.equal(wasm.doc.body, 'XYabcd');
  assert.equal(wasm.fragments.size, 0, '삽입 모드는 문단 조각을 잡지 않는다');
});

scenario('3. 문단 끝에서는 덮을 글자가 없어 삽입한다', () => {
  const wasm = makeWasm({ body: 'abc' });
  const h = makeHandler(wasm, bodyPos(2));
  h.type('XY');
  assert.equal(wasm.doc.body, 'abXY', 'c 를 덮은 뒤 문단 끝에서는 삽입해야 한다');
  h.undo();
  assert.equal(wasm.doc.body, 'abc');
});

scenario('4. 탭·개체·누름틀 끝은 덮어쓰지 않는다', () => {
  let wasm = makeWasm({ body: 'a\tb' });
  makeHandler(wasm, bodyPos(1)).type('X');
  assert.equal(wasm.doc.body, 'aX\tb', '탭은 지우지 않고 그 앞에 삽입해야 한다');
  // a [개체] b — 개체는 텍스트 위치 1, 캐럿은 개체 앞
  wasm = makeWasm({ body: 'ab', objects: [1] });
  makeHandler(wasm, bodyPos(1)).type('X');
  assert.equal(wasm.doc.body, 'aXb', '개체 너머의 b 를 덮어쓰면 안 된다');
  // 빈 누름틀(1..1) 안에서 입력 — 필드 밖 b 를 지우면 안 된다
  wasm = makeWasm({ body: 'ab', ranges: [['field', 1, 1]] });
  makeHandler(wasm, bodyPos(1)).type('X');
  assert.equal(wasm.doc.body, 'aXb', '누름틀 끝 너머 글자를 덮어쓰면 안 된다');
});

scenario('5. 표 셀도 같은 입력 경로로 덮어쓴다', () => {
  const wasm = makeWasm({ cell: 'abcd' });
  const h = makeHandler(wasm, cellPos(1));
  h.type('XY');
  assert.equal(wasm.doc.cell, 'aXYd');
  h.undo();
  assert.equal(wasm.doc.cell, 'abcd');
});

scenario('6. IME 조합: 음절마다 한 글자를 한 번만 덮고, 한 번의 되돌리기로 돌아간다', () => {
  const wasm = makeWasm({ body: 'abcd' });
  const h = makeHandler(wasm, bodyPos(0));
  h.compose('ㅎ', '하', '한');
  assert.equal(wasm.doc.body, '한bcd', '조합 갱신이 거듭돼도 덮는 글자는 a 하나뿐이어야 한다');
  h.compose('ㄱ', '그', '글');
  assert.equal(wasm.doc.body, '한글cd');
  h.undo();
  assert.equal(wasm.doc.body, 'abcd', '조합 입력도 일반 입력처럼 한 번에 되돌아가야 한다');
  assert.equal(wasm.fragments.size, 0, '병합된 조합의 문단 조각도 모두 정리돼야 한다');
});

scenario('7. 조합 취소: 덮었던 글자를 되살리고 아무것도 기록하지 않는다', () => {
  const wasm = makeWasm({ body: 'abcd' });
  const h = makeHandler(wasm, bodyPos(0));
  h.compose('ㅎ', '');
  assert.equal(wasm.doc.body, 'abcd', '조합을 지우면 덮었던 a 가 돌아와야 한다');
  assert.equal(h.cursor.getPosition().charOffset, 0);
  assert.equal(h.history.canUndo(), false, '취소된 조합은 기록하지 않는다');
  assert.equal(wasm.fragments.size, 0, '취소한 조합의 문단 조각이 남으면 안 된다');
});

// ── 범위·글자 모양: abcdef 의 cd(2..4)에 링크 ─────────────────────────────────
const linkDoc = () => makeWasm({ body: 'abcdef', ranges: [['link', 2, 4]] });

scenario('8. 링크 글자를 덮어쓴 뒤 되돌리면 링크 범위도 2-4 로 돌아온다', () => {
  const wasm = linkDoc();
  const h = makeHandler(wasm, bodyPos(2));
  h.type('X');
  assert.equal(wasm.doc.body, 'abXdef');
  const typed = rangesOf(wasm);
  h.undo();
  assert.equal(wasm.doc.body, 'abcdef');
  assert.equal(rangesOf(wasm), 'link 2-4', '되돌리면 c 가 링크 안으로 돌아와야 한다');
  h.redo();
  assert.equal(rangesOf(wasm), typed, '다시 실행은 처음 입력과 같은 범위를 만들어야 한다');
});

scenario('9. IME 로 링크 글자를 덮어 확정한 뒤 되돌리면 링크 범위가 돌아온다', () => {
  const wasm = linkDoc();
  const h = makeHandler(wasm, bodyPos(2));
  h.compose('ㅎ', '하');
  assert.equal(wasm.doc.body, 'ab하def');
  h.undo();
  assert.equal(wasm.doc.body, 'abcdef');
  assert.equal(rangesOf(wasm), 'link 2-4');
});

scenario('10. 링크 글자 위 IME 조합을 취소하면 링크 범위까지 그대로다', () => {
  const wasm = linkDoc();
  const h = makeHandler(wasm, bodyPos(2));
  h.compose('ㅎ', '');
  assert.equal(wasm.doc.body, 'abcdef');
  assert.equal(rangesOf(wasm), 'link 2-4', '취소하면 c 가 링크 안에 있어야 한다');
  assert.equal(h.history.canUndo(), false);
});

scenario('11. 링크 글자 전체를 빠르게 덮어쓴 뒤 한 번 되돌리면 링크가 돌아온다', () => {
  const wasm = linkDoc();
  const h = makeHandler(wasm, bodyPos(2));
  h.type('XY');
  assert.equal(wasm.doc.body, 'abXYef');
  const typed = rangesOf(wasm);
  h.undo();
  assert.equal(h.history.canUndo(), false, '연속 입력은 한 번에 되돌아가야 한다');
  assert.equal(rangesOf(wasm), 'link 2-4', '빈 링크로 남으면 안 된다');
  assert.equal(wasm.fragments.size, 0);
  h.redo();
  assert.equal(rangesOf(wasm), typed, '병합된 입력도 다시 실행하면 처음 입력과 같아야 한다');
});

scenario('12. 누름틀 앞에서 들어가며 빠르게 덮어쓴 뒤 한 번 되돌리면 누름틀이 2-4 다', () => {
  // abcde 의 cd(2..4)가 누름틀. 캐럿 1 에서 XYZ — b 를 덮고 누름틀 안의 c, d 로 이어진다.
  const wasm = makeWasm({ body: 'abcde', ranges: [['field', 2, 4]] });
  const h = makeHandler(wasm, bodyPos(1));
  h.type('XYZ');
  assert.equal(wasm.doc.body, 'aXYZe');
  h.undo();
  assert.equal(wasm.doc.body, 'abcde');
  assert.equal(h.history.canUndo(), false, '연속 입력은 한 번에 되돌아가야 한다');
  assert.equal(rangesOf(wasm), 'field 2-4', '누름틀이 b 까지 넓어지면 안 된다');
});

scenario('13. 굵은 글자를 덮어쓴 뒤 되돌리면 굵게가 돌아온다', () => {
  const wasm = makeWasm({ body: 'abcdef', bold: [2, 4] });
  const h = makeHandler(wasm, bodyPos(2));
  h.type('XY');
  h.undo();
  assert.equal(boldOf(wasm), 'abCDef', 'c, d 가 굵게 돌아와야 한다');
});

// ── 양식 모드: abcde 의 cd(2..4)가 편집 가능한 누름틀, 캐럿은 끝을 빠져나온 4 ──────────
// 누름틀 밖 e 는 지울 수 없다. 수정 모드도 삽입 모드와 똑같이 끝나야 한다.
function formRun(insertMode, act) {
  const wasm = makeWasm({ body: 'abcde', ranges: [['field', 2, 4]] });
  const h = makeHandler(wasm, bodyPos(4), { insertMode, editMode: 'form', exitedFieldEnd: true });
  act(h);
  return { body: wasm.doc.body, ranges: rangesOf(wasm), canUndo: h.history.canUndo() };
}

scenario('14. 양식 모드: 누름틀 끝을 나와 입력해도 보호된 e 를 지우지 않는다', () => {
  const overwrite = formRun(false, (h) => h.type('X'));
  assert.equal(overwrite.body, 'abcdXe', '보호된 e 가 남아야 한다');
  assert.deepEqual(overwrite, formRun(true, (h) => h.type('X')), '삽입 모드와 같아야 한다');
});

scenario('15. 양식 모드: 같은 자리의 IME 조합 취소는 e 를 겹치지 않는다', () => {
  const act = (h) => h.compose('ㅎ', '');
  const overwrite = formRun(false, act);
  assert.ok(!overwrite.body.includes('ee'), `e 가 겹쳤다: ${overwrite.body}`);
  assert.deepEqual(overwrite, formRun(true, act), '삽입 모드와 같아야 한다');
});

scenario('16. 양식 모드: 같은 자리의 IME 확정은 삽입 모드와 같이 기록된다', () => {
  const act = (h) => {
    h.compose('ㅎ', '하');
    h.undo();
  };
  const overwrite = formRun(false, (h) => h.compose('ㅎ', '하'));
  assert.equal(overwrite.body.at(-1), 'e', '보호된 e 가 남아야 한다');
  assert.deepEqual(overwrite, formRun(true, (h) => h.compose('ㅎ', '하')), '삽입 모드와 같아야 한다');
  assert.equal(formRun(false, act).body, 'abcde', '기록된 편집을 되돌리면 원문이어야 한다');
});

// ── 누름틀 바로 앞의 IME: abcde 의 cd(2..4)가 누름틀 ───────────────────────────────
// b 를 덮은 첫 조합 글자 뒤 캐럿이 누름틀 시작에 서면 누름틀이 활성화된다. 다음 조합 갱신이
// 그 글자를 지웠다 다시 넣어도 누름틀 안으로 끌려가면 안 된다.
const fieldDoc = () => makeWasm({ body: 'abcde', ranges: [['field', 2, 4]] });

scenario('17. 누름틀 바로 앞 글자를 덮은 IME 조합은 누름틀 밖에 남는다', () => {
  // Home, → 로 캐럿 1
  let wasm = fieldDoc();
  const h = makeHandler(wasm, bodyPos(1));
  h.compose('ㅎ', '하');
  assert.equal(wasm.doc.body, 'a하cde');
  assert.equal(rangesOf(wasm), 'field 2-4', '하가 누름틀 안으로 들어가면 안 된다');
  // 이어 치면 일반 입력처럼 누름틀 안 글자를 덮는다.
  h.compose('ㄷ', '다');
  assert.equal(wasm.doc.body, 'a하다de');
  assert.equal(rangesOf(wasm), 'field 2-4');
  h.undo();
  assert.equal(wasm.doc.body, 'abcde');
  assert.equal(rangesOf(wasm), 'field 2-4');
  // 처음부터 '한글' — 두 번째 음절이 b 를 덮는다.
  wasm = fieldDoc();
  const h2 = makeHandler(wasm, bodyPos(0));
  h2.compose('ㅎ', '하', '한');
  h2.compose('ㄱ', '그', '글');
  assert.equal(wasm.doc.body, '한글cde');
  assert.equal(rangesOf(wasm), 'field 2-4', '글이 누름틀 안으로 들어가면 안 된다');
});

scenario('18. 표 셀에서도 누름틀 바로 앞 IME 조합은 누름틀 밖에 남는다', () => {
  const wasm = makeWasm({ cell: 'abcde', cellRanges: [['field', 2, 4]] });
  makeHandler(wasm, cellPos(1)).compose('ㅎ', '하');
  assert.equal(wasm.doc.cell, 'a하cde');
  assert.equal(rangesOf(wasm, 'cellRanges'), 'field 2-4', '하가 누름틀 안으로 들어가면 안 된다');
});

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('OVERWRITE_MODE_OK');

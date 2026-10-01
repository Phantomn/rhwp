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
 */
function makeWasm({ body = '', cell = '', objects = [], field = null } = {}) {
  const doc = { body, cell };
  const splice = (key, off, del, ins) => {
    const chars = [...doc[key]];
    chars.splice(off, del, ...ins);
    doc[key] = chars.join('');
  };
  const cellResult = (off) => ({ ok: true, charOffset: off, paginationDeferred: false, cellFlowChanged: false });
  return {
    doc,
    getTextRange: (_s, _p, off, n) => [...doc.body].slice(off, off + n).join(''),
    getTextInCell: (_s, _pp, _ci, _cei, _cpi, off, n) => [...doc.cell].slice(off, off + n).join(''),
    insertText: (_s, _p, off, t) => splice('body', off, 0, [...t]),
    deleteText: (_s, _p, off, n) => splice('body', off, n, []),
    replaceBodyTextLocal: (_s, _p, off, del, t) => {
      splice('body', off, del, [...t]);
      return { ok: true, charOffset: off + [...t].length, documentPaginationPending: false, flowChanged: false };
    },
    insertTextInCell: (_s, _pp, _ci, _cei, _cpi, off, t) => splice('cell', off, 0, [...t]),
    deleteTextInCell: (_s, _pp, _ci, _cei, _cpi, off, n) => splice('cell', off, n, []),
    insertTextInCellDeferredPagination: (_s, _pp, _ci, _cei, _cpi, off, t) => {
      splice('cell', off, 0, [...t]);
      return cellResult(off + [...t].length);
    },
    replaceTextInCellDeferredPagination: (_s, _pp, _ci, _cei, _cpi, off, del, t) => {
      splice('cell', off, del, [...t]);
      return cellResult(off + [...t].length);
    },
    textToLogicalOffset: (_s, _p, off) => off + objects.filter((p) => p < off).length,
    getFieldInfoAt: () => field ?? { inField: false },
    snapshotCapacity: () => 100,
  };
}

const bodyPos = (charOffset) => ({ sectionIndex: 0, paragraphIndex: 0, charOffset });
const cellPos = (charOffset) => ({
  sectionIndex: 0, paragraphIndex: 0, charOffset,
  parentParaIndex: 0, controlIndex: 0, cellIndex: 0, cellParaIndex: 0,
});

/** InputHandler 가 이 경로에서 실제로 쓰는 필드·메서드만 채운 mock this. */
function makeHandler(wasm, start, { insertMode = false } = {}) {
  let position = start;
  const history = new CommandHistory();
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
    canInsertTextInFormMode: () => true,
    canDeleteTextInFormMode: () => true,
    resetRawTextMutationEffects() {},
    consumeRawTextMutationBeforeCursor: () => false,
    afterTextInputEdit() {},
    afterEdit() {},
    updateCaret() {},
    executeOperation(desc) {
      if (desc.kind === 'command') position = history.execute(desc.command, wasm);
      else history.recordWithoutExecute(desc.command, wasm);
    },
    undo() { position = history.undo(wasm); },
    redo() { position = history.redo(wasm); },
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
  return h;
}

// ── 1. 이슈 재현: abcd, Home, 수정 모드, XY → XYcd. 한 번 되돌리면 abcd ──────────
{
  const wasm = makeWasm({ body: 'abcd' });
  const h = makeHandler(wasm, bodyPos(0));
  h.type('XY');
  assert.equal(wasm.doc.body, 'XYcd', '수정 모드는 캐럿 뒤 글자를 입력한 만큼 덮어써야 한다');
  assert.equal(h.cursor.getPosition().charOffset, 2, '캐럿은 입력 글자 뒤에 있어야 한다');
  h.undo();
  assert.equal(wasm.doc.body, 'abcd', '연속 입력은 한 번의 되돌리기로 원문이 돼야 한다');
  assert.equal(h.history.canUndo(), false, '되돌릴 편집이 하나만 기록돼야 한다');
  h.redo();
  assert.equal(wasm.doc.body, 'XYcd', '다시 실행은 같은 덮어쓰기를 재현해야 한다');
}

// ── 2. 삽입 모드는 그대로 끼워 넣는다 ──────────────────────────────────────
{
  const wasm = makeWasm({ body: 'abcd' });
  makeHandler(wasm, bodyPos(0), { insertMode: true }).type('XY');
  assert.equal(wasm.doc.body, 'XYabcd');
}

// ── 3. 문단 끝에서는 덮을 글자가 없어 삽입한다 ─────────────────────────────────
{
  const wasm = makeWasm({ body: 'abc' });
  const h = makeHandler(wasm, bodyPos(2));
  h.type('XY');
  assert.equal(wasm.doc.body, 'abXY', 'c 를 덮은 뒤 문단 끝에서는 삽입해야 한다');
  h.undo();
  assert.equal(wasm.doc.body, 'abc');
}

// ── 4. 탭·개체·누름틀 끝은 덮어쓰지 않는다 ─────────────────────────────────────
{
  const wasm = makeWasm({ body: 'a\tb' });
  makeHandler(wasm, bodyPos(1)).type('X');
  assert.equal(wasm.doc.body, 'aX\tb', '탭은 지우지 않고 그 앞에 삽입해야 한다');
}
{
  // a [개체] b — 개체는 텍스트 위치 1, 캐럿은 개체 앞
  const wasm = makeWasm({ body: 'ab', objects: [1] });
  makeHandler(wasm, bodyPos(1)).type('X');
  assert.equal(wasm.doc.body, 'aXb', '개체 너머의 b 를 덮어쓰면 안 된다');
}
{
  // 빈 누름틀(1..1) 안에서 입력 — 필드 밖 b 를 지우면 안 된다
  const field = { inField: true, fieldType: 'clickhere', startCharIdx: 1, endCharIdx: 1 };
  const wasm = makeWasm({ body: 'ab', field });
  makeHandler(wasm, bodyPos(1)).type('X');
  assert.equal(wasm.doc.body, 'aXb', '누름틀 끝 너머 글자를 덮어쓰면 안 된다');
}

// ── 5. 표 셀도 같은 입력 경로로 덮어쓴다 ─────────────────────────────────────
{
  const wasm = makeWasm({ cell: 'abcd' });
  const h = makeHandler(wasm, cellPos(1));
  h.type('XY');
  assert.equal(wasm.doc.cell, 'aXYd');
  h.undo();
  assert.equal(wasm.doc.cell, 'abcd');
}

// ── 6. IME 조합: 음절마다 한 글자를 한 번만 덮고, 한 번의 되돌리기로 돌아간다 ──────────
{
  const wasm = makeWasm({ body: 'abcd' });
  const h = makeHandler(wasm, bodyPos(0));
  h.compose('ㅎ', '하', '한');
  assert.equal(wasm.doc.body, '한bcd', '조합 갱신이 거듭돼도 덮는 글자는 a 하나뿐이어야 한다');
  h.compose('ㄱ', '그', '글');
  assert.equal(wasm.doc.body, '한글cd');
  h.undo();
  assert.equal(wasm.doc.body, 'abcd', '조합 입력도 일반 입력처럼 한 번에 되돌아가야 한다');
}

// ── 7. 조합 취소: 덮었던 글자를 되살리고 아무것도 기록하지 않는다 ─────────────────
{
  const wasm = makeWasm({ body: 'abcd' });
  const h = makeHandler(wasm, bodyPos(0));
  h.compose('ㅎ', '');
  assert.equal(wasm.doc.body, 'abcd', '조합을 지우면 덮었던 a 가 돌아와야 한다');
  assert.equal(h.cursor.getPosition().charOffset, 0);
  assert.equal(h.history.canUndo(), false, '취소된 조합은 기록하지 않는다');
}

console.log('OVERWRITE_MODE_OK');

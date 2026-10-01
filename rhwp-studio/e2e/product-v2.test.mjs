/** #7353: normal Studio, real keyboard/history, file input and save/reopen. */
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { runTest, loadApp, createNewDocument, assert, screenshot, setTestCase } from './helpers.mjs';

const out = resolve('../output/7353/wasm-product/w3/studio');
mkdirSync(out, { recursive: true });
const pause = page => page.evaluate(() => new Promise(ok => setTimeout(ok, 300)));
async function historyKey(page, key) {
  await page.keyboard.down('Control');
  await page.keyboard.press(key);
  await page.keyboard.up('Control');
  await pause(page);
}
async function assertPaintedPages(page) {
  const state = await page.evaluate(() => ({
    model: window.__wasm.doc.pageCount(),
    view: window.__canvasView.pages.length,
    painted: [...document.querySelectorAll('#scroll-container canvas.document-page-canvas')]
      .filter(c => c.width > 0 && c.height > 0 && c.dataset.rhwpRenderedZoom).length,
  }));
  assert(state.model > 0 && state.view === state.model && state.painted > 0,
    `Studio retains painted pages: ${JSON.stringify(state)}`);
}
async function upload(page, bytes, name) {
  await page.evaluate(({ bytes, name }) => {
    const input = document.querySelector('#file-input');
    const transfer = new DataTransfer();
    transfer.items.add(new File([new Uint8Array(bytes)], name));
    input.files = transfer.files;
    input.dataset.skipUnsavedGuard = 'true';
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, { bytes: [...bytes], name });
  await page.waitForFunction(name => window.__wasm?.fileName === name, { timeout: 15000 }, name);
  await page.waitForSelector('#scroll-container canvas');
  await pause(page);
}

runTest('#7353 일반 Studio V2 제품 여정', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  page.on('console', message => { if (message.type() === 'error') console.error('browser:', message.text()); });
  // Dismiss only this page's first-run skin dialog, not existing user tabs.
  await page.evaluate(() => {
    if (!document.querySelector('.skin-onboarding-card')) return;
    const button = [...document.querySelectorAll('button.dialog-btn-primary')].find(el => el.offsetParent !== null);
    button?.click();
  });
  const evidence = [];
  for (const [kind, fixture] of [
    ['body', '../tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp'],
    ['cell', '../tests/fixtures/issue7353_host_owner_review/split-saved.hwp'],
  ]) {
    if (process.argv.includes('--cell-only') && kind !== 'cell') continue;
    setTestCase(`${kind}: upload -> keyboard -> history -> save/reopen`);
    await upload(page, readFileSync(fixture), `w3-${kind}.hwp`);
    assert(await page.evaluate(() => window.__wasm.doc.getTypesetter()) === 'v2', 'normal file input selects V2');
    assert(await page.$('#v2-page-host') === null, 'not the read-only preview');
    const before = await page.evaluate(() => window.__wasm.doc.pageCount());
    await page.evaluate(kind => {
      const handler = window.__inputHandler;
      handler.cursor.moveTo(kind === 'cell' ? {
        sectionIndex: 0, paragraphIndex: 0, charOffset: 0,
        parentParaIndex: 1, controlIndex: 0, cellIndex: 0, cellParaIndex: 0,
      } : { sectionIndex: 0, paragraphIndex: 0, charOffset: 0 });
      handler.focus();
    }, kind);
    const read = () => page.evaluate(kind => {
      const doc = window.__wasm.doc;
      return kind === 'cell' ? doc.getTextInCell(0, 1, 0, 0, 0, 0, 100) : doc.getTextRange(0, 0, 0, 100);
    }, kind);
    const original = await read();
    await page.keyboard.type('EDIT ');
    await pause(page);
    assert((await read()).startsWith('EDIT '), 'real keyboard edits the targeted paragraph');
    await assertPaintedPages(page);
    // One typing run may contain several undo entries. Undo until the exact
    // prior text, recording the actual count rather than assuming coalescing.
    let undos = 0;
    while ((await read()) !== original && undos < 8) {
      await historyKey(page, 'KeyZ'); undos++;
    }
    assert((await read()) === original, 'Ctrl+Z restores exact text');
    for (let i = 0; i < undos; i++) await historyKey(page, 'KeyY');
    assert((await read()).startsWith('EDIT '), 'Ctrl+Y restores edited text');
    await assertPaintedPages(page);
    await page.keyboard.down('Shift');
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.up('Shift');
    await pause(page);
    const selection = await page.evaluate(() => ({
      active: window.__inputHandler.cursor.hasSelection(),
      highlights: [...document.querySelectorAll('.selection-layer > div')]
        .filter(el => el.getBoundingClientRect().width > 0 && el.getBoundingClientRect().height > 0)
        .map(el => ({ left: el.style.left, top: el.style.top, width: el.style.width, height: el.style.height })),
    }));
    assert(selection.active && selection.highlights.length > 0, 'Shift+Left paints a text selection');
    await screenshot(page, `product-v2-${kind}-edited`);
    await page.screenshot({ path: resolve(out, `${kind}-edited.png`) });
    const state = await page.evaluate(kind => {
      const w = window.__wasm;
      const doc = w.doc;
      const target = kind === 'cell' ? 1 : 0;
      // Export through the Studio bridge (its portable-metrics transaction),
      // then feed the saved bytes through the actual file-input handler.
      const bytes = Array.from(w.exportHwp());
      return { engine: doc.getTypesetter(), pages: doc.pageCount(), bytes, tree: doc.getPageRenderTree(target), cursor: window.__inputHandler.cursor.getRect() };
    }, kind);
    writeFileSync(resolve(out, `${kind}-edited.hwp`), new Uint8Array(state.bytes));
    await upload(page, state.bytes, `w3-${kind}-reopened.hwp`);
    assert((await read()).startsWith('EDIT '), 'saved file reopens through normal Studio');
    evidence.push({ kind, before, ...state, bytes: state.bytes.length, undos, selection });
  }
  setTestCase('new document default');
  await createNewDocument(page);
  assert(await page.evaluate(() => window.__wasm.doc.getTypesetter()) === 'v2', 'new document keeps V2');
  await page.evaluate(() => window.__inputHandler.focus());
  await page.keyboard.type('NEW'); await pause(page);
  assert((await page.evaluate(() => window.__wasm.doc.getTextRange(0, 0, 0, 50))).includes('NEW'), 'new document accepts keyboard input');
  await screenshot(page, 'product-v2-new');
  setTestCase('explicit Legacy reopening');
  // This is our disposable test document. Preserve its bytes before the
  // deliberate navigation, then handle only that tab's beforeunload prompt.
  writeFileSync(resolve(out, 'new-edited.hwp'), new Uint8Array(
    await page.evaluate(() => Array.from(window.__wasm.exportHwp())),
  ));
  const leaveTestDocument = async dialog => {
    assert(dialog.type() === 'beforeunload', `expected navigation guard, got ${dialog.type()}`);
    if (dialog.type() === 'beforeunload') await dialog.accept();
    else await dialog.dismiss();
  };
  page.on('dialog', leaveTestDocument);
  try { await loadApp(page, '?typeset=legacy'); }
  finally { page.off('dialog', leaveTestDocument); }
  await upload(page, readFileSync('../tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp'), 'legacy-control.hwp');
  assert(await page.evaluate(() => window.__wasm.doc.getTypesetter()) === 'legacy', 'explicit Legacy remains available');
  writeFileSync(resolve(out, 'result.json'), JSON.stringify({ evidence, errors }, null, 2));
  assert(errors.length === 0, `uncaught browser errors: ${errors.join('; ')}`);
});

#!/usr/bin/env node
// Actual HwpDocument product output/editing, not HostedSectionV2 SVG preview.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import http from 'node:http';
import { findChrome, parseWebfontRules, selectWebfontRules, buildWebfontCss } from './rasterize-svg-webfonts.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
function option(name) {
  const i = process.argv.indexOf(name);
  assert.ok(i >= 0 && process.argv[i + 1], `Missing ${name}`);
  return resolve(process.argv[i + 1]);
}
const pkg = option('--pkg'), input = option('--input'), out = option('--out');
// Browser evaluate uses JSON transport, which canonicalizes -0 to 0. Apply
// that same transport to Native evidence; do not round nonzero geometry.
const native = JSON.parse(JSON.stringify(JSON.parse(readFileSync(join(out, 'native.json'), 'utf8'))));
const require = createRequire(join(root, 'rhwp-studio/package.json'));
const files = new Map([
  ['/', ['text/html', Buffer.from('<!doctype html><html><head></head><body style="margin:0"></body></html>')]],
  ['/rhwp.js', ['text/javascript', readFileSync(join(pkg, 'rhwp.js'))]],
  ['/rhwp_bg.wasm', ['application/wasm', readFileSync(join(pkg, 'rhwp_bg.wasm'))]],
  ['/input', ['application/octet-stream', readFileSync(input)]],
]);
const server = http.createServer((request, response) => {
  const file = files.get(request.url);
  if (!file) { response.writeHead(404).end(); return; }
  response.setHeader('Content-Type', file[0]); response.end(file[1]);
});
await new Promise(ok => server.listen(0, '127.0.0.1', ok));
let browser;
try {
  browser = await require('puppeteer-core').launch({ executablePath: findChrome(process.env.VISUAL_SWEEP_CHROME), headless: true });
  const page = await browser.newPage();
  await page.goto(`http://127.0.0.1:${server.address().port}`);
  const result = await page.evaluate(async defaultV2 => {
    const module = await import('/rhwp.js');
    await module.default({ module_or_path: '/rhwp_bg.wasm' });
    const input = new Uint8Array(await (await fetch('/input')).arrayBuffer());
    window.documentV2 = module.HwpDocument.openWithTypesetter(input, 'v2');
    const legacy = module.HwpDocument.openWithTypesetter(input, 'legacy');
    const defaultDoc = new module.HwpDocument(input);
    const modes = [window.documentV2.getTypesetter(), legacy.getTypesetter(), defaultDoc.getTypesetter()];
    legacy.free(); defaultDoc.free();
    if (defaultV2) {
      const check = (ok, message) => { if (!ok) throw new Error(message); };
      const empty = module.HwpDocument.createEmpty();
      check(empty.getTypesetter() === 'v2' && empty.pageCount() === 1, 'default empty V2');
      empty.insertText(0, 0, 0, 'NEW');
      const encrypted = empty.exportHwpWithPassword('w3-test');
      const reopened = module.HwpDocument.openWithPassword(encrypted, 'w3-test');
      check(reopened.getTypesetter() === 'v2' && reopened.getTextRange(0, 0, 0, 3) === 'NEW', 'password open defaults to V2');
      const legacyPassword = module.HwpDocument.openWithPasswordAndTypesetter(encrypted, 'w3-test', 'legacy');
      check(legacyPassword.getTypesetter() === 'legacy', 'explicit password Legacy');
      const legacyEmpty = module.HwpDocument.createEmptyWithTypesetter('legacy');
      check(legacyEmpty.getTypesetter() === 'legacy', 'explicit empty Legacy');
      for (const doc of [empty, reopened, legacyPassword, legacyEmpty]) doc.free();
    }
    let rejectsUnknown = false;
    try { module.HwpDocument.openWithTypesetter(input, 'unknown'); } catch { rejectsUnknown = true; }
    const pages = [];
    for (let i = 0; i < window.documentV2.pageCount(); i++) {
      pages.push({ tree: JSON.parse(window.documentV2.getPageRenderTree(i)), svg: window.documentV2.renderPageSvg(i), layer: JSON.parse(window.documentV2.getPageLayerTree(i)) });
    }
    return { modes, rejectsUnknown, pages };
  }, process.argv.includes('--default-v2'));
  assert.deepEqual(result.modes, ['v2', 'legacy', process.argv.includes('--default-v2') ? 'v2' : 'legacy']);
  assert.equal(result.rejectsUnknown, true);
  assert.equal(result.pages.length, native.length);
  const rules = parseWebfontRules(readFileSync(join(root, 'rhwp-studio/src/core/generated/font-rule-projections/webfont-supply.ts'), 'utf8'));
  mkdirSync(join(out, 'browser'), { recursive: true });
  for (const [i, actual] of result.pages.entries()) {
    assert.deepEqual(actual.tree, native[i].tree, `Native/WASM product tree page ${i + 1}`);
    assert.deepEqual(actual.layer, native[i].layer, `Native/WASM product layer page ${i + 1}`);
    assert.equal(actual.svg, native[i].svg, `Native/WASM product SVG page ${i + 1}`);
    writeFileSync(join(out, `wasm-${i + 1}.json`), JSON.stringify(actual, null, 2));
    writeFileSync(join(out, `wasm-${i + 1}.svg`), actual.svg);
    const css = buildWebfontCss(root, selectWebfontRules(actual.svg, rules)).replace(/file:\/\/[^"\)]+/g, url => {
      const route = `/font-${files.size}`;
      files.set(route, ['font/woff2', readFileSync(fileURLToPath(url))]);
      return route;
    });
    await page.addStyleTag({ content: css });
    const size = await page.evaluate(async ({ index, svg }) => {
      // Load the very same font supply used by the SVG rasterizer, then paint
      // the actual Canvas product API. Do not rasterize SVG as Canvas evidence.
      const holder = document.createElement('div'); holder.innerHTML = svg; document.body.append(holder);
      await document.fonts.ready;
      const source = holder.querySelector('svg');
      const width = Math.ceil(parseFloat(source.getAttribute('width'))), height = Math.ceil(parseFloat(source.getAttribute('height')));
      holder.remove();
      document.body.replaceChildren();
      const canvas = document.createElement('canvas'); document.body.append(canvas);
      window.documentV2.renderPageToCanvas(index, canvas, 1);
      return { width, height };
    }, { index: i, svg: actual.svg });
    await page.setViewport(size);
    await page.screenshot({ path: join(out, 'browser', `canvas-${i + 1}.png`) });
  }
  let editing;
  if (process.argv.includes('--edit') || process.argv.includes('--edit-cell')) {
    editing = await page.evaluate(async table => {
      const { HwpDocument } = await import('/rhwp.js');
      const doc = window.documentV2;
      const check = (ok, message) => { if (!ok) throw new Error(message); };
      // SVG can emit separate tspans for glyphs; inspect displayed text rather
      // than requiring the serialization to contain one contiguous string.
      const svgText = svg => Array.from(new DOMParser().parseFromString(svg, 'image/svg+xml').querySelectorAll('text'), node => node.textContent).join('');
      check(!table || Array.from({ length: doc.pageCount() }, (_, i) => doc.renderPageSvg(i)).some(svg => svgText(svg).replace(/\s/g, '').includes('ROW01')), 'cell journey requires the saved split-table fixture');
      const target = table ? 1 : 0;
      const before = doc.renderPageSvg(target);
      const undo = doc.saveSnapshot();
      const editResult = table ? doc.insertTextInCell(0, 1, 0, 0, 0, 0, 'EDIT ') : doc.insertText(0, 0, 0, 'EDIT ');
      const edited = doc.renderPageSvg(target);
      check(svgText(edited).includes('EDIT'), `edit must reach product output: ${JSON.stringify({ table, editResult, text: svgText(edited) })}`);
      const cursor = JSON.parse(table ? doc.getCursorRectInCell(0, 1, 0, 0, 0, 0) : doc.getCursorRect(0, 0, 0));
      const hit = JSON.parse(doc.hitTest(cursor.pageIndex, cursor.x + 1, cursor.y + 1));
      check(cursor.pageIndex === target, 'caret must address the painted fragment');
      check(!table || cursor.height === 12 && cursor.x === 25, '9pt cell glyph and 5px padding');
      check(!table || hit.parentParaIndex === 1 && hit.cellIndex === 0, 'hit-test must preserve cell ownership');
      const selection = JSON.parse(table ? doc.getSelectionRectsInCell(0, 1, 0, 0, 0, 0, 0, 4) : doc.getSelectionRects(0, 0, 0, 0, 4));
      check(selection.length > 0 && selection.every(r => r.pageIndex === target && r.width > 0 && r.height > 0), 'selection must cover the edited run');
      const canvas = document.querySelector('canvas');
      doc.renderPageToCanvas(target, canvas, 1);
      const redo = doc.saveSnapshot();
      doc.restoreSnapshot(undo);
      check(doc.renderPageSvg(target) === before, 'undo must restore exact geometry and paint');
      doc.restoreSnapshot(redo);
      check(doc.renderPageSvg(target) === edited, 'redo must restore exact geometry and paint');
      const reopened = HwpDocument.openWithTypesetter(doc.exportHwp(), doc.getTypesetter());
      check(reopened.getTypesetter() === 'v2' && svgText(reopened.renderPageSvg(target)).includes('EDIT'), 'save/reopen must retain edited V2 content');
      reopened.free();
      let growth, paragraphSplit;
      if (table) {
        const pagesBefore = doc.pageCount();
        const offset = doc.getCellParagraphLength(0, 1, 0, 0, 0);
        const added = Array.from({ length: 30 }, (_, i) => `\nADDED${String(i).padStart(2, '0')}`).join('');
        doc.insertTextInCell(0, 1, 0, 0, 0, offset, added);
        const pages = Array.from({ length: doc.pageCount() }, (_, i) => doc.renderPageSvg(i));
        check(pages.length > pagesBefore, 'cell growth must paginate');
        const markerPage = pages.findIndex(svg => svgText(svg).includes('ADDED29'));
        check(markerPage >= 0, 'last added line must be painted');
        const end = doc.getCellParagraphLength(0, 1, 0, 0, 0);
        const caret = JSON.parse(doc.getCursorRectInCell(0, 1, 0, 0, 0, end - 1));
        const rects = JSON.parse(doc.getSelectionRectsInCell(0, 1, 0, 0, 0, end - 7, 0, end));
        check(caret.pageIndex === markerPage && caret.height === 12, 'grown-cell caret must address the last painted line');
        check(rects.length > 0 && rects.every(r => r.pageIndex === markerPage && r.width > 0 && r.height > 0), 'grown-cell selection must address the last painted line');
        growth = { pagesBefore, pagesAfter: pages.length, markerPage, caret, selection: rects };
        doc.restoreSnapshot(redo);
        check(doc.renderPageSvg(target) === edited, 'growth undo must restore edited page');
        const countBefore = doc.getCellParagraphCount(0, 1, 0, 0);
        doc.splitParagraphInCell(0, 1, 0, 0, 0, 5, undefined);
        check(doc.getCellParagraphCount(0, 1, 0, 0) === countBefore + 1, 'Enter must create a cell paragraph');
        const splitCaret = JSON.parse(doc.getCursorRectInCell(0, 1, 0, 0, 1, 0));
        const splitHit = JSON.parse(doc.hitTest(splitCaret.pageIndex, splitCaret.x + 1, splitCaret.y + 1));
        check(splitCaret.y > cursor.y && splitHit.cellParaIndex === 1, 'Enter caret/hit must use the new paragraph line');
        const splitSelection = JSON.parse(doc.getSelectionRectsInCell(0, 1, 0, 0, 1, 0, 1, 3));
        check(splitSelection.length > 0 && splitSelection.every(r => r.pageIndex === splitCaret.pageIndex && r.y > selection[0].y), 'Enter selection must follow the new line');
        paragraphSplit = { cursor: splitCaret, hit: splitHit, selection: splitSelection };
        doc.restoreSnapshot(redo);
        check(doc.renderPageSvg(target) === edited, 'Enter undo must restore the original cell paragraph');
      }
      return { result: 'PASS', scope: 'product APIs, not Studio history UI', table, target, cursor, hit, selection, growth, paragraphSplit };
    }, process.argv.includes('--edit-cell'));
    await page.screenshot({ path: join(out, 'browser', 'edited-canvas.png') });
    writeFileSync(join(out, 'editing.json'), JSON.stringify(editing, null, 2));
  }
  const sha = data => createHash('sha256').update(data).digest('hex');
  writeFileSync(join(out, 'browser-manifest.json'), JSON.stringify({
    result: 'PASS', route: 'HwpDocument.openWithTypesetter(v2) -> product Canvas/SVG',
    browser: await browser.version(), input_sha256: sha(readFileSync(input)),
    wasm_sha256: sha(files.get('/rhwp_bg.wasm')[1]), js_sha256: sha(files.get('/rhwp.js')[1]), pages: result.pages.length,
    default_v2: process.argv.includes('--default-v2'),
    scope: process.argv.includes('--default-v2')
      ? 'W3 default constructor/empty/password APIs and product output; Studio UI evidence is separate'
      : editing ? 'W2 product editing APIs; Studio default/history UI remain W3' : 'W1 reading/output only; editing and default switch are W2/W3', editing,
  }, null, 2));
  console.log(`PASS: ${result.pages.length} product pages; Native/WASM SVG parity; actual Canvas captures`);
} finally {
  if (browser) await browser.close();
  await new Promise(ok => server.close(ok));
}

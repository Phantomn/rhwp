#!/usr/bin/env node
// W1 actual HwpDocument product route, not HostedSectionV2 SVG preview.
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
  const result = await page.evaluate(async () => {
    const module = await import('/rhwp.js');
    await module.default({ module_or_path: '/rhwp_bg.wasm' });
    const input = new Uint8Array(await (await fetch('/input')).arrayBuffer());
    window.documentV2 = module.HwpDocument.openWithTypesetter(input, 'v2');
    const legacy = module.HwpDocument.openWithTypesetter(input, 'legacy');
    const defaultDoc = new module.HwpDocument(input);
    const modes = [window.documentV2.getTypesetter(), legacy.getTypesetter(), defaultDoc.getTypesetter()];
    legacy.free(); defaultDoc.free();
    let rejectsUnknown = false;
    try { module.HwpDocument.openWithTypesetter(input, 'unknown'); } catch { rejectsUnknown = true; }
    const pages = [];
    for (let i = 0; i < window.documentV2.pageCount(); i++) {
      pages.push({ tree: JSON.parse(window.documentV2.getPageRenderTree(i)), svg: window.documentV2.renderPageSvg(i), layer: JSON.parse(window.documentV2.getPageLayerTree(i)) });
    }
    return { modes, rejectsUnknown, pages };
  });
  assert.deepEqual(result.modes, ['v2', 'legacy', 'legacy']);
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
  const sha = data => createHash('sha256').update(data).digest('hex');
  writeFileSync(join(out, 'browser-manifest.json'), JSON.stringify({
    result: 'PASS', route: 'HwpDocument.openWithTypesetter(v2) -> product Canvas/SVG',
    browser: await browser.version(), input_sha256: sha(readFileSync(input)),
    wasm_sha256: sha(files.get('/rhwp_bg.wasm')[1]), js_sha256: sha(files.get('/rhwp.js')[1]), pages: result.pages.length,
    scope: 'W1 reading/output only; editing and default switch are W2/W3',
  }, null, 2));
  console.log(`PASS: ${result.pages.length} product pages; Native/WASM SVG parity; actual Canvas captures`);
} finally {
  if (browser) await browser.close();
  await new Promise(ok => server.close(ok));
}

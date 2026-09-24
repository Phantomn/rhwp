#!/usr/bin/env node
// Explicit fresh-WASM contract runner for issue_7353_table_v2_export fixtures.
// This compares two execution backends, NOT Hancom fidelity or a document engine.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import http from 'node:http';
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { findChrome } from './rasterize-svg-webfonts.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
function option(name, fallback) {
  const i = process.argv.indexOf(name);
  if (i < 0) {
    if (fallback) return fallback;
    throw Error(`Missing ${name}`);
  }
  if (!process.argv[i + 1] || process.argv[i + 1].startsWith('--')) throw Error(`Missing value: ${name}`);
  return resolve(process.argv[i + 1]);
}
const hash = bytes => createHash('sha256').update(bytes).digest('hex');

async function main() {
  const pkg = option('--pkg'), fixtures = option('--fixtures'), out = option('--out');
  const require = createRequire(join(option('--dependencies-root', root), 'rhwp-studio/package.json'));
  const names = ['merged', 'nested', 'partial', 'stored', 'rowspan'];
  const js = readFileSync(join(pkg, 'rhwp.js')), wasm = readFileSync(join(pkg, 'rhwp_bg.wasm'));
  const files = new Map([
    ['/rhwp.js', ['application/javascript', js]],
    ['/rhwp_bg.wasm', ['application/wasm', wasm]],
    ...names.map(name => [`/${name}`, ['application/octet-stream', readFileSync(join(fixtures, `${name}.hwpx`))]]),
  ]);
  const configs = Object.fromEntries(names.map(name => [name, JSON.parse(readFileSync(join(fixtures, `${name}.options.json`)))]));
  const server = http.createServer((req, res) => {
    if (req.url === '/') { res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><meta charset="utf-8">'); return; }
    const file = files.get(req.url);
    if (!file) { res.writeHead(404).end(); return; }
    res.setHeader('Content-Type', file[0]); res.end(file[1]);
  });
  await new Promise((ok, fail) => { server.once('error', fail); server.listen(0, '127.0.0.1', ok); });
  let browser;
  try {
    browser = await require('puppeteer-core').launch({ executablePath: findChrome(process.env.VISUAL_SWEEP_CHROME), headless: true });
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${server.address().port}`);
    const result = await page.evaluate(async configs => {
      const m = await import('/rhwp.js');
      await m.default({ module_or_path: '/rhwp_bg.wasm' });
      const check = (ok, text) => { if (!ok) throw Error(text); };
      const throws = (fn, text) => { let failed = false; try { fn(); } catch (e) { failed = String(e).includes(text); } check(failed, `Expected rejection: ${text}`); };
      const input = {};
      for (const name of Object.keys(configs)) input[name] = new Uint8Array(await (await fetch(`/${name}`)).arrayBuffer());
      const open = (name, config = configs[name]) => new m.TableV2Preview(input[name], JSON.stringify(config));
      check(typeof m.TableV2Preview === 'function', 'Missing experimental export');
      const pages = {};
      for (const name of ['merged', 'nested', 'partial']) {
        const a = open(name), b = open(name);
        try {
          pages[name] = [];
          check(a.emittedPages() === 0 && b.emittedPages() === 0, 'initial count');
          const source = input[name].slice();
          input[name].fill(0); // Both sessions must own snapshots, not borrowed JS bytes.
          input[name] = source;
          for (let i = 0; i < 10; i++) {
            const value = a.nextPage();
            if (value === undefined) break;
            pages[name].push(JSON.parse(value));
          }
          check(a.nextPage() === undefined && a.nextPage() === undefined, 'stable end');
          check(a.emittedPages() === pages[name].length, 'emitted count');
          check(b.emittedPages() === 0, 'independent cursor');
          check(JSON.stringify(JSON.parse(b.nextPage())) === JSON.stringify(pages[name][0]), 'independent first page');
        } finally { a.free(); b.free(); }
      }
      for (const name of ['stored', 'rowspan']) throws(() => open(name), 'Unsupported');
      throws(() => new m.TableV2Preview(input.merged, '{}'), 'Options');
      throws(() => new m.TableV2Preview(new Uint8Array([0]), JSON.stringify(configs.merged)), 'Parse');
      for (const change of [c => { c.engine = 'legacy'; }, c => { c.pages.body.widht = 1; }, c => { c.max_pages = 4294967296; }, c => { c.selection.control = -1; }]) {
        const c = structuredClone(configs.merged); change(c); throws(() => open('merged', c), 'Options');
      }
      for (const change of [c => { c.dpi = 0; }, c => { c.max_pages = 0; }, c => { c.selection.control = 999; }, c => { c.pages.body.height = 500; }]) {
        const c = structuredClone(configs.merged); change(c); throws(() => open('merged', c), 'Preview');
      }
      const limited = open('merged', { ...configs.merged, max_pages: 1 });
      try {
        check(limited.nextPage() !== undefined, 'first page before limit');
        for (let i = 0; i < 2; i++) { throws(() => limited.nextPage(), 'PageLimit'); check(limited.emittedPages() === 1, 'limit rollback'); }
      } finally { limited.free(); }
      const narrow = structuredClone(configs.merged); narrow.pages.body.height = 18;
      const blocked = open('merged', narrow);
      try {
        for (let i = 0; i < 2; i++) { throws(() => blocked.nextPage(), 'DoesNotFit'); check(blocked.emittedPages() === 0, 'fit rollback'); }
      } finally { blocked.free(); }
      return { pages, version: m.version() };
    }, configs);
    mkdirSync(out, { recursive: true });
    const collect = (node, kind) => [ ...(node.node_type[kind] ? [node] : []), ...node.children.flatMap(n => collect(n, kind)) ];
    // Expectations independent of Native output: 18px lines, 36px body, repeated title.
    const expected = { merged: [['title', 'A'], ['title', 'B']], nested: [['title', 'A'], ['title', 'B'], ['host', 'after']], partial: [['title', 'A'], ['title', 'B']] };
    const raster = await browser.newPage();
    const artifacts = [];
    for (const [name, pages] of Object.entries(result.pages)) {
      const native = JSON.parse(readFileSync(join(fixtures, `${name}.native.json`)));
      writeFileSync(join(out, `${name}.wasm.json`), JSON.stringify(pages, null, 2));
      assert.equal(pages.length, expected[name].length);
      for (const [index, output] of pages.entries()) {
        assert.equal(output.engine, 'table_v2'); assert.equal(output.scope, 'selected_table');
        assert.equal(output.page_index, index + (name === 'partial' ? 1 : 0));
        const rootNode = output.render_tree.root;
        assert.deepEqual(collect(rootNode, 'TextRun').map(n => n.node_type.TextRun.text), expected[name][index]);
        const lines = collect(rootNode, 'TextLine');
        assert.deepEqual(lines.map(n => n.bbox.y), [30, 48]);
        assert.ok(lines.every(n => n.bbox.x === 20 && n.bbox.y + n.bbox.height <= 66));
        for (const c of collect(rootNode, 'TableCell')) {
          assert.equal(c.node_type.TableCell.col_span, 2);
          assert.equal(c.bbox.x, 20); assert.equal(c.bbox.width, 200);
          assert.ok(c.bbox.y >= 30 && c.bbox.y + c.bbox.height <= 66);
        }
        const stem = `${name}-${index}`;
        writeFileSync(join(out, `${stem}.svg`), output.svg);
        // Preserve evidence even if a backend comparison fails below.
        await raster.setViewport({ width: 400, height: 400, deviceScaleFactor: 1 });
        for (const [backend, svg] of [['native', native[index].svg], ['wasm', output.svg]]) {
          await raster.setContent(`<body style="margin:0;background:white">${svg}</body>`);
          await raster.evaluate(() => document.fonts.ready);
          await raster.screenshot({ path: join(out, `${stem}.${backend}.png`) });
        }
        // Same Chrome raster environment; RGB overlay reference is Native, NOT Hancom.
        const imgs = ['native', 'wasm'].map(b => `data:image/png;base64,${readFileSync(join(out, `${stem}.${b}.png`)).toString('base64')}`);
        const overlay = await raster.evaluate(async imgs => {
          const canvases = await Promise.all(imgs.map(async url => {
            const image = new Image(); image.src = url; await image.decode();
            const c = document.createElement('canvas'); c.width = 400; c.height = 400;
            c.getContext('2d').drawImage(image, 0, 0); return c;
          }));
          const context = canvases[0].getContext('2d'), a = context.getImageData(0, 0, 400, 400), b = canvases[1].getContext('2d').getImageData(0, 0, 400, 400);
          for (let i = 0; i < a.data.length; i += 4) {
            const r = Math.round((a.data[i] + a.data[i + 1] + a.data[i + 2]) / 3);
            const g = Math.round((b.data[i] + b.data[i + 1] + b.data[i + 2]) / 3);
            a.data[i] = r; a.data[i + 1] = g; a.data[i + 2] = g; a.data[i + 3] = 255;
          }
          context.putImageData(a, 0, 0); return canvases[0].toDataURL();
        }, imgs);
        writeFileSync(join(out, `${stem}.overlay.png`), Buffer.from(overlay.split(',')[1], 'base64'));
        await raster.setViewport({ width: 1200, height: 430, deviceScaleFactor: 1 });
        await raster.setContent(`<body style="margin:0;display:flex;background:white;font:16px sans-serif">${[...imgs, overlay].map((url, i) => `<div><div style="height:30px">${['Native', 'Fresh WASM', 'Native/WASM overlay'][i]} ${stem}</div><img src="${url}"></div>`).join('')}</body>`);
        await raster.evaluate(() => Promise.all([...document.images].map(image => image.decode())));
        await raster.screenshot({ path: join(out, `${stem}.review.png`) });
        assert.deepEqual(output.render_tree, native[index].render_tree, `${stem}: exact tree parity`);
        assert.equal(output.svg, native[index].svg, `${stem}: exact SVG parity`);
        artifacts.push({ stem, svg_sha256: hash(output.svg), review_sha256: hash(readFileSync(join(out, `${stem}.review.png`))) });
      }
    }
    const manifest = { version: result.version, browser: await browser.version(), js_sha256: hash(js), wasm_sha256: hash(wasm), inputs: Object.fromEntries(names.map(n => [n, hash(files.get(`/${n}`)[1])])), artifacts, result: 'PASS', oracle: 'synthetic geometry and Native backend; not Hancom' };
    writeFileSync(join(out, 'manifest.json'), JSON.stringify(manifest, null, 2));
    console.log(`PASS: 7 page exports; exact Native/WASM tree+SVG parity; isolation, rejection, rollback, termination. ${manifest.browser}`);
  } finally {
    try {
      if (browser) await browser.close();
    } finally {
      await new Promise(ok => server.close(ok));
    }
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });

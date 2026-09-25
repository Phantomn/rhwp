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

// Source HWP units pass through the common page calculator (e.g.2250HU becomes
//30.000000000000004px). Compare the independent rational geometry within32ULPs;
// do NOT round output or relax the exact Native/WASM tree/SVG comparison below.
const sameCoordinate = (actual, expected) => Number.isFinite(actual)
  && Math.abs(actual-expected) <= 32*Number.EPSILON*Math.max(1,Math.abs(expected));
function assertDocumentGeometry(actual, expected) {
  if (typeof expected === 'number') {
    assert.ok(sameCoordinate(actual,expected),`${actual} != ${expected}`);
  } else if (Array.isArray(expected)) {
    assert.ok(Array.isArray(actual)); assert.equal(actual.length,expected.length);
    expected.forEach((item,i)=>assertDocumentGeometry(actual[i],item));
  } else {
    assert.deepEqual(Object.keys(actual).sort(),Object.keys(expected).sort());
    for (const key of Object.keys(expected)) assertDocumentGeometry(actual[key],expected[key]);
  }
}

// Independent intact-cell oracle:18px lines,6/12px padding and declared row
// minima. These are not derived from Native output or from the WASM algorithm.
function verticalContract(name, page) {
  const box = (x,y,width,height) => ({x,y,width,height});
  const edges = ({x,y,width:w,height:h}) => [[x,y,x,y+h],[x+w,y,x+w,y+h],[x,y,x+w,y],[x,y+h,x+w,y+h]];
  if (name === 'valign-rows' && page === 1) {
    const b=box(20,30,300,36);
    return {xs:[20],ys:[36],cells:[b],tables:[b],edges:edges(b)};
  }
  if (name === 'valign-rows' || name === 'valign-whole') {
    const whole=name === 'valign-whole', bottom=whole ? 156 : 120;
    const cells=[box(20,30,100,90),box(120,30,100,90),box(220,30,100,90)];
    const xs=[20,20,120,120,220,220],ys=[36,54,54,72,72,90];
    if (whole) { cells.push(box(20,120,300,36)); xs.push(20); ys.push(126); }
    const lines=[[20,30,20,bottom],[120,30,120,120],[220,30,220,120],[320,30,320,bottom],[20,30,320,30],[20,120,320,120]];
    if (whole) lines.push([20,156,320,156]);
    return {xs,ys,cells,tables:[box(20,30,300,bottom-30)],edges:lines};
  }
  if (name === 'valign-header') {
    return {xs:[20,20],ys:[45,90],cells:[box(20,30,200,54),box(20,84,200,36)],
      tables:[box(20,30,200,90)],edges:[[20,30,20,120],[220,30,220,120],[20,30,220,30],[20,84,220,84],[20,120,220,120]]};
  }
  if (name === 'valign-nested') {
    const parent=box(20,30,200,108),child=box(90,page===0 ? 48 : 30,100,page===0 ? 90 : 72);
    return {xs:page===0 ? [30,90,90] : [90,30,30],ys:page===0 ? [30,72,90] : [72,102,120],
      cells:[parent,child],tables:[parent,child],edges:[...edges(child),...edges(parent)]};
  }
  assert.equal(name,'valign-parent');
  const parent=box(20,30,200,90),child=box(120,45,100,18);
  return {xs:[120,20,20],ys:[45,63,81],cells:[parent,child],tables:[parent,child],edges:[...edges(child),...edges(parent)]};
}

async function main() {
  const pkg = option('--pkg'), fixtures = option('--fixtures'), out = option('--out');
  const require = createRequire(join(option('--dependencies-root', root), 'rhwp-studio/package.json'));
  const positive = ['merged', 'nested', 'partial'];
  // Opt-in keeps the stage11 command/fixtures valid without silently skipping
  // missing stage12 evidence. Every requested fixture is mandatory.
  const negative = ['stored', 'rowspan'];
  if (process.argv.includes('--split-line-property')) {
    negative.push('separate-cell', 'separate-table', 'separate-child');
  }
  if (process.argv.includes('--solid-backgrounds')) {
    positive.push('fill-merged', 'fill-nested', 'fill-split', 'fill-band', 'fill-transparent');
    negative.push('fill-border', 'fill-pattern');
  }
  const paintFailures = [];
  if (process.argv.includes('--fractional-cell-fit')) positive.push('fractional-source');
  if (process.argv.includes('--terminal-cell-end')) positive.push('terminal-source');
  if (process.argv.includes('--stored-margins')) {
    positive.push('stored-margin-left','stored-margin-center','stored-margin-right',
      ...[2,4,5,6,7,10,11].map(i=>`stored-margin-source-${i}`));
  }
  if (process.argv.includes('--pictures')) {
    for (const a of ['left','center','right']) positive.push(`picture-${a}`,`picture-${a}-defer`);
    positive.push('picture-rows','picture-crop-margin','picture-source-1','picture-source-8','picture-nested');
  }
  if (process.argv.includes('--stored-frames')) {
    positive.push('stored-frame-left', 'stored-frame-center', 'stored-frame-right',
      'stored-frame-empty-center', 'stored-frame-empty-right');
  }
  if (process.argv.includes('--linear-backgrounds')) {
    positive.push('gradient-horizontal', 'gradient-vertical', 'gradient-stepped', 'gradient-whole', 'inactive-diagonal');
  }
  if (process.argv.includes('--nested-alignment')) {
    positive.push('align-left', 'align-center', 'align-right', 'align-cell-padding', 'align-deep', 'align-header');
  }
  if (process.argv.includes('--cell-vertical-align')) {
    positive.push('valign-rows','valign-whole','valign-header','valign-nested','valign-parent');
  }
  if (process.argv.includes('--document-flow')) {
    positive.push('document-split','document-atomic','document-nested','document-hwp');
  }
  if (process.argv.includes('--stored-body')) positive.push('document-stored');
  if (process.argv.includes('--tac-metrics')) positive.push(
    'document-frame-left','document-frame-center','document-frame-right',
    'document-inline-signed-budget','document-inline-signed-nested','document-signed-rows-hwp');
  if (process.argv.includes('--stored-tac')) positive.push('document-inline', 'document-inline-rows', 'document-inline-nested');
  if (process.argv.includes('--structural-tac')) positive.push('document-inline-first', 'document-inline-first-rows');
  if (process.argv.includes('--empty-page-borders')) positive.push('document-empty-page-borders', 'document-empty-page-borders-hwp');
  if (process.argv.includes('--page-numbers')) positive.push(
    'document-number-4', 'document-number-5', 'document-number-6',
    'document-number-4-hwp', 'document-number-5-hwp', 'document-number-6-hwp',
    'document-number-start-hwp', 'document-number-hidden', 'document-number-tac-hwp');
  if (process.argv.includes('--rowspan')) positive.push('rowspan-groups','rowspan-whole','rowspan-header','rowspan-align','rowspan-nested','rowspan-spanning-header','rowspan-inner');
  if (process.argv.includes('--solid-borders')) {
    positive.push('border-grid', 'border-header', 'border-nested', 'border-one-sided');
    paintFailures.push('border-conflict');
  }
  if (process.argv.includes('--split-borders')) {
    positive.push('cut-lines', 'cut-tail', 'cut-siblings', 'cut-header', 'cut-nested', 'cut-padding');
  }
  if (process.argv.includes('--matching-table-borders')) {
    positive.push('outer-border-grid', 'outer-border-header', 'outer-border-nested', 'outer-border-one-sided',
      'outer-cut-lines', 'outer-cut-tail', 'outer-cut-nested');
    paintFailures.push('outer-missing', 'outer-conflict', 'outer-continuation');
  }
  const names = [...positive, ...negative, ...paintFailures];
  const js = readFileSync(join(pkg, 'rhwp.js')), wasm = readFileSync(join(pkg, 'rhwp_bg.wasm'));
  const files = new Map([
    ['/rhwp.js', ['application/javascript', js]],
    ['/rhwp_bg.wasm', ['application/wasm', wasm]],
    ...names.map(name => [`/${name}`, ['application/octet-stream', readFileSync(join(fixtures, `${name}.${name.endsWith('-hwp') || name==='document-inline-first-rows'?'hwp':'hwpx'}`))]]),
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
    const result = await page.evaluate(async ({ configs, positive, negative, paintFailures }) => {
      const m = await import('/rhwp.js');
      await m.default({ module_or_path: '/rhwp_bg.wasm' });
      const check = (ok, text) => { if (!ok) throw Error(text); };
      const throws = (fn, text) => { let failed = false; try { fn(); } catch (e) { failed = String(e).includes(text); } check(failed, `Expected rejection: ${text}`); };
      const input = {};
      for (const name of Object.keys(configs)) input[name] = new Uint8Array(await (await fetch(`/${name}`)).arrayBuffer());
      const open = (name, config = configs[name]) => name.startsWith('document-')
        ? new m.DocumentV2(input[name], JSON.stringify(config))
        : new m.TableV2Preview(input[name], JSON.stringify(config));
      check(typeof m.TableV2Preview === 'function', 'Missing experimental export');
      const pages = {};
      for (const name of positive) {
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
      for (const name of negative) throws(() => open(name),
        name.startsWith('separate-') ? 'V2 separate split-cell border' : 'Unsupported');
      if (positive.includes('document-split')) {
        check(typeof m.DocumentV2 === 'function', 'Missing document-body export');
        for (const options of [{}, {dpi:0,max_pages:1}, {dpi:96,max_pages:0},
          {dpi:96,max_pages:10,engine:'legacy'}]) {
          throws(() => new m.DocumentV2(input['document-split'],JSON.stringify(options)), 'Options');
        }
        throws(() => new m.DocumentV2(new Uint8Array([0]),JSON.stringify(configs['document-split'])), 'Parse');
        const limitedDocument = open('document-split',{...configs['document-split'],max_pages:1});
        try {
          check(limitedDocument.nextPage() !== undefined,'document first page before limit');
          for (let i=0;i<2;i++) {
            throws(() => limitedDocument.nextPage(),'PageLimit');
            check(limitedDocument.emittedPages()===1,'document limit rollback');
          }
        } finally { limitedDocument.free(); }
      }
      for (const name of paintFailures) {
        const session = open(name);
        try {
          const accepted = name === 'outer-continuation' ? 1 : 0;
          if (accepted) check(session.nextPage() !== undefined, 'accepted fragment before outline conflict');
          for (let i = 0; i < 2; i++) {
            throws(() => session.nextPage(), name.startsWith('outer-')
              ? 'V2 table/cell outline disagreement' : 'conflicting shared V2 cell borders');
            check(session.emittedPages() === accepted, 'paint failure rollback');
          }
        } finally { session.free(); }
      }
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
    }, { configs, positive, negative, paintFailures });
    mkdirSync(out, { recursive: true });
    const collect = (node, kind) => [ ...(node.node_type[kind] ? [node] : []), ...node.children.flatMap(n => collect(n, kind)) ];
    // Expectations independent of Native output: 18px lines, 36px body, repeated title.
    const expected = { merged: [['title', 'A'], ['title', 'B']], nested: [['title', 'A'], ['title', 'B'], ['host', 'after']], partial: [['title', 'A'], ['title', 'B']] };
    for (const a of ['left','center','right']) expected[`stored-margin-${a}`]=[['A'],['B'],[''],['after']];
    // Original-source text, grouped by saved row (not by font-run boundaries).
    const marginSource={
      2:['보 도 자 료'],4:['보도일시'],
      5:['2012.8.29(수) 조간부터 보도가능','(방송․인터넷매체는 28일 낮 12시)'],
      6:['담당부서'],7:['부산지방공정거래사무소 소비자과'],10:['담당자'],
      11:['과  장 이원두 051)460-1030','조사관 이효권 051)460-1034 '],
    };
    for (const [i,rows] of Object.entries(marginSource)) expected[`stored-margin-source-${i}`]=[rows];
    expected['fractional-source']=[['','','','www.ftc.go.kr','보 도 자 료',
      '함께 하는 공정사회!','더 큰 희망 대한민국','보도일시',...marginSource[5],
      '담당부서','부산지방공정거래사무소 소비자과','','배포일시','2012.8.28(화)',
      '담당자',...marginSource[11]]];
    expected['terminal-source']=expected['fractional-source'];
    expected['fill-merged'] = expected.merged;
    expected['fill-nested'] = expected.nested;
    expected['fill-split'] = [['A', ''], ['B', 'C']];
    expected['fill-band'] = [['A'], [], []];
    expected['fill-transparent'] = expected.merged;
    for (const name of ['gradient-horizontal', 'gradient-vertical', 'gradient-stepped', 'inactive-diagonal']) expected[name] = expected.merged;
    expected['gradient-whole'] = [['title', 'A']];
    for (const alignment of ['left','center','right']) expected[`stored-frame-${alignment}`] = [['A','B'],['after']];
    for (const alignment of ['center','right']) expected[`stored-frame-empty-${alignment}`] = [['','after']];
    for (const a of ['left','center','right']) {
      expected[`picture-${a}`]=[['before'],['after']];
      expected[`picture-${a}-defer`]=[['before'],[],['after']];
    }
    expected['picture-rows']=[['before'],[],[],['after']];
    expected['picture-crop-margin']=[['before'],['after']];
    expected['picture-source-1']=[[]];
    expected['picture-source-8']=[[]];
    expected['picture-nested']=[['before'],['after',''],['tail']];
    expected['border-grid'] = [['title', 'L1', 'R1']];
    expected['border-header'] = [['title', 'L1', 'R1'], ['title', 'L2', 'R2']];
    expected['border-nested'] = [...expected['border-header'], ['host', 'after']];
    expected['border-one-sided'] = [['L1', 'R1']];
    expected['cut-lines'] = [['A', ''], ['B', 'C']];
    expected['cut-tail'] = [['A'], [], []];
    expected['cut-siblings'] = [['L','A','B'], ['C','D']];
    expected['cut-header'] = [['title','L','A'], ['title','B'], ['title','C']];
    expected['cut-nested'] = [['A','B'], ['C','host'], ['after']];
    expected['cut-padding'] = [['A'], ['B']];
    for (const name of ['align-left','align-center','align-right','align-cell-padding']) {
      expected[name] = [['A','B'], ['C','host'], ['after']];
    }
    expected['align-deep'] = [['A','B'], ['C','middle'], ['host','after']];
    expected['align-header'] = [['title','A'], ['title','B'], ['host','after']];
    expected['valign-rows'] = [['T','','','C','','B'],['after']];
    expected['valign-whole'] = [['T','','','C','','B','after']];
    expected['valign-header'] = [['title','A'],['title','B']];
    expected['valign-nested'] = [['before','','C'],['D','host','after']];
    expected['valign-parent'] = [['A','host','after']];
    expected['document-split'] = [['before','A','B','C'],['D','E','host','after']];
    expected['document-atomic'] = [['before1','before2'],['A','B','C','host'],['after']];
    expected['document-nested'] = [['before','','A','B'],['C','D','inner','tail'],['host','after']];
    expected['document-stored'] = [['before','one','two','after'],['next']];
    expected['document-inline'] = [['before','lead'],['A','a','B','b','after']];
    for (const alignment of ['left','center','right']) expected[`document-frame-${alignment}`]=[['A','a','B','b','after']];
    expected['document-inline-signed-budget']=[['before'],['A','a','B','b','after']];
    expected['document-inline-signed-nested']=[['A','a','B','b','tail']];
    expected['document-signed-rows-hwp']=[['A','a','B','b','after']];
    expected['document-inline-rows'] = [['before','A','a'],['B','b']];
    expected['document-inline-nested'] = [['A','a','B','b','tail']];
    expected['document-inline-first'] = [['A','a','B','b','after']];
    expected['document-inline-first-rows'] = [['A','a'],['B','b','after']];
    expected['rowspan-groups'] = [['prefix'],['A','B','C','D','E','F'],['after']];
    expected['rowspan-whole'] = [['prefix','A','B','C','D','E','F','after']];
    expected['rowspan-header'] = [['prefix','A','B','C','D','E','F'],['prefix','after']];
    expected['rowspan-align'] = [['T','C','B','a','b']];
    expected['rowspan-nested'] = [['prefix'],['A','B','C','D','E','F'],['after','host','tail']];
    expected['rowspan-spanning-header'] = [['header','h1','h2','body1'],['header','h1','h2','body2']];
    expected['rowspan-inner'] = [['inner','host','tail','a','b']];
    for (const name of positive.filter(n=>n.startsWith('document-number-'))) {
      const body = name==='document-number-tac-hwp' ? expected['document-inline-first-rows'] : expected['document-split'];
      expected[name] = body.map((texts,i)=>[...texts,...(name.endsWith('-hidden') ? []
        : [name==='document-number-start-hwp' ? `[${9+i}]` : `- ${i+1} -`])]);
    }
    const raster = await browser.newPage();
    const artifacts = [];
    for (const [fixtureName, pages] of Object.entries(result.pages)) {
      // Matching table outlines must have the same independently specified
      // geometry/content as their cell-only counterparts, not extra paint.
      const name = ['document-hwp','document-empty-page-borders','document-empty-page-borders-hwp'].includes(fixtureName)
        ? 'document-split' : fixtureName.replace(/^outer-/, '');
      const native = JSON.parse(readFileSync(join(fixtures, `${fixtureName}.native.json`)));
      writeFileSync(join(out, `${fixtureName}.wasm.json`), JSON.stringify(pages, null, 2));
      assert.equal(pages.length, expected[name].length);
      for (const [index, output] of pages.entries()) {
        assert.equal(output.engine, 'table_v2'); assert.equal(output.scope, name.startsWith('document-') ? 'document_body' : 'selected_table');
        assert.equal(output.page_index, index + (name === 'partial' ? 1 : 0));
        const rootNode = output.render_tree.root;
        const lines = collect(rootNode, 'TextLine');
        assert.deepEqual(name.startsWith('stored-margin-source-') || name==='fractional-source' || name==='terminal-source'
          ? lines.map(n=>collect(n,'TextRun').map(r=>r.node_type.TextRun.text).join(''))
          : collect(rootNode, 'TextRun').map(n => n.node_type.TextRun.text), expected[name][index]);
        const aligned = name.startsWith('align-');
        const deep = name === 'align-deep', header = name === 'align-header';
        const childX = name === 'align-left' ? 30 : name === 'align-right' ? 90 : 60;
        const bordered = name.startsWith('border-');
        const cut = name.startsWith('cut-');
        const borderTail = name === 'border-nested' && index === 2;
        if (name==='fractional-source' || name==='terminal-source') {
          // Integer-HWP-unit output of the SAME unmodified binary. Scale
          // invariance is not a Hancom fidelity/terminal-height approval.
          const unit=JSON.parse(readFileSync(join(fixtures,`${name}-units.native.json`)))[0].render_tree.root;
          if (name==='terminal-source') assert.ok(Math.abs(collect(rootNode,'Table')[0].bbox.height-11102/75)<1e-9);
          assert.equal(collect(rootNode,'Table').length,1);
          assert.equal(collect(rootNode,'TableCell').length,12);
          assert.equal(collect(rootNode,'Image').length,2);
          for(const kind of ['Table','TableCell','TextLine','Image']) {
            assertDocumentGeometry(collect(rootNode,kind).map(n=>n.bbox),
              collect(unit,kind).map(n=>Object.fromEntries(Object.entries(n.bbox).map(([k,v])=>[k,v/75]))));
          }
        } else if (name.startsWith('stored-margin-source-')) {
          const ci=Number(name.split('-').at(-1));
          const sw=ci===2?25652:[4,6,10].includes(ci)?5036:19832;
          for (const line of lines) {
            assertDocumentGeometry([line.bbox.x,line.bbox.width],[20+641/75,sw/75]);
            for(const run of line.children) {
              assert.ok(run.bbox.x>=line.bbox.x-1e-9);
              assert.ok(run.bbox.x+run.bbox.width<=line.bbox.x+line.bbox.width+1e-9);
            }
          }
        } else if (name.startsWith('stored-margin-')) {
          const b={x:index===1?40:30,y:30,width:index===1?160:170,height:12};
          assertDocumentGeometry(lines.map(n=>n.bbox),[b]);
          assertDocumentGeometry(collect(rootNode,'TableCell').map(n=>n.bbox),[{x:20,y:30,width:200,height:18}]);
          const r=lines[0].children[0].bbox;
          assertDocumentGeometry(r.x,name.endsWith('-center')?b.x+(b.width-r.width)/2:name.endsWith('-right')?b.x+b.width-r.width:b.x);
          if(index===2) assert.equal(r.width,0);
        } else if (name.startsWith('picture-source-')) {
          const pictures=collect(rootNode,'Image');
          assert.equal(pictures.length,1);
          const [w,h]=name.endsWith('-1')?[8021,5064]:[10738,4617];
          assertDocumentGeometry([pictures[0].bbox.width,pictures[0].bbox.height],[w/75,h/75]);
          const cell=collect(rootNode,'TableCell')[0].bbox, b=pictures[0].bbox;
          assert.ok(b.x>=cell.x&&b.y>=cell.y&&b.x+b.width<=cell.x+cell.width+1e-9&&b.y+b.height<=cell.y+cell.height+1e-9);
        } else if (name.startsWith('picture-')) {
          const defer=name.endsWith('-defer'), rows=name==='picture-rows', cropped=name==='picture-crop-margin';
          const imagePage=rows ? index===1||index===2 : index===(defer?1:0);
          const x=cropped?72:rows||name==='picture-nested'?30:({left:30,center:75,right:120}[name.split('-')[1]]);
          const y=cropped?52:defer||rows?30:48;
          const boxes=imagePage ? Array.from({length:rows?1:2},(_,i)=>({x:x+i*(cropped?45:40),y,width:40,height:cropped?16:24})) : [];
          assertDocumentGeometry(collect(rootNode,'Image').map(n=>n.bbox),boxes);
          assertDocumentGeometry(collect(rootNode,'TableCell').map(n=>n.bbox.height),
            name==='picture-nested'?[[42,42],[42,24],[18]][index]:[rows ? [18,30,30,18][index] : defer ? [18,30,18][index] : [42,24][index]]);
          for (const line of lines) assert.ok(line.bbox.y+line.bbox.height<=30+configs[fixtureName].pages.body.height);
        } else if (name.startsWith('stored-frame-')) {
          const empty=name.startsWith('stored-frame-empty-');
          const boxes=empty ? [{x:30,y:30,width:150,height:12},{x:20,y:48,width:200,height:12}]
            : index===0 ? [{x:30,y:30,width:150,height:12},{x:40,y:48,width:170,height:12}]
            : [{x:20,y:30,width:200,height:12}];
          assertDocumentGeometry(lines.map(n=>n.bbox),boxes);
          assertDocumentGeometry(collect(rootNode,'TableCell').map(n=>n.bbox),[{x:20,y:30,width:200,height:index===0?36:18}]);
          if(empty) assert.equal(lines[0].children[0].bbox.width,0);
          for(const line of lines) {
            const run=line.children[0].bbox, b=line.bbox;
            const x=name.endsWith('-center') ? b.x+(b.width-run.width)/2
              : name.endsWith('-right') ? b.x+b.width-run.width : b.x;
            assertDocumentGeometry(run.x,x);
          }
        } else if (name.startsWith('document-frame-') || name.includes('-signed-')) {
          const budget=name==='document-inline-signed-budget', nested=name==='document-inline-signed-nested';
          const rows=name==='document-signed-rows-hwp', frame=name.startsWith('document-frame-');
          const x=frame ? 32+({left:0,center:36,right:72}[name.split('-').at(-1)]) : rows ? 130 : 88;
          const boxes=budget && index===0 ? [] : [
            ...(nested ? [{x:20,y:30,width:300,height:40}] : []),
            {x,y:32,width:80,height:36},
            {x:rows?130:x+84,y:rows?71:32,width:80,height:36}];
          const body=collect(rootNode,'Body')[0];
          assertDocumentGeometry(body.bbox,{x:20,y:30,width:300,height:budget?57:rows?120:72});
          assertDocumentGeometry(collect(body,'Table').map(n=>n.bbox),boxes);
          assertDocumentGeometry(collect(body,'TableCell').map(n=>n.bbox),boxes);
          assertDocumentGeometry(lines.map(n=>n.bbox.y),budget && index===0 ? [30]
            : rows ? [32,50,71,89,108] : [32,50,32,50,frame?74:nested?70:69]);
          assertDocumentGeometry(lines.map(n=>n.bbox.x),budget && index===0 ? [20]
            : [x,x,rows?130:x+84,rows?130:x+84,20]);
          for (const n of [...lines,...collect(body,'TableCell')]) assert.ok(n.bbox.y+n.bbox.height<=body.bbox.y+body.bbox.height);
          const edges=collect(rootNode,'Line').map(n=>['x1','y1','x2','y2'].map(k=>n.node_type.Line[k]));
          const expectedEdges=boxes.flatMap(({x,y,width:w,height:h})=>[[x,y,x,y+h],[x+w,y,x+w,y+h],[x,y,x+w,y],[x,y+h,x+w,y+h]]);
          assert.equal(edges.length,expectedEdges.length);
          for (const edge of expectedEdges) {
            const at=edges.findIndex(candidate=>candidate.every((v,i)=>sameCoordinate(v,edge[i])));
            assert.ok(at>=0,`missing saved TAC edge ${edge}`); edges.splice(at,1);
          }
        } else if (name.startsWith('document-number-')) {
          const body=collect(rootNode,'Body')[0], tac=name==='document-number-tac-hwp';
          assertDocumentGeometry(body.bbox,{x:20,y:30,width:300,height:72});
          const bodyLines=collect(body,'TextLine');
          assertDocumentGeometry(bodyLines.map(n=>n.bbox.y), tac ? (index===0 ? [32,50] : [32,50,74])
            : (index===0 ? [30,48,66,84] : [30,48,66,84]));
          assertDocumentGeometry(bodyLines.map(n=>n.bbox.x), tac ? (index===0 ? [130,130] : [130,130,20])
            : (index===0 ? [20,20,20,20] : [20,20,20,20]));
          const boxes=tac ? [{x:130,y:32,width:80,height:36}]
            : [{x:20,y:index===0?48:30,width:200,height:index===0?54:36}];
          assertDocumentGeometry(collect(body,'Table').map(n=>n.bbox),boxes);
          assertDocumentGeometry(collect(body,'TableCell').map(n=>n.bbox),boxes);
          for (const n of [...bodyLines,...collect(body,'TableCell')]) assert.ok(n.bbox.y+n.bbox.height<=102 || sameCoordinate(n.bbox.y+n.bbox.height,102));
          if (name.endsWith('-hidden')) assert.equal(rootNode.children.length,1);
          else {
            assert.equal(rootNode.children.length,2);
            const story=rootNode.children[1], run=story.children[0];
            assertDocumentGeometry(run.node_type.TextRun.style.font_size,40/3);
            assertDocumentGeometry(run.node_type.TextRun.baseline,40/3);
            // TAC fixture uses the same footer distances/body frame as other documents.
            assertDocumentGeometry(run.bbox.y,102+15+40/9);
            assertDocumentGeometry(story.bbox.y,run.bbox.y);
            assertDocumentGeometry(story.bbox.height,16);
            assertDocumentGeometry(run.bbox.width,story.bbox.width);
            const pos=name.startsWith('document-number-4') ? 4 : name.startsWith('document-number-6') ? 6 : 5;
            assertDocumentGeometry(run.bbox.x+(pos===4 ? 0 : run.bbox.width/(pos===5 ? 2 : 1)),pos===4 ? 20 : pos===5 ? 170 : 320);
            assert.ok(story.bbox.y+story.bbox.height<200);
          }
        } else if (name.startsWith('document-')) {
          assertDocumentGeometry(collect(rootNode,'Body').map(n=>n.bbox),[{x:20,y:30,width:300,height:72}]);
          const inline = name.startsWith('document-inline');
          const firstRows = name === 'document-inline-first-rows';
          const inlineYs = firstRows ? (index===0 ? [32,50] : [32,50,74])
            : name === 'document-inline-rows' ? (index===0 ? [30,50,68] : [32,50])
            : name === 'document-inline' && index===0 ? [30,48] : [32,50,32,50,74];
          assertDocumentGeometry(lines.map(n=>n.bbox.y),inline ? inlineYs : expected[name][index].map((_,i)=>30+18*i));
          const inlineBoxes = firstRows ? [{x:130,y:32,width:80,height:36}]
            : name === 'document-inline-rows' ? [{x:130,y:index===0?50:32,width:80,height:36}]
            : name === 'document-inline' && index===0 ? []
            : [...(name === 'document-inline-nested' ? [{x:20,y:30,width:300,height:44}] : []),
              {x:88,y:32,width:80,height:36},{x:172,y:32,width:80,height:36}];
          const tableBoxes = inline ? inlineBoxes : name === 'document-stored' ? [] : name === 'document-split'
            ? [{x:70,y:index===0?48:30,width:200,height:index===0?54:36}]
            : name === 'document-atomic'
            ? (index===1 ? [{x:70,y:30,width:200,height:54}] : [])
            : index===0 ? [{x:70,y:48,width:200,height:54},{x:120,y:66,width:100,height:36}]
            : index===1 ? [{x:70,y:30,width:200,height:72},{x:120,y:30,width:100,height:36}] : [];
          assertDocumentGeometry(collect(rootNode,'Table').map(n=>n.bbox),tableBoxes);
          assertDocumentGeometry(collect(rootNode,'TableCell').map(n=>n.bbox),tableBoxes);
          const expectedEdges=tableBoxes.flatMap(({x,y,width:w,height:h})=>
            [[x,y,x,y+h],[x+w,y,x+w,y+h],[x,y,x+w,y],[x,y+h,x+w,y+h]]);
          const actualEdges=collect(rootNode,'Line').map(n=>['x1','y1','x2','y2'].map(k=>n.node_type.Line[k]));
          assert.equal(actualEdges.length,expectedEdges.length);
          const remainingEdges=[...actualEdges];
          for (const edge of expectedEdges) {
            const match=remainingEdges.findIndex(candidate=>candidate.every((v,i)=>sameCoordinate(v,edge[i])));
            assert.ok(match>=0,`missing document edge ${edge}`); remainingEdges.splice(match,1);
          }
          const inlineXs = firstRows ? (index===0 ? [130,130] : [130,130,20])
            : name === 'document-inline-rows' ? (index===0 ? [20,130,130] : [130,130])
            : name === 'document-inline' && index===0 ? [20,20] : [88,88,172,172,20];
          const xs = inline ? inlineXs : name === 'document-stored' ? expected[name][index].map(()=>20)
            : name === 'document-split' ? (index===0 ? [20,70,70,70] : [70,70,20,20])
            : name === 'document-atomic' ? (index===1 ? [70,70,70,20] : expected[name][index].map(()=>20))
            : index===0 ? [20,70,120,120] : index===1 ? [120,120,70,70] : [20,20];
          assertDocumentGeometry(lines.map(n=>n.bbox.x),xs);
          for (const n of [...lines,...collect(rootNode,'TableCell')]) {
            const end=n.bbox.y+n.bbox.height;
            assert.ok(end<=102 || sameCoordinate(end,102));
          }
        } else if (name.startsWith('rowspan-')) {
          const box=(x,y,width,height)=>({x,y,width,height});
          const group=y=>[box(20,y,60,72),box(80,y,120,36),box(80,y+36,60,72),
            box(140,y+36,60,36),box(20,y+72,60,36),box(140,y+72,60,36)];
          const small=box(20,30,180,18);
          const aligned=name==='rowspan-align';
          const nested=name==='rowspan-nested';
          const spanHeader=name==='rowspan-spanning-header',inner=name==='rowspan-inner';
          let cells=spanHeader ? [box(20,30,60,72),box(80,30,60,36),box(80,66,60,36),box(20,102,120,18)]
            : inner ? [box(20,30,60,72),box(20,39,60,18),box(80,30,60,36),box(80,66,60,36)]
            : aligned ? [box(20,30,60,72),box(80,30,60,72),box(140,30,60,72),box(200,30,60,36),box(200,66,60,36)]
            : name==='rowspan-whole' ? [small,...group(48),box(20,156,180,18)]
            : name==='rowspan-header' ? (index===0 ? [small,...group(48)] : [small,box(20,48,180,18)])
            : index===1 ? group(30) : [small];
          if(nested) cells=[box(20,30,180,[18,108,54][index]),...cells];
          assertDocumentGeometry(collect(rootNode,'TableCell').map(n=>n.bbox),cells);
          const tableHeight=spanHeader ? 90 : inner||aligned ? 72
            : name==='rowspan-whole' ? 144 : name==='rowspan-header' ? (index===0?126:36)
            : nested ? [18,108,54][index] : [18,108,18][index];
          const tables=[box(20,30,aligned?240:spanHeader||inner?120:180,tableHeight)];
          if(nested) tables.push(box(20,30,180,[18,108,18][index]));
          if(inner) tables.push(box(20,39,60,18));
          assertDocumentGeometry(collect(rootNode,'Table').map(n=>n.bbox),tables);
          const ys=spanHeader ? [30,30,66,102] : inner ? [39,57,75,30,66] : aligned ? [36,54,72,36,72]
            : name==='rowspan-whole' ? [30,48,48,84,84,120,120,156]
            : name==='rowspan-header' ? (index===0 ? [30,48,48,84,84,120,120] : [30,48])
            : index===1 ? [30,30,66,66,102,102] : nested&&index===2 ? [30,48,66] : [30];
          assertDocumentGeometry(lines.map(n=>n.bbox.y),ys);
          const body=configs[fixtureName].pages.body;
          assert.ok(cells.every(b=>b.y>=body.y && b.y+b.height<=body.y+body.height));
          if(name==='rowspan-groups' && index===1) {
            assertDocumentGeometry(collect(rootNode,'Line').map(n=>['x1','y1','x2','y2'].map(k=>n.node_type.Line[k])),[
              [20,30,20,138],[80,30,80,138],[140,66,140,138],[200,30,200,138],
              [20,30,200,30],[80,66,200,66],[20,102,80,102],[140,102,200,102],[20,138,200,138]]);
            assert.deepEqual(collect(rootNode,'TableCell').map(n=>n.node_type.TableCell.row_span),[2,1,2,1,1,1]);
          }
        } else if (name.startsWith('valign-')) {
          const contract=verticalContract(name,index);
          assert.deepEqual(lines.map(n=>n.bbox.x),contract.xs);
          assert.deepEqual(lines.map(n=>n.bbox.y),contract.ys);
          //9pt at96DPI gives12px text boxes; fixed18px flow pitch is separate.
          assert.ok(lines.every(n=>n.bbox.height===12));
          assert.deepEqual(collect(rootNode,'TableCell').map(n=>n.bbox),contract.cells);
          assert.deepEqual(collect(rootNode,'Table').map(n=>n.bbox),contract.tables);
          assert.deepEqual(collect(rootNode,'Line').map(n=>['x1','y1','x2','y2'].map(k=>n.node_type.Line[k])),contract.edges);
          const body=configs[fixtureName].pages.body;
          assert.ok(contract.cells.every(b=>b.y>=body.y && b.y+b.height<=body.y+body.height));
        } else {
        const cutYs = name === 'cut-tail' ? (index === 0 ? [30] : [])
          : name === 'cut-siblings' ? (index === 0 ? [30,30,48] : [30,48])
          : name === 'cut-header' ? (index === 0 ? [30,48,48] : [30,48])
          : name === 'cut-padding' ? (index === 0 ? [48] : [30])
          : name === 'cut-nested' && index === 2 ? [30] : [30,48];
        const ys = cut ? cutYs : name === 'fill-band' ? (index === 0 ? [30] : [])
          : bordered && !borderTail ? (name === 'border-one-sided' ? [30,30] : [30,48,48]) : [30,48];
        assert.deepEqual(lines.map(n => n.bbox.y), aligned
          ? (index === 2 && !deep && !header ? [30] : [30,48]) : ys);
        const cutXs = name === 'cut-siblings' ? (index === 0 ? [20,120,120] : [120,120])
          : name === 'cut-header' ? (index === 0 ? [20,20,120] : [20,120])
          : lines.map(() => name === 'cut-lines' ? 30 : 20);
        const xs = cut ? cutXs : bordered && !borderTail ? (name === 'border-one-sided' ? [20,120] : [20,20,120])
          : lines.map(() => name === 'fill-split' ? 30 : 20);
        assert.deepEqual(lines.map(n => n.bbox.x), aligned
          ? header ? (index < 2 ? [90,90] : [30,30])
          : deep ? [[120,120],[120,100],[30,30]][index]
          : [[childX,childX],[childX,30],[30]][index] : xs);
        assert.ok(lines.every(n => n.bbox.y + n.bbox.height <= 66));
        for (const c of collect(rootNode, 'TableCell')) {
          const span = c.node_type.TableCell.col_span;
          if (aligned) {
            assert.equal(span, 1);
          } else if (bordered || cut) {
            assert.ok(span === 1 || span === 2);
            assert.equal(c.bbox.x, 20 + c.node_type.TableCell.col * 100);
          } else { assert.equal(span, 2); assert.equal(c.bbox.x, 20); }
          if (!aligned) assert.equal(c.bbox.width, span * 100);
          assert.ok(c.bbox.y >= 30 && c.bbox.y + c.bbox.height <= 66);
        }
        if (aligned) {
          // Independent geometric oracle: declared widths/padding and18px lines.
          // Nested origin:20+10+(160-100)=90; grandchild90+10+(80-40)/2=120.
          const boxes = [{x:20,y:30,width:200,height:index===2 && !deep && !header ? 18 : 36}];
          if (index < 2) {
            if (header) boxes.push({x:90,y:30,width:100,height:18},{x:90,y:48,width:100,height:18});
            else if (deep) boxes.push({x:90,y:30,width:100,height:36},{x:120,y:30,width:40,height:index===0 ? 36 : 18});
            else boxes.push({x:childX,y:30,width:100,height:index===0 ? 36 : 18});
          }
          assert.deepEqual(collect(rootNode,'TableCell').map(n=>n.bbox),boxes);
          const boxEdges = ({x,y,width:w,height:h}) => [[x,y,x,y+h],[x+w,y,x+w,y+h],[x,y,x+w,y],[x,y+h,x+w,y+h]];
          const geometry = header && index < 2
            ? [[90,30,90,66],[190,30,190,66],[90,30,190,30],[90,48,190,48],[90,66,190,66],...boxEdges(boxes[0])]
            : boxes.toReversed().flatMap(boxEdges);
          assert.deepEqual(collect(rootNode,'Line').map(n=>['x1','y1','x2','y2'].map(k=>n.node_type.Line[k])),geometry);
        }
        if (cut) {
          const box = height => [[20,30,20,30+height],[220,30,220,30+height],[20,30,220,30],[20,30+height,220,30+height]];
          let geometry = box((name === 'cut-tail' || name === 'cut-nested') && index === 2 ? 18 : 36);
          if (name === 'cut-siblings') geometry.splice(1,0,[120,30,120,66]);
          if (name === 'cut-header') {
            geometry.splice(1,0,[120,48,120,66]);
            geometry.splice(4,0,[20,48,220,48]);
          }
          if (name === 'cut-nested' && index < 2) geometry = [...box(index === 0 ? 36 : 18), ...geometry];
          if (name === 'cut-padding') geometry.splice(2,1); // None source top edge remains absent.
          const edges = collect(rootNode, 'Line');
          assert.deepEqual(edges.map(n => ['x1','y1','x2','y2'].map(k => n.node_type.Line[k])), geometry);
          for (const edge of edges) {
            assert.equal(edge.node_type.Line.style.color, 0x332211);
            assert.ok(Math.abs(edge.node_type.Line.style.width - 1.92) < 1e-9);
          }
        }
        if (bordered) {
          const edges = collect(rootNode, 'Line');
          const geometry = borderTail ? [] : name === 'border-one-sided'
            ? [[120,30,120,48],[220,30,220,48],[120,30,220,30],[120,48,220,48]]
            : [[20,30,20,66],[120,48,120,66],[220,30,220,66],[20,30,220,30],[20,48,220,48],[20,66,220,66]];
          assert.deepEqual(edges.map(n => ['x1','y1','x2','y2'].map(k => n.node_type.Line[k])), geometry);
          for (const edge of edges) {
            assert.equal(edge.node_type.Line.style.color, 0x332211);
            assert.ok(Math.abs(edge.node_type.Line.style.width - 1.92) < 1e-9);
          }
        }
        if (name.startsWith('gradient-')) {
          const rectangles = collect(rootNode, 'Rectangle');
          assert.equal(rectangles.length, name === 'gradient-whole' ? 1 : 2);
          for (const cell of collect(rootNode, name === 'gradient-whole' ? 'Table' : 'TableCell')) {
            const rect = cell.children[0];
            assert.deepEqual(rect.bbox, cell.bbox);
            const g = rect.node_type.Rectangle.gradient;
            assert.equal(g.gradient_type, 1);
            assert.equal(g.angle, name === 'gradient-vertical' ? 0 : 90);
            assert.equal(g.colors.length, name === 'gradient-stepped' ? 100 : 2);
            assert.equal(g.colors[0], 0);
            assert.equal(g.colors.at(-1), name === 'gradient-stepped' ? 0xEFEFEF : 0xFFFFFF);
            assert.equal(g.positions[0], 0); assert.equal(g.positions.at(-1), 1);
          }
        }
        if (name === 'inactive-diagonal') {
          assert.equal(collect(rootNode, 'Line').length, 0);
          assert.deepEqual(collect(rootNode, 'Rectangle').map(n => n.node_type.Rectangle.style.fill_color), [0xEEEEFF]);
        }
        if (name.startsWith('fill-')) {
          const colors = name === 'fill-merged' ? [0xFFEEEE, 0xEEEEFF, 0xEEFFEE]
            : name === 'fill-nested' ? (index < 2 ? [0xEEEEFF, 0xEEFFEE, 0xFFEEEE] : [0xEEEEFF])
            : name === 'fill-transparent' ? (index === 0 ? [0xEEEEFF] : [0xEEEEFF, 0xFFFFFF])
            : name === 'fill-split' ? [0xEEFFEE] : [0xFFEEEE];
          const rectangles = collect(rootNode, 'Rectangle');
          assert.deepEqual(rectangles.map(n => n.node_type.Rectangle.style.fill_color), colors);
          // Background is the first child, never a separate recomputed bbox or a clip.
          const verify = node => {
            for (const [i, child] of node.children.entries()) {
              if (child.node_type.Rectangle) {
                assert.equal(i, 0); assert.deepEqual(child.bbox, node.bbox);
                assert.equal(child.node_type.Rectangle.style.stroke_color, null);
                assert.equal(child.node_type.Rectangle.style.stroke_width, 0);
              }
              verify(child);
            }
          };
          verify(rootNode);
          if (name === 'fill-band') assert.equal(collect(rootNode, 'TableCell')[0].bbox.height, index < 2 ? 36 : 18);
        }
        }
        const stem = `${fixtureName}-${index}`;
        writeFileSync(join(out, `${stem}.svg`), output.svg);
        // Preserve evidence even if a backend comparison fails below.
        const rasterWidth=['fractional-source','terminal-source'].includes(name)?800:400;
        await raster.setViewport({ width: rasterWidth, height: 400, deviceScaleFactor: 1 });
        for (const [backend, svg] of [['native', native[index].svg], ['wasm', output.svg]]) {
          await raster.setContent(`<body style="margin:0;background:white">${svg}</body>`);
          await raster.evaluate(() => document.fonts.ready);
          await raster.screenshot({ path: join(out, `${stem}.${backend}.png`) });
        }
        // Same Chrome raster environment; RGB overlay reference is Native, NOT Hancom.
        const imgs = ['native', 'wasm'].map(b => `data:image/png;base64,${readFileSync(join(out, `${stem}.${b}.png`)).toString('base64')}`);
        if (name.startsWith('picture-') && !name.startsWith('picture-source-')) {
          const cropped=name==='picture-crop-margin';
          const points=collect(rootNode,'Image').flatMap(n=>[[n.bbox.x+5,n.bbox.y+4],[n.bbox.x+35,n.bbox.y+4],[n.bbox.x+5,n.bbox.y+n.bbox.height-4],[n.bbox.x+35,n.bbox.y+n.bbox.height-4]]);
          const pixels=await raster.evaluate(async ({imgs,points})=>Promise.all(imgs.map(async url=>{
            const i=new Image();i.src=url;await i.decode();const c=document.createElement('canvas');c.width=400;c.height=400;
            const ctx=c.getContext('2d');ctx.drawImage(i,0,0);return points.map(([x,y])=>[...ctx.getImageData(x,y,1,1).data]);
          })),{imgs,points});
          const colors=[[255,0,0,255],[0,255,0,255],[0,0,255,255],[255,255,0,255]];
          for(const samples of pixels) assert.deepEqual(samples,points.map((_,i)=>colors[cropped?0:i%4]),`${stem}: decoded embedded picture colors`);
        }
        if ((bordered && !borderTail) || cut) {
          const bottom = (name === 'cut-tail' || name === 'cut-nested') && index === 2 ? 48 : 66;
          const points = cut ? [[20,40],[220,40],[150,bottom],[17,40],[150,bottom+3]]
            : name === 'border-one-sided'
            ? [[150,30],[120,40],[150,48],[117,40],[150,51]]
            : [[150,30],[120,55],[150,48],[17,40],[150,69]];
          const samples = await raster.evaluate(async ({ imgs, points }) => Promise.all(imgs.map(async url => {
            const image = new Image(); image.src = url; await image.decode();
            const c = document.createElement('canvas'); c.width = 400; c.height = 400;
            const ctx = c.getContext('2d'); ctx.drawImage(image, 0, 0);
            return points.map(([x,y]) => [...ctx.getImageData(x,y,1,1).data]);
          })), { imgs, points });
          for (const sample of samples) {
            assert.ok(sample.slice(0,3).every(rgba => rgba.slice(0,3).every(v => v < 80)), `${stem}: visible dark edges including shared edge`);
            assert.ok(sample.slice(3).every(rgba => rgba.every(v => v === 255)), `${stem}: no ink beyond half-stroke envelope`);
          }
        }
        if (name.startsWith('fill-')) {
          const rgb = color => [color & 255, (color >> 8) & 255, (color >> 16) & 255, 255];
          const rowColors = name === 'fill-merged' ? [0xEEEEFF, 0xEEFFEE]
            : name === 'fill-nested' ? (index < 2 ? [0xEEFFEE, 0xFFEEEE] : [0xEEEEFF, 0xEEEEFF])
            : name === 'fill-transparent' ? [0xEEEEFF, index === 0 ? 0xEEEEFF : 0xFFFFFF]
            : name === 'fill-split' ? [0xEEFFEE, 0xEEFFEE]
            : [0xFFEEEE, index < 2 ? 0xFFEEEE : 0xFFFFFF];
          const samples = await raster.evaluate(async imgs => Promise.all(imgs.map(async url => {
            const image = new Image(); image.src = url; await image.decode();
            const c = document.createElement('canvas'); c.width = 400; c.height = 400;
            const ctx = c.getContext('2d'); ctx.drawImage(image, 0, 0);
            return [[200,35],[200,53],[200,67],[19,35],[221,35]].map(([x,y]) => [...ctx.getImageData(x,y,1,1).data]);
          })), imgs);
          for (const sample of samples) assert.deepEqual(sample, [...rowColors.map(rgb), rgb(0xFFFFFF), rgb(0xFFFFFF), rgb(0xFFFFFF)], `${stem}: visible fill and no overflow`);
        }
        if (name.startsWith('gradient-')) {
          const points = name === 'gradient-vertical' ? [[200,47],[200,31]] : [[21,40],[218,40]];
          const samples = await raster.evaluate(async ({imgs,points}) => Promise.all(imgs.map(async url => {
            const image = new Image(); image.src = url; await image.decode();
            const c = document.createElement('canvas'); c.width = 400; c.height = 400;
            const ctx = c.getContext('2d'); ctx.drawImage(image,0,0);
            return points.map(([x,y]) => [...ctx.getImageData(x,y,1,1).data]);
          })), {imgs,points});
          for (const sample of samples) {
            assert.ok(sample[0].slice(0,3).every(v => v < 30), `${stem}: dark start`);
            assert.ok(sample[1].slice(0,3).every(v => v > 225), `${stem}: light end`);
          }
        }
        const overlay = await raster.evaluate(async ({imgs,width}) => {
          const canvases = await Promise.all(imgs.map(async url => {
            const image = new Image(); image.src = url; await image.decode();
            const c = document.createElement('canvas'); c.width = width; c.height = 400;
            c.getContext('2d').drawImage(image, 0, 0); return c;
          }));
          const context = canvases[0].getContext('2d'), a = context.getImageData(0, 0, width, 400), b = canvases[1].getContext('2d').getImageData(0, 0, width, 400);
          for (let i = 0; i < a.data.length; i += 4) {
            const r = Math.round((a.data[i] + a.data[i + 1] + a.data[i + 2]) / 3);
            const g = Math.round((b.data[i] + b.data[i + 1] + b.data[i + 2]) / 3);
            a.data[i] = r; a.data[i + 1] = g; a.data[i + 2] = g; a.data[i + 3] = 255;
          }
          context.putImageData(a, 0, 0); return canvases[0].toDataURL();
        }, {imgs,width:rasterWidth});
        writeFileSync(join(out, `${stem}.overlay.png`), Buffer.from(overlay.split(',')[1], 'base64'));
        await raster.setViewport({ width: rasterWidth*3, height: 430, deviceScaleFactor: 1 });
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
    console.log(`PASS: ${artifacts.length} page exports; exact Native/WASM tree+SVG parity; isolation, rejection, rollback, termination. ${manifest.browser}`);
  } finally {
    try {
      if (browser) await browser.close();
    } finally {
      await new Promise(ok => server.close(ok));
    }
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });

import test from 'node:test';
import assert from 'node:assert/strict';
import { TableV2Session, resolveTypesetMode, splitSvgFontFamilies } from '../src/core/table-v2-session.ts';

test('SVG fallback lists supply separate font names to the existing font loader', () => {
  assert.deepEqual(splitSvgFontFamilies(`'HY신명조',"HCR Batang", serif`), ['HY신명조', 'HCR Batang', 'serif']);
  assert.deepEqual(splitSvgFontFamilies(`'A,B', Arial`), ['A,B', 'Arial']);
});

test('V2 runtime is explicit; a separate dev mode defaults to V2', () => {
  assert.equal(resolveTypesetMode('', 'development'), 'legacy');
  assert.equal(resolveTypesetMode('?typeset=v2', 'development'), 'v2');
  assert.equal(resolveTypesetMode('', 'table-v2'), 'v2');
  assert.equal(resolveTypesetMode('?typeset=legacy', 'table-v2'), 'legacy');
});

test('V2 owns page count, packet and handle lifetime without Legacy construction', () => {
  let freed = 0;
  const session = new TableV2Session((bytes, options) => {
    assert.deepEqual([...bytes], [1, 2]);
    assert.deepEqual(JSON.parse(options), { section: 2, dpi: 96, cell_end_policy: 'omit_final_paragraph_gap' });
    return { pageCount: () => 2, free: () => { freed++; },
      renderPage: index => JSON.stringify({ engine: 'table_v2_hosted_section', page_index: index, svg: '<svg/>' }) };
  });
  session.open(new Uint8Array([1, 2]), 2);
  assert.equal(session.pageCount, 2);
  assert.equal(session.render(1), '<svg/>');
  assert.throws(() => session.render(2), /범위/);
  session.close(); session.close();
  assert.equal(freed, 1);
  assert.equal(session.pageCount, 0);
  assert.throws(() => session.render(0), /열려/);
});

test('a rejected replacement clears the old V2 session and surfaces the actual error', () => {
  let calls = 0, freed = 0;
  const session = new TableV2Session(() => {
    if (++calls === 2) throw Error('UnsupportedHost');
    return { pageCount: () => 1, free: () => { freed++; }, renderPage: () => '' };
  });
  session.open(new Uint8Array(), 0);
  assert.throws(() => session.open(new Uint8Array(), 0), /UnsupportedHost/);
  assert.equal(calls, 2);
  assert.equal(freed, 1);
  assert.equal(session.pageCount, 0);
});

test('wrong engine or page cannot be displayed as V2', () => {
  for (const packet of [{ engine: 'legacy', page_index: 0, svg: '<svg/>' },
    { engine: 'table_v2_hosted_section', page_index: 1, svg: '<svg/>' }]) {
    const session = new TableV2Session(() => ({ pageCount: () => 1, free() {}, renderPage: () => JSON.stringify(packet) }));
    session.open(new Uint8Array(), 0);
    assert.throws(() => session.render(0), /계약/);
    session.close();
  }
});

test('invalid section and page count fail closed', () => {
  let freed = 0;
  const session = new TableV2Session(() => ({ pageCount: () => NaN, free: () => { freed++; }, renderPage: () => '' }));
  assert.throws(() => session.open(new Uint8Array(), -1), /구역/);
  assert.throws(() => session.open(new Uint8Array(), 0), /쪽/);
  assert.equal(freed, 1);
  assert.equal(session.pageCount, 0);
});

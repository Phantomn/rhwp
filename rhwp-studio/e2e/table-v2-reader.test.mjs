/** Real Studio upload -> HostedSectionV2 WASM. No Legacy runTest bootstrap. */
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { launchBrowser, createPage, closeBrowser } from './helpers.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = path.join(root, 'output/7353/closeout/wasm-v2/studio');
const url = new URL(process.env.VITE_URL || 'http://localhost:7700');
url.searchParams.set('typeset', 'preview');
await mkdir(output, { recursive: true });
const browser = await launchBrowser();
try {
  const page = await createPage(browser, 1280, 1000);
  const requests = [];
  const errors = [];
  page.on('request', request => requests.push(request.url()));
  page.on('pageerror', error => errors.push(String(error)));
  await page.goto(url.href, { waitUntil: 'networkidle0' });
  await page.waitForFunction(() => document.querySelector('#studio-root')?.dataset.state === 'idle');
  const waitState = state => page.waitForFunction(expected =>
    document.querySelector('#studio-root')?.dataset.state === expected, {}, state);
  async function upload(relativePath, state = 'ready') {
    await (await page.$('#v2-file')).uploadFile(path.join(root, relativePath));
    await waitState(state);
  }
  const text = () => page.$$eval('#v2-page-host svg text', els => els.map(el => el.textContent).join(''));
  const control = 'tests/fixtures/issue7353_host_shape_slots/saved.hwp';
  await upload(control);
  assert.equal(await page.$eval('#studio-root', el => el.dataset.renderEngine), 'table_v2_hosted_section');
  assert.equal(await page.$$eval('#v2-page-host svg', els => els.length), 1);
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ path: path.join(output, 'approved-control.png') });

  await upload('samples/hwpx/form-002.hwpx', 'error');
  assert.match(await page.$eval('#v2-error', el => el.textContent), /body page-number declaration with table controls/);
  assert.equal(await page.$$eval('#v2-page-host svg', els => els.length), 0);
  await page.screenshot({ path: path.join(output, 'unsupported-input.png') });
  await upload(control);
  // Same file selection must work too (the input value is reset after reading).
  await upload(control);

  await page.$eval('#v2-section', el => { el.value = '999'; });
  await page.click('#v2-reopen');
  await waitState('error');
  assert.equal(await page.$$eval('#v2-page-host svg', els => els.length), 0);
  await page.$eval('#v2-section', el => { el.value = '1'; });
  await page.click('#v2-reopen');
  await waitState('ready');

  // Hancom-saved two-page fixture: independent expected page membership in its README.
  await upload('tests/fixtures/issue7353_body_frame_review/portrait-saved.hwp');
  assert.equal(await page.$eval('#v2-page', el => el.max), '2');
  assert.match(await text(), /BEFORE/);
  assert.doesNotMatch(await text(), /DELTA/);
  await page.click('#v2-next');
  assert.match(await text(), /DELTA/);
  assert.match(await text(), /AFTER/);
  assert.doesNotMatch(await text(), /BEFORE/);
  assert.equal(await page.$eval('#v2-next', el => el.disabled), true);
  await page.screenshot({ path: path.join(output, 'page-two.png') });
  await page.click('#v2-prev');
  assert.match(await text(), /BEFORE/);
  assert.equal(await page.$eval('#v2-prev', el => el.disabled), true);
  assert.equal(await page.evaluate(() => typeof window.__wasm), 'undefined');
  assert.equal(requests.some(value => /\/src\/(main\.ts|core\/wasm-bridge\.ts)(\?|$)/.test(value)), false);
  assert.deepEqual(errors, []);

  // Explicit Legacy selection must still initialize the original editor.
  url.searchParams.set('typeset', 'legacy');
  await page.goto(url.href, { waitUntil: 'networkidle0' });
  await page.waitForFunction(() => Boolean(window.__wasm && window.__canvasView), { timeout: 60000 });
  assert.equal(await page.$('#v2-file'), null);
  assert.deepEqual(errors, []);
  await writeFile(path.join(output, 'result.json'), JSON.stringify({
    passed: true, browser: await browser.version(),
    checks: ['V2 upload and engine', 'unsupported input without fallback', 'reload and same-file retry',
      'invalid section recovery', 'two-page navigation and content ownership', 'no Legacy bootstrap in V2',
      'explicit Legacy editor bootstrap'],
  }, null, 2));
  console.log('PASS: Studio V2 upload, rejection, recovery, page navigation and Legacy isolation');
} finally {
  await closeBrowser(browser);
}

import init, { HostedSectionV2 } from '@wasm/rhwp.js';
import { TableV2Session, splitSvgFontFamilies } from './core/table-v2-session.ts';
import { loadWebFonts } from './core/font-loader.ts';
import { appendSvgMarkup } from './ui/dom-utils.ts';
import './styles/table-v2-studio.css';

export async function startTableV2Studio(): Promise<void> {
  const root = document.getElementById('studio-root')!;
  root.className = 'v2-studio';
  root.dataset.typesetEngine = 'v2';
  document.title = 'rhwp-studio — V2 읽기 모드';
  root.innerHTML = `
    <header id="studio-header"><strong>rhwp-studio · V2 WASM</strong>
      <span>읽기 전용 · 편집/저장 미연결 · Legacy 자동 우회 없음</span></header>
    <div id="icon-toolbar">
      <label>문서 열기 <input id="v2-file" type="file" accept=".hwp,.hwpx" disabled></label>
      <label>구역 <input id="v2-section" type="number" min="1" step="1" value="1"></label>
      <button id="v2-reopen" disabled>선택 구역 열기</button>
      <button id="v2-prev" disabled>이전 쪽</button>
      <label>구역 내 쪽 <input id="v2-page" type="number" min="1" step="1" value="1" disabled></label>
      <button id="v2-next" disabled>다음 쪽</button>
      <label>배율 <select id="v2-zoom"><option value="0.75">75%</option>
        <option value="1" selected>100%</option><option value="1.25">125%</option>
        <option value="1.5">150%</option></select></label>
    </div>
    <p class="v2-notice">선택한 구역만 V2로 조판합니다. 쪽 번호는 전체 문서의 쪽 번호가 아닙니다.
      미지원 문서는 오류를 표시하며, 일부 내용만 생략해 보여주지 않습니다.</p>
    <pre id="v2-error" role="alert" hidden></pre>
    <main id="scroll-container" aria-label="V2 문서 읽기 영역"><div id="v2-page-host"></div></main>
    <footer id="status-bar" role="status" aria-live="polite">V2 WASM 로딩 중…</footer>`;
  const file = document.getElementById('v2-file') as HTMLInputElement;
  const section = document.getElementById('v2-section') as HTMLInputElement;
  const pageInput = document.getElementById('v2-page') as HTMLInputElement;
  const previous = document.getElementById('v2-prev') as HTMLButtonElement;
  const next = document.getElementById('v2-next') as HTMLButtonElement;
  const reopen = document.getElementById('v2-reopen') as HTMLButtonElement;
  const zoom = document.getElementById('v2-zoom') as HTMLSelectElement;
  const host = document.getElementById('v2-page-host')!;
  const status = document.getElementById('status-bar')!;
  const errorBox = document.getElementById('v2-error')!;
  const session = new TableV2Session((bytes, options) => new HostedSectionV2(bytes, options));
  let source: Uint8Array | null = null;
  let name = '';
  let selectedSection = 0;
  let pageIndex = 0;
  let request = 0;

  function navigation(): void {
    pageInput.disabled = session.pageCount === 0;
    pageInput.max = String(session.pageCount);
    pageInput.value = String(pageIndex + 1);
    previous.disabled = session.pageCount === 0 || pageIndex === 0;
    next.disabled = session.pageCount === 0 || pageIndex + 1 >= session.pageCount;
  }
  function fail(error: unknown): void {
    session.close();
    delete root.dataset.renderEngine;
    host.replaceChildren();
    navigation();
    errorBox.hidden = false;
    errorBox.textContent = `V2 조판 실패 — Legacy로 전환하지 않았습니다.\n${String(error)}`;
    status.textContent = `${name || '문서'} · V2 출력 없음`;
    root.dataset.state = 'error';
  }
  function paint(index: number): void {
    host.replaceChildren();
    const svg = session.render(index);
    appendSvgMarkup(host, svg);
    const element = host.querySelector('svg');
    if (!element) throw Error('V2 SVG를 표시할 수 없습니다.');
    pageIndex = index;
    host.style.zoom = zoom.value;
    navigation();
    status.textContent = `${name} · V2 · 구역 ${selectedSection + 1} · ${pageIndex + 1} / ${session.pageCount}쪽`;
    root.dataset.state = 'ready';
    root.dataset.renderEngine = 'table_v2_hosted_section';
    const fonts = [...new Set(Array.from(element.querySelectorAll('[font-family]'),
      node => splitSvgFontFamilies(node.getAttribute('font-family')!)).flat())];
    void loadWebFonts(fonts, undefined, {
      disableExternalWebFonts: __RHWP_DISABLE_EXTERNAL_WEBFONTS__,
    }).catch(error => console.warn('V2 웹폰트 공급 실패', error));
  }
  function open(): void {
    if (!source) return;
    errorBox.hidden = true;
    host.replaceChildren();
    root.dataset.state = 'loading';
    try {
      selectedSection = section.valueAsNumber - 1;
      session.open(source, selectedSection);
      paint(0);
      document.title = `${name} — rhwp-studio V2 읽기 모드`;
    } catch (error) { fail(error); }
  }
  try {
    await init();
    if (typeof HostedSectionV2 !== 'function') throw Error('이 WASM에는 HostedSectionV2가 없습니다.');
    file.disabled = false;
    root.dataset.state = 'idle';
    status.textContent = 'V2 WASM 준비 완료 · HWP/HWPX 파일을 여세요.';
  } catch (error) { fail(error); return; }

  file.addEventListener('change', async () => {
    const selected = file.files?.[0];
    if (!selected) return;
    const ticket = ++request;
    source = null;
    session.close();
    host.replaceChildren();
    navigation();
    errorBox.hidden = true;
    reopen.disabled = true;
    name = selected.name;
    status.textContent = `${name} · 파일 읽는 중…`;
    root.dataset.state = 'loading';
    try {
      const bytes = new Uint8Array(await selected.arrayBuffer());
      if (ticket !== request) return;
      source = bytes;
      section.value = '1';
      reopen.disabled = false;
      open();
    } catch (error) { if (ticket === request) fail(error); }
    finally { if (ticket === request) file.value = ''; }
  });
  reopen.addEventListener('click', open);
  function turn(index: number): void {
    try { paint(index); } catch (error) { fail(error); }
  }
  previous.addEventListener('click', () => turn(pageIndex - 1));
  next.addEventListener('click', () => turn(pageIndex + 1));
  pageInput.addEventListener('change', () => turn(pageInput.valueAsNumber - 1));
  zoom.addEventListener('change', () => { host.style.zoom = zoom.value; });
  window.addEventListener('pagehide', event => { if (!event.persisted) session.close(); });
}

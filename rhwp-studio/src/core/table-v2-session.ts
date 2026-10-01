/** Read-only section preview. No DocumentCore/Legacy fallback or editing API. */
export interface HostedV2Handle {
  pageCount(): number;
  renderPage(index: number): string;
  free(): void;
}

export type HostedV2Factory = (bytes: Uint8Array, options: string) => HostedV2Handle;

/** SVG output supplies a CSS fallback list; the shared font loader needs individual face names. */
export function splitSvgFontFamilies(value: string): string[] {
  return Array.from(value.matchAll(/"([^"]+)"|'([^']+)'|([^,]+)/g),
    match => (match[1] ?? match[2] ?? match[3]).trim()).filter(Boolean);
}

export function resolveTypesetMode(search: string, mode: string): 'legacy' | 'v2' {
  const selected = new URLSearchParams(search).get('typeset');
  if (selected === 'v2' || selected === 'legacy') return selected;
  return 'v2';
}

/** The diagnostic SVG preview is separate from the normal V2 editor. */
export function isTableV2Preview(search: string, mode: string): boolean {
  const selected = new URLSearchParams(search).get('typeset');
  return selected === 'preview' || (!selected && mode === 'table-v2');
}

export class TableV2Session {
  private handle: HostedV2Handle | null = null;
  pageCount = 0;

  private readonly create: HostedV2Factory;

  constructor(create: HostedV2Factory) { this.create = create; }

  close(): void {
    this.handle?.free();
    this.handle = null;
    this.pageCount = 0;
  }

  open(bytes: Uint8Array, section: number): void {
    this.close();
    if (!Number.isSafeInteger(section) || section < 0) throw Error('구역 번호가 올바르지 않습니다.');
    try {
      this.handle = this.create(bytes, JSON.stringify({
        section, dpi: 96, cell_end_policy: 'omit_final_paragraph_gap',
      }));
      this.pageCount = this.handle.pageCount();
      if (!Number.isSafeInteger(this.pageCount) || this.pageCount < 1) {
        throw Error('V2가 표시할 쪽을 반환하지 않았습니다.');
      }
    } catch (error) {
      this.close();
      throw error;
    }
  }

  render(index: number): string {
    if (!this.handle) throw Error('V2 문서가 열려 있지 않습니다.');
    if (!Number.isSafeInteger(index) || index < 0 || index >= this.pageCount) {
      throw Error('쪽 번호가 범위를 벗어났습니다.');
    }
    const packet = JSON.parse(this.handle.renderPage(index));
    if (packet.engine !== 'table_v2_hosted_section' || packet.page_index !== index
        || typeof packet.svg !== 'string' || !packet.svg.trimStart().startsWith('<svg')) {
      throw Error('V2 출력 계약이 일치하지 않습니다. Legacy 출력으로 대체하지 않습니다.');
    }
    return packet.svg;
  }
}

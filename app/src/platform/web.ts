// 브라우저·자동 테스트용 Platform: 메모리 가상 파일 시스템 + 앱 안 한국어 대화상자.
import type { FileFilter, ImageEntry, Platform } from './platform';
import { normPath, VirtualFs } from './vfs';

/** 앱 안 대화상자(화면 쪽이 구현). 가상 파일 목록에서 고르거나 이름을 입력한다. */
export interface WebDialogProvider {
  openFile(title: string, files: string[]): Promise<string | null>;
  saveFile(title: string, defaultPath: string, files: string[]): Promise<string | null>;
  pickFolder(title: string, folders: string[], defaultPath: string): Promise<string | null>;
}

/** 시험 이미지가 놓이는 가상 폴더 */
export const SAMPLE_DIR = '/samples';

const sampleImages = import.meta.glob('../../../samples/images/*.png', { query: '?url', import: 'default', eager: true }) as Record<string, string>;
const sampleTexts = import.meta.glob(['../../../samples/kgbone/*.kgbone', '../../../samples/spine4*/*.json', '../../../samples/spine4*/*.atlas'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function matchesFilter(path: string, filters: FileFilter[]): boolean {
  if (!filters.length) return true;
  const ext = path.slice(path.lastIndexOf('.') + 1).toLowerCase();
  return filters.some((f) => f.extensions.includes('*') || f.extensions.map((e) => e.toLowerCase()).includes(ext));
}

function imageSize(url: string): Promise<{ width: number; height: number }> {
  return new Promise((resolve) => {
    if (typeof Image === 'undefined') return resolve({ width: 0, height: 0 });
    const img = new Image();
    img.onload = () => resolve({ width: img.naturalWidth, height: img.naturalHeight });
    img.onerror = () => resolve({ width: 0, height: 0 });
    img.src = url;
  });
}

export class WebPlatform implements Platform {
  readonly kind = 'web' as const;
  readonly fs = new VirtualFs();
  private closeCb: (() => Promise<boolean>) | null = null;

  constructor(private dialogs: WebDialogProvider) {
    for (const [src, url] of Object.entries(sampleImages)) this.fs.addUrl(`${SAMPLE_DIR}/images/${src.slice(src.lastIndexOf('/') + 1)}`, url);
    for (const [src, text] of Object.entries(sampleTexts)) {
      const m = /samples\/(.+)$/.exec(src);
      if (m) this.fs.writeText(`${SAMPLE_DIR}/${m[1]}`, text);
    }
    this.fs.mkdir(`${SAMPLE_DIR}/export`);
  }

  openFileDialog(title: string, filters: FileFilter[]): Promise<string | null> {
    return this.dialogs.openFile(title, this.fs.list('/').filter((p) => matchesFilter(p, filters)));
  }
  saveFileDialog(title: string, defaultName: string, filters: FileFilter[]): Promise<string | null> {
    const def = defaultName.startsWith('/') ? defaultName : `${SAMPLE_DIR}/${defaultName}`;
    return this.dialogs.saveFile(title, def, this.fs.list('/').filter((p) => matchesFilter(p, filters)));
  }
  pickFolder(title: string): Promise<string | null> {
    return this.dialogs.pickFolder(title, this.fs.listDirs(), SAMPLE_DIR);
  }
  readText(path: string): Promise<string> {
    return this.fs.readText(path);
  }
  async writeText(path: string, text: string): Promise<void> {
    this.fs.writeText(path, text);
  }
  readBinary(path: string): Promise<Uint8Array> {
    return this.fs.readBinary(path);
  }
  async writeBinary(path: string, data: Uint8Array): Promise<void> {
    this.fs.writeBinary(path, data);
  }
  async listImages(dir: string): Promise<ImageEntry[]> {
    const d = normPath(dir);
    const out: ImageEntry[] = [];
    for (const p of this.fs.list(d)) {
      if (!p.toLowerCase().endsWith('.png')) continue;
      const url = this.fs.urlOf(p);
      if (!url) continue;
      const rel = p.slice(d === '/' ? 1 : d.length + 1);
      const { width, height } = await imageSize(url);
      out.push({ name: rel.replace(/\.png$/i, ''), url, width, height });
    }
    return out;
  }
  async watchFolder(dir: string, cb: () => void): Promise<() => void> {
    return this.fs.watch(dir, cb);
  }
  async registerAssociation(): Promise<string> {
    return '브라우저에서는 지원하지 않습니다';
  }
  async launchFile(): Promise<string | null> {
    const q = typeof location !== 'undefined' ? new URLSearchParams(location.search).get('file') : null;
    return q && this.fs.exists(q) ? q : null;
  }
  onCloseRequested(cb: () => Promise<boolean>): void {
    this.closeCb = cb;
  }
  setTitle(title: string): void {
    if (typeof document !== 'undefined') document.title = title;
  }
  async quit(): Promise<void> {
    if (this.closeCb && !(await this.closeCb())) return;
    if (typeof window !== 'undefined') window.close();
  }
}

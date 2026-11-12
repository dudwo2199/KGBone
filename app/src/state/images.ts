// 이미지 폴더: 목록 불러오기, 크기·알파를 핵심에 등록, 폴더 자동 갱신.
import type { Core } from '../core/bridge';
import type { ImageEntry, Platform } from '../platform/platform';
import { Store } from './store';

export interface ImagesState {
  dir: string;
  entries: ImageEntry[];
  /** 다시 불러올 때마다 는다(텍스처 캐시 비우기) */
  version: number;
  loading: boolean;
}

/** 이미지 URL → 알파 바이트(가로×세로). 브라우저에서는 캔버스로 읽는다 */
export type AlphaReader = (url: string, width: number, height: number) => Promise<Uint8Array | undefined>;

export const canvasAlphaReader: AlphaReader = (url, width, height) =>
  new Promise((resolve) => {
    if (typeof document === 'undefined' || !width || !height) return resolve(undefined);
    const img = new Image();
    img.onload = () => {
      try {
        const c = document.createElement('canvas');
        c.width = width;
        c.height = height;
        const g = c.getContext('2d');
        if (!g) return resolve(undefined);
        g.drawImage(img, 0, 0);
        const d = g.getImageData(0, 0, width, height).data;
        const a = new Uint8Array(width * height);
        for (let i = 0; i < a.length; i++) a[i] = d[i * 4 + 3];
        resolve(a);
      } catch {
        resolve(undefined);
      }
    };
    img.onerror = () => resolve(undefined);
    img.src = url;
  });

export class ImageLibrary extends Store<ImagesState> {
  private unwatch: (() => void) | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private platform: Platform,
    private core: Core,
    private notify: (level: 'info' | 'warn' | 'error', text: string) => void,
    private readAlpha: AlphaReader = canvasAlphaReader,
  ) {
    super({ dir: '', entries: [], version: 0, loading: false });
  }

  byName(name: string): ImageEntry | undefined {
    return this.get().entries.find((e) => e.name === name);
  }

  /** 폴더를 (다시) 불러오고 자동 갱신을 건다 */
  async open(dir: string): Promise<void> {
    if (dir !== this.get().dir || !this.unwatch) {
      this.unwatch?.();
      this.unwatch = null;
      try {
        this.unwatch = await this.platform.watchFolder(dir, () => this.scheduleReload());
      } catch {
        // 지켜보기를 못 해도 목록은 불러온다
      }
    }
    this.set({ dir });
    await this.reload();
  }

  private scheduleReload(): void {
    if (this.timer) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.reload();
    }, 150);
  }

  async reload(): Promise<void> {
    const dir = this.get().dir;
    this.set({ loading: true });
    let entries: ImageEntry[] = [];
    try {
      entries = await this.platform.listImages(dir);
    } catch (e) {
      this.notify('warn', `이미지 폴더를 읽지 못했습니다: ${dir} (${e instanceof Error ? e.message : String(e)})`);
    }
    for (const e of entries) {
      try {
        const alpha = await this.readAlpha(e.url, e.width, e.height);
        this.core.setImage(e.name, e.width, e.height, alpha);
      } catch {
        // 핵심이 아직 받지 못해도 화면 표시는 계속한다
      }
    }
    this.set((s) => ({ entries, loading: false, version: s.version + 1 }));
  }

  dispose(): void {
    this.unwatch?.();
    this.unwatch = null;
  }
}

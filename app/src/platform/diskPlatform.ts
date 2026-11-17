// 'disk:' 경로(AI 연결로 연 이 PC의 실제 파일)를 처리하는 Platform 감싸개.
// 브라우저 판은 디스크를 직접 못 읽으므로 AI 연결 중계소(tools/mcp/link.mjs)를 거친다. Tauri 판은 앞머리만 떼고 바로 읽는다.
// 그 밖의 경로(메모리 '/…', File System Access 'fsa:/…')는 원래 Platform이 처리한다.
import { base64ToBytes, bytesToBase64 } from '../files/base64';
import type { FileFilter, ImageEntry, Platform } from './platform';

export const DISK_PREFIX = 'disk:';

export const isDiskPath = (p: string): boolean => p.startsWith(DISK_PREFIX);

/** 중계소 쪽 파일 작업(AgentLink.fs / watch) */
export interface DiskBridge {
  fs<T = unknown>(op: string, args: Record<string, unknown>): Promise<T>;
  watch(dir: string, cb: () => void): () => void;
}

interface DiskImage {
  name: string;
  width: number;
  height: number;
  base64: string;
}

export class DiskPlatform implements Platform {
  /** 폴더별로 만든 그림 URL(다시 읽을 때 정리) */
  private urls = new Map<string, string[]>();

  constructor(
    readonly base: Platform,
    private disk: DiskBridge,
  ) {}

  get kind(): Platform['kind'] {
    return this.base.kind;
  }

  /** Tauri는 디스크를 직접 읽는다 */
  private direct(p: string): string | null {
    return this.base.kind === 'tauri' ? p.slice(DISK_PREFIX.length) : null;
  }

  openFileDialog(title: string, filters: FileFilter[]): Promise<string | null> {
    return this.base.openFileDialog(title, filters);
  }
  saveFileDialog(title: string, defaultName: string, filters: FileFilter[]): Promise<string | null> {
    // 디스크 파일을 다른 이름으로 저장할 때 브라우저 대화상자에는 이름만 넘긴다
    const name = isDiskPath(defaultName) && this.base.kind !== 'tauri' ? (defaultName.split(/[\\/]/).pop() ?? defaultName) : defaultName;
    return this.base.saveFileDialog(title, this.direct(name) ?? name, filters);
  }
  pickFolder(title: string): Promise<string | null> {
    return this.base.pickFolder(title);
  }

  readText(path: string): Promise<string> {
    if (!isDiskPath(path)) return this.base.readText(path);
    const d = this.direct(path);
    return d ? this.base.readText(d) : this.disk.fs<string>('readText', { path });
  }
  async writeText(path: string, text: string): Promise<void> {
    if (!isDiskPath(path)) return this.base.writeText(path, text);
    const d = this.direct(path);
    if (d) return this.base.writeText(d, text);
    await this.disk.fs('writeText', { path, text });
  }
  async readBinary(path: string): Promise<Uint8Array> {
    if (!isDiskPath(path)) return this.base.readBinary(path);
    const d = this.direct(path);
    return d ? this.base.readBinary(d) : base64ToBytes(await this.disk.fs<string>('readBinary', { path }));
  }
  async writeBinary(path: string, data: Uint8Array): Promise<void> {
    if (!isDiskPath(path)) return this.base.writeBinary(path, data);
    const d = this.direct(path);
    if (d) return this.base.writeBinary(d, data);
    await this.disk.fs('writeBinary', { path, base64: bytesToBase64(data) });
  }

  async listImages(dir: string): Promise<ImageEntry[]> {
    if (!isDiskPath(dir)) return this.base.listImages(dir);
    const d = this.direct(dir);
    if (d) return this.base.listImages(d);
    const list = await this.disk.fs<DiskImage[]>('listImages', { dir });
    for (const u of this.urls.get(dir) ?? []) URL.revokeObjectURL(u);
    const entries = list.map((e) => {
      const bytes = base64ToBytes(e.base64);
      const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: 'image/png' }));
      return { name: e.name, url, width: e.width, height: e.height };
    });
    this.urls.set(
      dir,
      entries.map((e) => e.url),
    );
    return entries;
  }
  async watchFolder(dir: string, cb: () => void): Promise<() => void> {
    if (!isDiskPath(dir)) return this.base.watchFolder(dir, cb);
    const d = this.direct(dir);
    return d ? this.base.watchFolder(d, cb) : this.disk.watch(dir, cb);
  }

  registerAssociation(): Promise<string> {
    return this.base.registerAssociation();
  }
  launchFile(): Promise<string | null> {
    return this.base.launchFile();
  }
  onCloseRequested(cb: () => Promise<boolean>): void {
    this.base.onCloseRequested(cb);
  }
  setTitle(title: string): void {
    this.base.setTitle(title);
  }
  quit(): Promise<void> {
    return this.base.quit();
  }
}

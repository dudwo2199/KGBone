// 브라우저(Edge 앱 창) 판 Platform: File System Access API로 실제 파일을 다룬다.
// 고른 폴더를 뿌리로 등록하고 "fsa:/<폴더이름>/<파일>" 가짜 경로로 Platform 인터페이스를 만족시킨다.
// 프로젝트 파일이 등록한 폴더 안에 있어야 imagesPath 상대 경로가 풀리므로, 폴더 밖 파일을 열면 그 폴더를 고르도록 묻는다.
import type { FileFilter, ImageEntry, Platform } from './platform';
import { HandleTable, isFsaPath, type DirLike, type FileLike } from './fsaTable';

/** 브라우저 판 대화상자(화면 쪽 구현): 사용자가 버튼을 눌러야 폴더 고르기 창을 띄울 수 있다 */
export interface BrowserFsDialogs {
  confirm(title: string, text: string, ok: string, cancel: string): Promise<boolean>;
}

type Dir = FileSystemDirectoryHandle & DirLike;
type Fil = FileSystemFileHandle & FileLike;

interface PickerWindow {
  showOpenFilePicker(o?: unknown): Promise<FileSystemFileHandle[]>;
  showSaveFilePicker(o?: unknown): Promise<FileSystemFileHandle>;
  showDirectoryPicker(o?: unknown): Promise<FileSystemDirectoryHandle>;
}
type PermHandle = FileSystemHandle & {
  queryPermission?(o: { mode: string }): Promise<PermissionState>;
  requestPermission?(o: { mode: string }): Promise<PermissionState>;
};

function pickerWindow(): PickerWindow {
  return window as unknown as PickerWindow;
}

function types(filters: FileFilter[]) {
  return filters.map((f) => ({ description: f.name, accept: { 'application/octet-stream': f.extensions.filter((e) => e !== '*').map((e) => `.${e}`) } }));
}

function isAbort(e: unknown): boolean {
  return e instanceof DOMException && e.name === 'AbortError';
}

async function ensurePermission(h: FileSystemHandle, write: boolean): Promise<void> {
  const p = h as PermHandle;
  const mode = write ? 'readwrite' : 'read';
  if (!p.queryPermission) return;
  if ((await p.queryPermission({ mode })) === 'granted') return;
  if ((await p.requestPermission?.({ mode })) !== 'granted') throw new Error(`파일 접근 권한이 없습니다: ${h.name}`);
}

function imageSize(url: string): Promise<{ width: number; height: number }> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve({ width: img.naturalWidth, height: img.naturalHeight });
    img.onerror = () => resolve({ width: 0, height: 0 });
    img.src = url;
  });
}

export function browserFsAvailable(): boolean {
  return typeof window !== 'undefined' && 'showOpenFilePicker' in window && 'showDirectoryPicker' in window;
}

export class BrowserFsPlatform implements Platform {
  readonly kind = 'web' as const;
  private table = new HandleTable<Dir, Fil>();
  private urls = new Map<string, string[]>();
  private dirty = false;

  constructor(private dialogs: BrowserFsDialogs) {}

  /** 파일 핸들 → 경로. 등록한 폴더 밖이면 폴더를 고르게 한다(이미지 폴더 상대 경로를 풀기 위해) */
  private async pathOf(f: Fil): Promise<string> {
    const found = await this.table.locate(f);
    if (found) return found;
    const ok = await this.dialogs.confirm(
      '프로젝트 폴더 고르기',
      `"${f.name}"이(가) 있는 폴더(또는 그 위 폴더)를 고르면 이미지 폴더를 함께 읽고 쓸 수 있습니다.`,
      '폴더 고르기',
      '파일만 쓰기',
    );
    if (ok) {
      try {
        const dir = (await pickerWindow().showDirectoryPicker({ mode: 'readwrite', startIn: f })) as Dir;
        await this.table.addRoot(dir);
        const again = await this.table.locate(f);
        if (again) return again;
      } catch (e) {
        if (!isAbort(e)) throw e;
      }
    }
    return this.table.addLoose(f);
  }

  async openFileDialog(_title: string, filters: FileFilter[]): Promise<string | null> {
    try {
      const [h] = await pickerWindow().showOpenFilePicker({ types: types(filters), startIn: this.table.lastRoot() });
      return h ? this.pathOf(h as Fil) : null;
    } catch (e) {
      if (isAbort(e)) return null;
      throw e;
    }
  }

  async saveFileDialog(_title: string, defaultName: string, filters: FileFilter[]): Promise<string | null> {
    try {
      const name = defaultName.split(/[\\/]/).pop() || defaultName;
      const h = await pickerWindow().showSaveFilePicker({ suggestedName: name, types: types(filters), startIn: this.table.lastRoot() });
      return this.pathOf(h as Fil);
    } catch (e) {
      if (isAbort(e)) return null;
      throw e;
    }
  }

  async pickFolder(): Promise<string | null> {
    try {
      const d = (await pickerWindow().showDirectoryPicker({ mode: 'readwrite', startIn: this.table.lastRoot() })) as Dir;
      return this.table.addRoot(d);
    } catch (e) {
      if (isAbort(e)) return null;
      throw e;
    }
  }

  private async readFile(path: string): Promise<File> {
    const h = await this.table.file(path);
    await ensurePermission(h, false);
    return h.getFile();
  }

  async readText(path: string): Promise<string> {
    return (await this.readFile(path)).text();
  }
  async readBinary(path: string): Promise<Uint8Array> {
    return new Uint8Array(await (await this.readFile(path)).arrayBuffer());
  }
  private async write(path: string, data: string | Uint8Array): Promise<void> {
    const h = await this.table.file(path, true);
    await ensurePermission(h, true);
    const w = await h.createWritable();
    await w.write(typeof data === 'string' ? data : (data.slice().buffer as ArrayBuffer));
    await w.close();
  }
  writeText(path: string, text: string): Promise<void> {
    return this.write(path, text);
  }
  writeBinary(path: string, data: Uint8Array): Promise<void> {
    return this.write(path, data);
  }

  /** dir 아래 PNG(하위 폴더 포함): [상대 경로, 파일] */
  private async pngFiles(dir: string): Promise<[string, File][]> {
    if (!isFsaPath(dir)) return [];
    const root = await this.table.dir(dir);
    await ensurePermission(root, false);
    const out: [string, File][] = [];
    const walk = async (d: FileSystemDirectoryHandle, prefix: string) => {
      for await (const h of (d as unknown as { values(): AsyncIterable<FileSystemHandle> }).values()) {
        if (h.kind === 'directory') await walk(h as FileSystemDirectoryHandle, `${prefix}${h.name}/`);
        else if (h.name.toLowerCase().endsWith('.png')) out.push([`${prefix}${h.name}`, await (h as FileSystemFileHandle).getFile()]);
      }
    };
    await walk(root, '');
    return out.sort((a, b) => a[0].localeCompare(b[0]));
  }

  async listImages(dir: string): Promise<ImageEntry[]> {
    const files = await this.pngFiles(dir);
    for (const u of this.urls.get(dir) ?? []) URL.revokeObjectURL(u);
    const urls: string[] = [];
    const out: ImageEntry[] = [];
    for (const [rel, file] of files) {
      const url = URL.createObjectURL(file);
      urls.push(url);
      const { width, height } = await imageSize(url);
      out.push({ name: rel.replace(/\.png$/i, ''), url, width, height });
    }
    this.urls.set(dir, urls);
    return out;
  }

  /** 2초마다 이름·크기·수정 시각을 비교한다 */
  async watchFolder(dir: string, cb: () => void): Promise<() => void> {
    if (!isFsaPath(dir)) return () => {};
    const sig = async () => {
      try {
        return (await this.pngFiles(dir)).map(([n, f]) => `${n}:${f.size}:${f.lastModified}`).join('|');
      } catch {
        return '';
      }
    };
    let last = await sig();
    let busy = false;
    const id = setInterval(async () => {
      if (busy) return;
      busy = true;
      const now = await sig();
      busy = false;
      if (now !== last) {
        last = now;
        cb();
      }
    }, 2000);
    return () => clearInterval(id);
  }

  async registerAssociation(): Promise<string> {
    return '브라우저 판에서는 지원하지 않습니다';
  }
  async launchFile(): Promise<string | null> {
    return null;
  }
  onCloseRequested(): void {
    // 브라우저는 닫기 전에 비동기로 물을 수 없으므로, 저장 안 한 변경이 있으면 브라우저 기본 확인창을 띄운다
    window.addEventListener('beforeunload', (e) => {
      if (this.dirty) {
        e.preventDefault();
        e.returnValue = '';
      }
    });
  }
  setTitle(title: string): void {
    // 화면은 저장 안 한 변경이 있으면 제목 끝에 '*'를 붙인다
    this.dirty = title.endsWith('*');
    document.title = title;
  }
  async quit(): Promise<void> {
    window.close();
  }
}

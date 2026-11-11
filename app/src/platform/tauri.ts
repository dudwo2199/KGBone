// Tauri 앱 본체용 Platform 구현. 명령 이름·인자는 interface/tauri-commands.md와 1:1(src-tauri/src/commands/*.rs).
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { FileFilter, ImageEntry, Platform } from './platform';

/** kg_list_images 결과 한 개 */
interface ImageFile {
  name: string;
  path: string;
  width: number;
  height: number;
}

/** kg://folder-changed 이벤트 내용 */
interface FolderChanged {
  dir: string;
}

const FOLDER_CHANGED = 'kg://folder-changed';

/** 바이트 → base64(표준 알파벳, = 채움). 큰 파일에서 호출 스택이 넘치지 않게 나눠서 바꾼다. */
function toBase64(data: Uint8Array): string {
  const CHUNK = 0x8000;
  let binary = '';
  for (let i = 0; i < data.length; i += CHUNK) {
    binary += String.fromCharCode(...data.subarray(i, i + CHUNK));
  }
  return btoa(binary);
}

/** 대화상자 필터: 확장자 앞의 점은 뗀다 */
function dialogFilters(filters: FileFilter[]) {
  return filters.map((f) => ({ name: f.name, extensions: f.extensions.map((e) => e.replace(/^\./, '')) }));
}

/** 한 개만 고르는 대화상자 결과를 경로 하나로 */
function single(result: string | string[] | null): string | null {
  if (result === null) return null;
  return Array.isArray(result) ? (result[0] ?? null) : result;
}

/** 경로 비교용(윈도우: 대소문자·구분자 무시) */
function samePath(a: string, b: string): boolean {
  const norm = (p: string) => p.replace(/[\\/]+/g, '/').replace(/\/$/, '').toLowerCase();
  return norm(a) === norm(b);
}

/** 오류를 화면에 보여 줄 Error로(명령은 한국어 문자열로 거부한다) */
function asError(e: unknown): Error {
  return e instanceof Error ? e : new Error(String(e));
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw asError(e);
  }
}

class TauriPlatform implements Platform {
  readonly kind = 'tauri' as const;

  async openFileDialog(title: string, filters: FileFilter[]): Promise<string | null> {
    const result = await open({ title, filters: dialogFilters(filters), multiple: false, directory: false });
    return single(result);
  }

  async saveFileDialog(title: string, defaultName: string, filters: FileFilter[]): Promise<string | null> {
    return save({ title, defaultPath: defaultName, filters: dialogFilters(filters) });
  }

  async pickFolder(title: string): Promise<string | null> {
    const result = await open({ title, directory: true, multiple: false });
    return single(result);
  }

  readText(path: string): Promise<string> {
    return call<string>('kg_read_text', { path });
  }

  writeText(path: string, text: string): Promise<void> {
    return call<void>('kg_write_text', { path, text });
  }

  async readBinary(path: string): Promise<Uint8Array> {
    // raw 응답(tauri::ipc::Response)은 ArrayBuffer로 온다
    const buf = await call<ArrayBuffer | number[]>('kg_read_binary', { path });
    return buf instanceof ArrayBuffer ? new Uint8Array(buf) : Uint8Array.from(buf);
  }

  writeBinary(path: string, data: Uint8Array): Promise<void> {
    return call<void>('kg_write_binary', { path, base64: toBase64(data) });
  }

  async listImages(dir: string): Promise<ImageEntry[]> {
    const files = await call<ImageFile[]>('kg_list_images', { dir });
    return files.map((f) => ({ name: f.name, url: convertFileSrc(f.path), width: f.width, height: f.height }));
  }

  async watchFolder(dir: string, cb: () => void): Promise<() => void> {
    const unlisten = await listen<FolderChanged>(FOLDER_CHANGED, (e) => {
      if (samePath(e.payload.dir, dir)) cb();
    });
    let id: number;
    try {
      id = await call<number>('kg_watch_folder', { dir });
    } catch (e) {
      unlisten();
      throw e;
    }
    let stopped = false;
    return () => {
      if (stopped) return;
      stopped = true;
      unlisten();
      void call<void>('kg_unwatch_folder', { id }).catch(() => undefined);
    };
  }

  registerAssociation(): Promise<string> {
    return call<string>('kg_register_association');
  }

  launchFile(): Promise<string | null> {
    return call<string | null>('kg_launch_file');
  }

  onCloseRequested(cb: () => Promise<boolean>): void {
    // 처리기가 끝날 때까지 기다린 뒤, 막지 않았으면 Tauri가 창을 닫는다(destroy)
    void getCurrentWindow().onCloseRequested(async (e) => {
      let ok = false;
      try {
        ok = await cb();
      } catch {
        ok = false;
      }
      if (!ok) e.preventDefault();
    });
  }

  setTitle(title: string): void {
    void getCurrentWindow().setTitle(title).catch(() => undefined);
  }

  async quit(): Promise<void> {
    // destroy는 close-requested 이벤트를 거치지 않으므로 확인 창이 다시 뜨지 않는다
    await getCurrentWindow().destroy();
  }
}

export async function createTauriPlatform(): Promise<Platform> {
  return new TauriPlatform();
}

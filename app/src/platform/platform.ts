// 앱 본체(파일 처리) 인터페이스. Tauri 앱에서는 TauriPlatform(src-tauri의 명령), 브라우저·자동 테스트에서는 WebPlatform(메모리 파일).
// Tauri 명령 이름·인자는 interface/tauri-commands.md와 1:1.

export interface FileFilter {
  name: string;
  extensions: string[];
}

export interface ImageEntry {
  /** imagesPath 기준 상대 경로, 확장자 없음, '/' 구분 (첨부물 path와 같은 값) */
  name: string;
  /** <img>/텍스처로 불러올 수 있는 URL */
  url: string;
  width: number;
  height: number;
}

export interface ExportReport {
  /** 쓴 파일 전체 경로 */
  files: string[];
  warnings: { code: string; message: string; path: string }[];
}

export interface Platform {
  readonly kind: 'tauri' | 'web';
  /** 파일 열기 대화상자. 취소하면 null */
  openFileDialog(title: string, filters: FileFilter[]): Promise<string | null>;
  /** 저장 대화상자. defaultName의 확장자가 기본 확장자 */
  saveFileDialog(title: string, defaultName: string, filters: FileFilter[]): Promise<string | null>;
  pickFolder(title: string): Promise<string | null>;
  readText(path: string): Promise<string>;
  writeText(path: string, text: string): Promise<void>;
  readBinary(path: string): Promise<Uint8Array>;
  writeBinary(path: string, data: Uint8Array): Promise<void>;
  /** 폴더 안 PNG 목록(하위 폴더 포함) */
  listImages(dir: string): Promise<ImageEntry[]>;
  /** 폴더가 바뀌면 cb를 부른다. 돌려준 함수로 그만 지켜본다 */
  watchFolder(dir: string, cb: () => void): Promise<() => void>;
  /** .kgbone 파일 연결 등록(현재 사용자). 결과 메시지 */
  registerAssociation(): Promise<string>;
  /** 실행 인자(더블클릭·끌어 놓기)로 받은 파일 경로 */
  launchFile(): Promise<string | null>;
  /** 창을 닫으려 할 때. cb가 false면 닫지 않는다 */
  onCloseRequested(cb: () => Promise<boolean>): void;
  setTitle(title: string): void;
  quit(): Promise<void>;
}

/** 경로 도우미(윈도우 '\\'와 '/' 모두 처리) */
export const paths = {
  dirname(p: string): string {
    const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
    return i < 0 ? '' : p.slice(0, i);
  },
  basename(p: string): string {
    const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
    return p.slice(i + 1);
  },
  stem(p: string): string {
    const b = paths.basename(p);
    const i = b.lastIndexOf('.');
    return i <= 0 ? b : b.slice(0, i);
  },
  join(dir: string, rel: string): string {
    if (!dir) return rel;
    if (/^([a-zA-Z]:[\\/]|[\\/])/.test(rel)) return rel;
    const sep = dir.includes('\\') ? '\\' : '/';
    const parts = (dir.replace(/[\\/]+$/, '') + sep + rel.replace(/^\.[\\/]/, '')).split(/[\\/]/);
    const out: string[] = [];
    for (const s of parts) {
      if (s === '..' && out.length && out[out.length - 1] !== '..') out.pop();
      else if (s !== '.') out.push(s);
    }
    return out.join(sep);
  },
};

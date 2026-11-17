// 앱 안 대화상자 관리: 약속(Promise)으로 열고, DialogHost가 맨 위 것을 그린다.
import type { AppDialogs, PngSequenceChoice, SaveChoice, SpineExportChoice } from '../../files/dialogs';
import type { BrowserFsDialogs } from '../../platform/browser-fs';
import type { WebDialogProvider } from '../../platform/web';
import { Store } from '../../state/store';

export type DialogSpec =
  | { kind: 'confirmSave' }
  | { kind: 'prompt'; title: string; label: string; initial: string }
  | { kind: 'message'; title: string; text: string }
  | { kind: 'report'; title: string; lines: string[]; warnings: string[] }
  | { kind: 'exportSpine'; defaults: SpineExportChoice }
  | { kind: 'pngSequence'; defaults: PngSequenceChoice; animations: string[] }
  | { kind: 'pickImage'; names: string[] }
  | { kind: 'file'; mode: 'open' | 'save' | 'folder'; title: string; files: string[]; defaultPath: string }
  | { kind: 'shortcuts' }
  | { kind: 'aiConnect' }
  | { kind: 'confirm'; title: string; text: string; ok: string; cancel: string };

export interface OpenDialog {
  id: number;
  spec: DialogSpec;
  resolve(v: unknown): void;
}

export class DialogService extends Store<{ stack: OpenDialog[] }> implements AppDialogs, WebDialogProvider, BrowserFsDialogs {
  private nextId = 1;
  constructor() {
    super({ stack: [] });
  }

  open<T>(spec: DialogSpec): Promise<T> {
    return new Promise<T>((resolve) => {
      const d: OpenDialog = {
        id: this.nextId++,
        spec,
        resolve: (v) => {
          this.set((s) => ({ stack: s.stack.filter((x) => x !== d) }));
          resolve(v as T);
        },
      };
      this.set((s) => ({ stack: [...s.stack, d] }));
    });
  }

  /** 대화상자를 취소한 것처럼 닫는다(각 종류의 '취소' 값으로 끝냄) */
  cancel(d: OpenDialog): void {
    const k = d.spec.kind;
    d.resolve(k === 'confirmSave' ? 'cancel' : k === 'confirm' ? false : k === 'message' || k === 'report' || k === 'shortcuts' || k === 'aiConnect' ? undefined : null);
  }

  /** 대화상자 제목(알림용) */
  static titleOf(spec: DialogSpec): string {
    switch (spec.kind) {
      case 'confirmSave':
        return '저장 확인';
      case 'shortcuts':
        return '단축키 목록';
      case 'aiConnect':
        return 'MCP 연결';
      case 'pickImage':
        return '이미지 고르기';
      case 'exportSpine':
        return '내보내기';
      case 'pngSequence':
        return 'PNG 시퀀스';
      default:
        return (spec as { title?: string; kind: string }).title ?? (spec as { kind: string }).kind;
    }
  }

  isOpen(): boolean {
    return this.get().stack.length > 0;
  }

  confirmSave(): Promise<SaveChoice> {
    return this.open<SaveChoice>({ kind: 'confirmSave' });
  }
  prompt(title: string, label: string, initial: string): Promise<string | null> {
    return this.open({ kind: 'prompt', title, label, initial });
  }
  message(title: string, text: string): Promise<void> {
    return this.open({ kind: 'message', title, text });
  }
  report(title: string, lines: string[], warnings: string[]): Promise<void> {
    return this.open({ kind: 'report', title, lines, warnings });
  }
  exportSpine(defaults: SpineExportChoice): Promise<SpineExportChoice | null> {
    return this.open({ kind: 'exportSpine', defaults });
  }
  pngSequence(defaults: PngSequenceChoice, animations: string[]): Promise<PngSequenceChoice | null> {
    return this.open({ kind: 'pngSequence', defaults, animations });
  }
  pickImage(names: string[]): Promise<string | null> {
    return this.open({ kind: 'pickImage', names });
  }
  confirm(title: string, text: string, ok: string, cancel: string): Promise<boolean> {
    return this.open({ kind: 'confirm', title, text, ok, cancel });
  }
  shortcuts(): Promise<void> {
    return this.open({ kind: 'shortcuts' });
  }
  aiConnect(): Promise<void> {
    return this.open({ kind: 'aiConnect' });
  }

  // WebDialogProvider
  openFile(title: string, files: string[]): Promise<string | null> {
    return this.open({ kind: 'file', mode: 'open', title, files, defaultPath: '' });
  }
  saveFile(title: string, defaultPath: string, files: string[]): Promise<string | null> {
    return this.open({ kind: 'file', mode: 'save', title, files, defaultPath });
  }
  pickFolder(title: string, folders: string[], defaultPath: string): Promise<string | null> {
    return this.open({ kind: 'file', mode: 'folder', title, files: folders, defaultPath });
  }
}

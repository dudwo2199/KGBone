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

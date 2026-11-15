// 파일 흐름: 새로 만들기·열기·저장·가져오기·내보내기·이미지 폴더·파일 연결·끝내기.
import type { Core } from '../core/bridge';
import type { Warning } from '../core/types';
import { paths, type FileFilter, type Platform } from '../platform/platform';
import { errorText, type DocStore } from '../state/document';
import type { ImageLibrary } from '../state/images';
import { base64ToBytes } from './base64';
import type { AppDialogs } from './dialogs';

export const KGBONE_FILTER: FileFilter = { name: 'KGBone 프로젝트 (*.kgbone)', extensions: ['kgbone'] };
export const SPINE_JSON_FILTER: FileFilter = { name: 'Spine JSON 호환 (*.json)', extensions: ['json'] };

export interface FileFlowDeps {
  core: Core;
  platform: Platform;
  doc: DocStore;
  images: Pick<ImageLibrary, 'open' | 'reload' | 'get'>;
  dialogs: AppDialogs;
  notify(level: 'info' | 'warn' | 'error', text: string): void;
  /** 문서가 통째로 바뀐 뒤(선택·뷰 초기화) */
  onDocumentReplaced(): void;
  /** 저장 직전(뷰 등 작업 상태를 문서에 넣기) */
  beforeSave?(): void;
}

function warningLines(ws: Warning[]): string[] {
  return ws.map((w) => (w.path ? `${w.message} (${w.path})` : w.message));
}

export class FileFlow {
  constructor(private d: FileFlowDeps) {}

  /** 저장하지 않은 변경이 있으면 물어본다(J10). 계속해도 되면 true */
  async confirmDiscard(): Promise<boolean> {
    if (!this.d.doc.get().docState.dirty) return true;
    const choice = await this.d.dialogs.confirmSave();
    if (choice === 'cancel') return false;
    if (choice === 'save') return await this.save();
    return true;
  }

  imagesDir(): string {
    const s = this.d.doc.get();
    return paths.join(s.baseDir, s.project.imagesPath || './images');
  }

  private async afterReplace(): Promise<void> {
    this.d.onDocumentReplaced();
    await this.d.images.open(this.imagesDir());
  }

  async newFile(): Promise<void> {
    if (!(await this.confirmDiscard())) return;
    try {
      this.d.core.newDocument();
    } catch (e) {
      this.d.notify('error', errorText(e));
    }
    this.d.doc.setFile(null, this.d.doc.defaultBaseDir);
    await this.afterReplace();
    this.d.notify('info', '새 문서를 만들었습니다');
  }

  async open(path?: string, opts: { skipConfirm?: boolean } = {}): Promise<boolean> {
    if (!opts.skipConfirm && !(await this.confirmDiscard())) return false;
    const p = path ?? (await this.d.platform.openFileDialog('열기', [KGBONE_FILTER]));
    if (!p) return false;
    try {
      const text = await this.d.platform.readText(p);
      const { warnings } = this.d.core.openKgbone(text);
      this.d.doc.setFile(p, paths.dirname(p));
      await this.afterReplace();
      this.d.notify('info', `열었습니다: ${paths.basename(p)}`);
      if (warnings.length) await this.d.dialogs.report('열기 경고', [], warningLines(warnings));
      return true;
    } catch (e) {
      this.d.notify('error', `열지 못했습니다: ${errorText(e)}`);
      return false;
    }
  }

  async save(): Promise<boolean> {
    const path = this.d.doc.get().filePath;
    if (!path) return this.saveAs();
    return this.writeTo(path);
  }

  async saveAs(path?: string): Promise<boolean> {
    const s = this.d.doc.get();
    const defaultName = `${paths.stem(s.filePath ?? s.untitledName)}.kgbone`;
    let p = path ?? (await this.d.platform.saveFileDialog('다른 이름으로 저장', s.filePath ?? defaultName, [KGBONE_FILTER]));
    if (!p) return false;
    if (!/\.kgbone$/i.test(p)) p += '.kgbone';
    return this.writeTo(p);
  }

  private async writeTo(path: string): Promise<boolean> {
    try {
      this.d.beforeSave?.();
      const text = this.d.core.saveKgbone();
      await this.d.platform.writeText(path, text);
      const before = this.d.doc.get().baseDir;
      this.d.doc.setFile(path, paths.dirname(path));
      if (paths.dirname(path) !== before) await this.d.images.open(this.imagesDir());
      this.d.notify('info', `저장했습니다: ${paths.basename(path)}`);
      return true;
    } catch (e) {
      this.d.notify('error', `저장하지 못했습니다: ${errorText(e)}`);
      return false;
    }
  }

  async importSpineJson(path?: string): Promise<void> {
    if (!(await this.confirmDiscard())) return;
    const p = path ?? (await this.d.platform.openFileDialog('가져오기: Spine JSON 호환', [SPINE_JSON_FILTER]));
    if (!p) return;
    try {
      const text = await this.d.platform.readText(p);
      const { warnings } = this.d.core.importSpineJson(text);
      this.d.doc.setFile(null, paths.dirname(p), paths.stem(p));
      await this.afterReplace();
      this.d.notify('info', `가져왔습니다: ${paths.basename(p)}`);
      if (warnings.length) await this.d.dialogs.report('가져오기 경고', [], warningLines(warnings));
    } catch (e) {
      this.d.notify('error', `가져오지 못했습니다: ${errorText(e)}`);
    }
  }

  /** 이미지 폴더의 PNG 원본을 핵심에 등록(내보내기용) */
  private async registerPngs(): Promise<void> {
    const dir = this.imagesDir();
    for (const e of this.d.images.get().entries) {
      try {
        const bytes = await this.d.platform.readBinary(paths.join(dir, `${e.name}.png`));
        this.d.core.addPng(e.name, bytes);
      } catch (err) {
        this.d.notify('warn', `이미지를 읽지 못했습니다: ${e.name} (${errorText(err)})`);
      }
    }
  }

  private async writeBundle(dir: string, files: { name: string; base64: string }[]): Promise<string[]> {
    const written: string[] = [];
    for (const f of files) {
      const full = paths.join(dir, f.name);
      await this.d.platform.writeBinary(full, base64ToBytes(f.base64));
      written.push(full);
    }
    return written;
  }

  private nameStem(): string {
    const s = this.d.doc.get();
    return paths.stem(s.filePath ?? s.untitledName) || 'skeleton';
  }

  async exportSpine(args: { dir?: string; version?: '4.3' | '4.2'; pretty?: boolean; maxPageSize?: number; padding?: number } = {}): Promise<void> {
    const base = this.d.doc.get().baseDir;
    let choice = args.dir
      ? { dir: args.dir, version: args.version ?? '4.3', pretty: args.pretty ?? true, maxPageSize: args.maxPageSize ?? 2048, padding: args.padding ?? 2 }
      : await this.d.dialogs.exportSpine({ dir: paths.join(base, 'export'), version: args.version ?? '4.3', pretty: args.pretty ?? true, maxPageSize: args.maxPageSize ?? 2048, padding: args.padding ?? 2 });
    if (!choice) return;
    choice = { ...choice };
    try {
      await this.registerPngs();
      const options = { version: choice.version, pretty: choice.pretty, name: this.nameStem(), imagesPath: this.d.doc.get().project.imagesPath };
      Object.assign(options, { maxPageSize: choice.maxPageSize, padding: choice.padding });
      const bundle = this.d.core.exportSpineBundle(options);
      const written = await this.writeBundle(choice.dir, bundle.files);
      this.d.notify('info', `내보냈습니다: 파일 ${written.length}개`);
      await this.d.dialogs.report('내보내기 결과 (Spine JSON 호환 + 아틀라스)', written, warningLines(bundle.warnings));
    } catch (e) {
      this.d.notify('error', `내보내지 못했습니다: ${errorText(e)}`);
    }
  }

  async exportPngSequence(args: { dir?: string; animation?: string; fps?: number; scale?: number } = {}): Promise<void> {
    const s = this.d.doc.get();
    const anims = s.project.skeleton.animations.map((a) => a.name);
    const defaults = {
      dir: args.dir ?? paths.join(s.baseDir, 'export'),
      animation: args.animation ?? s.project.editor.activeAnimation ?? anims[0] ?? null,
      fps: args.fps ?? s.project.skeleton.fps ?? 30,
      scale: args.scale ?? 1,
      width: 0,
      height: 0,
      transparent: true,
    };
    const choice = args.dir ? defaults : await this.d.dialogs.pngSequence(defaults, anims);
    if (!choice) return;
    try {
      await this.registerPngs();
      const bundle = this.d.core.exportPngSequence({
        name: this.nameStem(),
        animation: choice.animation,
        skin: s.project.editor.activeSkin,
        fps: choice.fps,
        scale: choice.scale,
        ...(choice.width > 0 ? { width: choice.width } : {}),
        ...(choice.height > 0 ? { height: choice.height } : {}),
      });
      const written = await this.writeBundle(choice.dir, bundle.files);
      this.d.notify('info', `PNG 시퀀스를 내보냈습니다: ${written.length}장`);
      await this.d.dialogs.report('내보내기 결과 (PNG 시퀀스)', written, warningLines(bundle.warnings));
    } catch (e) {
      this.d.notify('error', `내보내지 못했습니다: ${errorText(e)}`);
    }
  }

  async setImagesPath(path?: string): Promise<void> {
    const p = path ?? (await this.d.platform.pickFolder('이미지 폴더 지정'));
    if (!p) return;
    const rel = relativePath(this.d.doc.get().baseDir, p);
    const r = this.d.doc.run('project.setImagesPath', { path: rel });
    if (!r.ok) return;
    await this.d.images.open(this.imagesDir());
    this.d.notify('info', `이미지 폴더: ${rel}`);
  }

  async reloadImages(): Promise<void> {
    await this.d.images.open(this.imagesDir());
    this.d.notify('info', `이미지 ${this.d.images.get().entries.length}개를 다시 불러왔습니다`);
  }

  async registerAssociation(): Promise<void> {
    try {
      const msg = await this.d.platform.registerAssociation();
      await this.d.dialogs.message('파일 연결 등록', msg);
    } catch (e) {
      this.d.notify('error', errorText(e));
    }
  }

  async quit(): Promise<void> {
    if (!(await this.confirmDiscard())) return;
    await this.d.platform.quit();
  }
}

/** from 폴더 기준 to의 상대 경로('./…'). 드라이브가 다르면 절대 경로 */
export function relativePath(from: string, to: string): string {
  if (!from) return to;
  const sep = /[\\/]/;
  const a = from.split(sep).filter(Boolean);
  const b = to.split(sep).filter(Boolean);
  if (a.length && b.length && a[0].toLowerCase() !== b[0].toLowerCase() && /:$/.test(a[0])) return to;
  let i = 0;
  while (i < a.length && i < b.length && a[i] === b[i]) i++;
  const up = a.slice(i).map(() => '..');
  const rest = b.slice(i);
  const parts = [...up, ...rest];
  if (!parts.length) return '.';
  return up.length ? parts.join('/') : './' + parts.join('/');
}

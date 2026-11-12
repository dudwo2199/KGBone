// 단위 테스트용 가짜 Core·Platform.
import type { Core } from '../core/bridge';
import type { Bundle, DocState, PoseOutput, Project } from '../core/types';
import type { ImageEntry, Platform } from '../platform/platform';
import { emptyProject } from '../state/document';

export class FakeCore implements Core {
  state_: DocState = { dirty: false, canUndo: false, canRedo: false, undoLabel: null, redoLabel: null };
  project_: Project = emptyProject();
  calls: { id: string; args: Record<string, unknown> }[] = [];
  saved = 0;
  exprCalls: [string, number][] = [];
  images: { path: string; w: number; h: number }[] = [];
  newDocument() {
    this.project_ = emptyProject();
    this.state_ = { ...this.state_, dirty: false };
  }
  openKgbone() {
    this.state_ = { ...this.state_, dirty: false };
    return { warnings: [] };
  }
  saveKgbone() {
    this.saved++;
    this.state_ = { ...this.state_, dirty: false };
    return '{}';
  }
  importSpineJson() {
    return { warnings: [] };
  }
  exportSpineJson() {
    return { text: '', warnings: [] };
  }
  exportSpineBundle(): Bundle {
    return { files: [], warnings: [] };
  }
  exportPngSequence(): Bundle {
    return { files: [], warnings: [] };
  }
  execute<T>(id: string, args: Record<string, unknown> = {}): T {
    this.calls.push({ id, args });
    this.state_ = { ...this.state_, dirty: true };
    return null as T;
  }
  beginGroup() {}
  endGroup() {}
  project() {
    return this.project_;
  }
  state() {
    return this.state_;
  }
  pose(): PoseOutput {
    return { bones: [], locals: [], active: [], draw: [], overlays: [], events: [] };
  }
  resetPhysics() {}
  evalExpr(input: string, current: number) {
    this.exprCalls.push([input, current]);
    if (input === 'v*2') return current * 2;
    if (input === '+=5') return current + 5;
    if (/^-?\d+(\.\d+)?$/.test(input)) return Number(input);
    throw new Error('잘못된 수식');
  }
  setImage(path: string, w: number, h: number) {
    this.images.push({ path, w, h });
  }
  addPng() {}
}

export class FakePlatform implements Platform {
  readonly kind = 'web' as const;
  files = new Map<string, string>();
  images: ImageEntry[] = [];
  watchCb: (() => void) | null = null;
  saveAnswer: string | null = '/p/new.kgbone';
  async openFileDialog() {
    return null;
  }
  async saveFileDialog() {
    return this.saveAnswer;
  }
  async pickFolder() {
    return null;
  }
  async readText(p: string) {
    return this.files.get(p) ?? '';
  }
  async writeText(p: string, t: string) {
    this.files.set(p, t);
  }
  async readBinary() {
    return new Uint8Array();
  }
  async writeBinary() {}
  async listImages() {
    return this.images;
  }
  async watchFolder(_dir: string, cb: () => void) {
    this.watchCb = cb;
    return () => {
      this.watchCb = null;
    };
  }
  async registerAssociation() {
    return '';
  }
  async launchFile() {
    return null;
  }
  onCloseRequested() {}
  setTitle() {}
  async quit() {}
}

// WebAssembly 핵심 불러오기. crates/kgbone-wasm의 C ABI 함수를 감싸 Core 인터페이스를 만든다.
import type { Core, PngSequenceOptions, SpineExportOptions } from './bridge';
import { CoreError } from './bridge';
import type { Bundle, DocState, PoseOutput, PoseRequest, Project, Warning } from './types';

interface Exports {
  memory: WebAssembly.Memory;
  kg_alloc(len: number): number;
  kg_free(ptr: number, len: number): void;
  kg_call(ptr: number, len: number): number;
  kg_result_ptr(): number;
  kg_set_image(pathPtr: number, pathLen: number, w: number, h: number, alphaPtr: number, alphaLen: number): void;
  kg_add_png(pathPtr: number, pathLen: number, pngPtr: number, pngLen: number): void;
}

const enc = new TextEncoder();
const dec = new TextDecoder();

class WasmCore implements Core {
  constructor(private ex: Exports) {}

  private put(bytes: Uint8Array): [number, number] {
    const ptr = this.ex.kg_alloc(bytes.length);
    new Uint8Array(this.ex.memory.buffer, ptr, bytes.length).set(bytes);
    return [ptr, bytes.length];
  }

  private call<T>(req: Record<string, unknown>): T {
    const [ptr, len] = this.put(enc.encode(JSON.stringify(req)));
    const outLen = this.ex.kg_call(ptr, len);
    this.ex.kg_free(ptr, len);
    const outPtr = this.ex.kg_result_ptr();
    const text = dec.decode(new Uint8Array(this.ex.memory.buffer, outPtr, outLen));
    const resp = JSON.parse(text) as { ok?: T; error?: string };
    if (resp.error !== undefined) throw new CoreError(resp.error);
    return resp.ok as T;
  }

  newDocument() {
    this.call({ method: 'new' });
  }
  openKgbone(text: string) {
    return this.call<{ warnings: Warning[] }>({ method: 'openKgbone', text });
  }
  saveKgbone() {
    return this.call<{ text: string }>({ method: 'saveKgbone' }).text;
  }
  importSpineJson(text: string) {
    return this.call<{ warnings: Warning[] }>({ method: 'importSpineJson', text });
  }
  exportSpineJson(options: SpineExportOptions = {}) {
    return this.call<{ text: string; warnings: Warning[] }>({ method: 'exportSpineJson', options });
  }
  exportSpineBundle(options: SpineExportOptions = {}) {
    return this.call<Bundle>({ method: 'exportSpineBundle', options });
  }
  exportPngSequence(options: PngSequenceOptions = {}) {
    return this.call<Bundle>({ method: 'exportPngSequence', options });
  }
  execute<T = unknown>(id: string, args: Record<string, unknown> = {}): T {
    return this.call<T>({ method: 'execute', id, args });
  }
  beginGroup(label: string) {
    this.execute('history.begin', { label });
  }
  endGroup() {
    this.execute('history.end');
  }
  project() {
    return this.call<Project>({ method: 'project' });
  }
  state() {
    return this.call<DocState>({ method: 'state' });
  }
  pose(request: PoseRequest) {
    return this.call<PoseOutput>({ method: 'pose', request });
  }
  resetPhysics() {
    this.call({ method: 'resetPhysics' });
  }
  evalExpr(input: string, current: number) {
    return this.call<number>({ method: 'evalExpr', input, current });
  }
  setImage(path: string, width: number, height: number, alpha?: Uint8Array) {
    const [pp, pl] = this.put(enc.encode(path));
    const [ap, al] = alpha && alpha.length ? this.put(alpha) : [0, 0];
    this.ex.kg_set_image(pp, pl, width, height, ap, al);
    this.ex.kg_free(pp, pl);
    if (al) this.ex.kg_free(ap, al);
  }
  addPng(path: string, png: Uint8Array) {
    const [pp, pl] = this.put(enc.encode(path));
    const [bp, bl] = this.put(png);
    this.ex.kg_add_png(pp, pl, bp, bl);
    this.ex.kg_free(pp, pl);
    this.ex.kg_free(bp, bl);
  }
}

/** wasm 바이트(또는 URL)로 핵심 모듈을 만든다. */
export async function loadCore(source: string | BufferSource): Promise<Core> {
  const bytes = typeof source === 'string' ? await (await fetch(source)).arrayBuffer() : source;
  const { instance } = await WebAssembly.instantiate(bytes, {});
  return new WasmCore(instance.exports as unknown as Exports);
}

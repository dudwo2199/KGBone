// kgbone.wasm을 Node에서 불러와 JSON 요청을 보낸다. 화면 쪽 app/src/core/wasm.ts와 같은 C ABI를 쓴다
// (함수 목록: interface/core-api.md 1장).
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const enc = new TextEncoder();
const dec = new TextDecoder();
const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

/** 핵심이 돌려준 오류(한국어 메시지) */
export class CoreError extends Error {}

/** 쓸 wasm 파일 경로: KGBONE_WASM → app/public/kgbone.wasm */
export function findWasm() {
  const candidates = [process.env.KGBONE_WASM, join(repoRoot, "app", "public", "kgbone.wasm")].filter(Boolean);
  const found = candidates.find((p) => existsSync(p));
  if (!found) {
    throw new Error("kgbone.wasm이 없습니다. npm run wasm(Rust 필요) 또는 npm run wasm:fetch(배포된 빌드 받기)를 먼저 실행하세요.");
  }
  return found;
}

export class Core {
  #ex;
  constructor(exports) {
    this.#ex = exports;
  }

  #put(bytes) {
    const ptr = this.#ex.kg_alloc(bytes.length);
    new Uint8Array(this.#ex.memory.buffer, ptr, bytes.length).set(bytes);
    return [ptr, bytes.length];
  }

  /** JSON 요청 하나. 오류 응답이면 CoreError를 던진다 */
  call(request) {
    const [ptr, len] = this.#put(enc.encode(JSON.stringify(request)));
    const outLen = this.#ex.kg_call(ptr, len);
    this.#ex.kg_free(ptr, len);
    const text = dec.decode(new Uint8Array(this.#ex.memory.buffer, this.#ex.kg_result_ptr(), outLen));
    const resp = JSON.parse(text);
    if (resp.error !== undefined) throw new CoreError(resp.error);
    return resp.ok;
  }

  execute(id, args = {}) {
    return this.call({ method: "execute", id, args });
  }
  project() {
    return this.call({ method: "project" });
  }
  state() {
    return this.call({ method: "state" });
  }

  /** 이미지 크기·알파 등록(알파는 가로×세로 바이트, 없으면 생략) */
  setImage(path, width, height, alpha) {
    const [pp, pl] = this.#put(enc.encode(path));
    const [ap, al] = alpha && alpha.length ? this.#put(alpha) : [0, 0];
    this.#ex.kg_set_image(pp, pl, width, height, ap, al);
    this.#ex.kg_free(pp, pl);
    if (al) this.#ex.kg_free(ap, al);
  }

  /** 그리기·내보내기용 PNG 원본 등록 */
  addPng(path, png) {
    const [pp, pl] = this.#put(enc.encode(path));
    const [bp, bl] = this.#put(png);
    this.#ex.kg_add_png(pp, pl, bp, bl);
    this.#ex.kg_free(pp, pl);
    this.#ex.kg_free(bp, bl);
  }
}

export async function loadCore(wasmPath = findWasm()) {
  const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
  return new Core(instance.exports);
}

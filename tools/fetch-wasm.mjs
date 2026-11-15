// Rust가 없는 PC용: GitHub Pages에 올라간 최신 빌드의 kgbone.wasm을 app/public/kgbone.wasm으로 받는다.
// 받은 파일은 origin main 기준이다. Rust 핵심을 이 PC에서 고쳤다면 받지 말고 npm run wasm으로 직접 빌드한다.
// 사용: npm run wasm:fetch   (주소를 바꾸려면 KGBONE_WASM_URL)
import { mkdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const url = process.env.KGBONE_WASM_URL || "https://dudwo2199.github.io/KGBone/kgbone.wasm";
const out = join(dirname(fileURLToPath(import.meta.url)), "..", "app", "public", "kgbone.wasm");

const res = await fetch(url);
if (!res.ok) {
  console.error(`받지 못했습니다(${res.status}): ${url}`);
  process.exit(1);
}
const bytes = new Uint8Array(await res.arrayBuffer());
// WebAssembly 파일은 "\0asm"으로 시작한다
if (bytes.length < 8 || bytes[0] !== 0 || bytes[1] !== 0x61 || bytes[2] !== 0x73 || bytes[3] !== 0x6d) {
  console.error(`WebAssembly 파일이 아닙니다: ${url}`);
  process.exit(1);
}
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, bytes);
console.log(`app/public/kgbone.wasm 받음 (${bytes.length}바이트) ← ${url}`);

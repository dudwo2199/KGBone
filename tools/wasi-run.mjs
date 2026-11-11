// cargo test --target wasm32-wasip1 실행기: 테스트 .wasm을 Node의 WASI로 돌린다.
// 이 PC는 Windows 스마트 앱 컨트롤이 새로 빌드한 서명 없는 .exe를 막기 때문에, 테스트는 WebAssembly로 돌린다.
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";
import { argv, exit, env } from "node:process";
import { resolve } from "node:path";

const [wasmPath, ...args] = argv.slice(2);
const root = resolve(import.meta.dirname, "..");
const wasi = new WASI({ version: "preview1", args: [wasmPath, ...args], env, preopens: { "/": root, ".": root }, returnOnExit: true });
const mod = await WebAssembly.compile(await readFile(wasmPath));
const inst = await WebAssembly.instantiate(mod, wasi.getImportObject());
try {
  exit(wasi.start(inst) ?? 0);
} catch (e) {
  // panic=abort 이면 unreachable 트랩으로 끝난다.
  console.error("테스트 실행 중단:", e.message);
  exit(101);
}

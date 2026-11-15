// Vite 플러그인: 빌드가 끝나면 app/dist/sw.js에 캐시할 파일 목록과 판 번호를 채워 넣는다.
// 판 번호는 파일 내용 해시라서, 화면이 바뀌면 서비스 워커도 바뀌어 옛 캐시가 지워진다.
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, writeFileSync, existsSync } from "node:fs";
import { join } from "node:path";

const VERSION_LINE = /^const VERSION = .*$/m;
const FILES_LINE = /^const FILES = .*$/m;

/** dir 아래 모든 파일의 상대 경로('/' 구분, 정렬) */
function listFiles(dir, prefix = "") {
  const out = [];
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.isDirectory()) out.push(...listFiles(join(dir, e.name), `${prefix}${e.name}/`));
    else out.push(prefix + e.name);
  }
  return out.sort();
}

/**
 * 서비스 워커 원본에 목록과 판 번호를 채운다.
 * files: [상대 경로, 내용] 목록. sw.js 자신은 빼고, index.html은 './'로 담는다.
 */
export function fillServiceWorker(source, files) {
  if (!VERSION_LINE.test(source) || !FILES_LINE.test(source)) throw new Error("sw.js에서 VERSION/FILES 줄을 찾지 못했습니다");
  const hash = createHash("sha256");
  const urls = [];
  for (const [name, data] of [...files].sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0))) {
    if (name === "sw.js") continue;
    hash.update(name).update("\0").update(data).update("\0");
    urls.push(name === "index.html" ? "./" : name);
  }
  const version = hash.digest("hex").slice(0, 12);
  return source
    .replace(VERSION_LINE, `const VERSION = ${JSON.stringify(version)};`)
    .replace(FILES_LINE, `const FILES = ${JSON.stringify(urls)};`);
}

export function precachePlugin() {
  let outDir = "";
  return {
    name: "kgbone-precache",
    apply: "build",
    configResolved(config) {
      outDir = join(config.root, config.build.outDir);
    },
    closeBundle() {
      const sw = join(outDir, "sw.js");
      if (!existsSync(sw)) return;
      const files = listFiles(outDir).map((name) => [name, readFileSync(join(outDir, name))]);
      writeFileSync(sw, fillServiceWorker(readFileSync(sw, "utf8"), files));
    },
  };
}

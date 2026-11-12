// KGBone 브라우저 판 실행기: app/dist를 내 PC 안에서만(127.0.0.1) 서비스하고 Edge 앱 창으로 연다.
// 서명 없는 exe가 막히는 PC(스마트 앱 컨트롤)에서도 서명된 Node·Edge만으로 실행된다.
// 사용: npm run app  (먼저 npm run build가 돌아간다)
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join, extname, normalize, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "app", "dist");
const port = Number(process.env.KGBONE_PORT || 1421);
const types = {
  ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm", ".png": "image/png", ".svg": "image/svg+xml", ".json": "application/json; charset=utf-8",
  ".ico": "image/x-icon", ".woff2": "font/woff2",
};

if (!existsSync(join(root, "index.html"))) {
  console.error("app/dist가 없습니다. 먼저 npm run build를 실행하세요.");
  process.exit(1);
}

const server = createServer(async (req, res) => {
  try {
    const url = decodeURIComponent(new URL(req.url, "http://x").pathname);
    let file = normalize(join(root, url));
    if (!file.startsWith(root)) throw new Error("범위 밖");
    if (!existsSync(file) || (await stat(file)).isDirectory()) file = join(root, "index.html");
    res.writeHead(200, { "Content-Type": types[extname(file).toLowerCase()] || "application/octet-stream", "Cache-Control": "no-cache" });
    res.end(await readFile(file));
  } catch {
    res.writeHead(404);
    res.end();
  }
});

server.listen(port, "127.0.0.1", () => {
  const url = `http://127.0.0.1:${port}/`;
  console.log(`KGBone 실행 중: ${url}  (이 창을 닫으면 종료)`);
  const edges = [
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
  ];
  const edge = edges.find((p) => existsSync(p));
  if (edge && !process.argv.includes("--no-open")) {
    spawn(edge, [`--app=${url}`, "--window-size=1600,1000"], { detached: true, stdio: "ignore" }).unref();
  } else {
    console.log("브라우저에서 위 주소를 여세요.");
  }
});

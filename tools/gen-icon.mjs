// KGBone 앱 아이콘 생성기(독자 디자인). 1024×1024 PNG를 코드로 그린다(Node 내장 zlib만 사용).
// 디자인: 둥근 사각 바탕(남색→보라 세로 그라데이션) + 관절 세 개로 이어진 두 마디 뼈 + 아래쪽 "KG" 글자.
// 실행: node tools/gen-icon.mjs  →  tools/icon.png(1024), app/public/icons/icon-192.png·icon-512.png(설치형 웹 앱용)
// 아이콘 세트: npx tauri icon tools/icon.png -o src-tauri/icons
import { writeFileSync, mkdirSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const N = 1024; // 도형 좌표의 기준 크기

// ---------- PNG 쓰기 (tools/gen-samples.mjs와 같은 방식) ----------
const crcTable = new Uint32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const td = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(td));
  return Buffer.concat([len, td, crc]);
}
function encodePng(w, h, rgba) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    rgba.copy(raw, y * (w * 4 + 1) + 1, y * w * 4, (y + 1) * w * 4);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw, { level: 9 })), chunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---------- 거리 함수(음수 = 안쪽, 단위 픽셀) ----------
const len = (x, y) => Math.hypot(x, y);
/** 둥근 사각형: 중심 (cx,cy), 반폭 hw, 반높이 hh, 모서리 반지름 r */
function sdRoundRect(px, py, cx, cy, hw, hh, r) {
  const qx = Math.abs(px - cx) - hw + r, qy = Math.abs(py - cy) - hh + r;
  return len(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
}
/** 선분 a-b, 두께 반지름 r (끝이 둥근 막대) */
function sdSegment(px, py, ax, ay, bx, by, r) {
  const pax = px - ax, pay = py - ay, bax = bx - ax, bay = by - ay;
  const t = Math.max(0, Math.min(1, (pax * bax + pay * bay) / (bax * bax + bay * bay)));
  return len(pax - bax * t, pay - bay * t) - r;
}
/** 선분 a-b, 끝 반지름 ra → rb로 가늘어지는 막대(뼈 모양) */
function sdTaper(px, py, ax, ay, bx, by, ra, rb) {
  const pax = px - ax, pay = py - ay, bax = bx - ax, bay = by - ay;
  const t = Math.max(0, Math.min(1, (pax * bax + pay * bay) / (bax * bax + bay * bay)));
  return len(pax - bax * t, pay - bay * t) - (ra + (rb - ra) * t);
}
const sdCircle = (px, py, cx, cy, r) => len(px - cx, py - cy) - r;
/** 원호: 중심, 반지름 R, 두께 반지름 w, 각도 a0→a1(라디안, 수학 방향·y 아래) 바깥은 끝을 둥글게 */
function sdArc(px, py, cx, cy, R, w, a0, a1) {
  let a = Math.atan2(py - cy, px - cx);
  const inRange = (ang) => {
    let t = ang - a0;
    while (t < 0) t += Math.PI * 2;
    return t <= a1 - a0;
  };
  if (inRange(a)) return Math.abs(len(px - cx, py - cy) - R) - w;
  const e0 = sdCircle(px, py, cx + R * Math.cos(a0), cy + R * Math.sin(a0), w);
  const e1 = sdCircle(px, py, cx + R * Math.cos(a1), cy + R * Math.sin(a1), w);
  return Math.min(e0, e1);
}
/** 덮임 정도. k = 출력 한 픽셀이 기준 좌표로 몇 픽셀인지 */
const cover = (d, k = 1) => Math.max(0, Math.min(1, 0.5 - d / k));

// ---------- 도형 배치 ----------
const BG_TOP = [30, 42, 78], BG_BOTTOM = [86, 58, 150];
const BONE = [242, 244, 250];
const JOINT = [255, 176, 72];
const TEXT = [255, 255, 255];

// 두 마디 뼈: 어깨 → 팔꿈치 → 손목 (팔을 굽힌 모양)
const J = [[250, 510], [470, 250], [770, 380]];
const boneShape = (x, y) => Math.min(
  sdTaper(x, y, J[0][0], J[0][1], J[1][0], J[1][1], 46, 26),
  sdTaper(x, y, J[1][0], J[1][1], J[2][0], J[2][1], 42, 22),
);
const jointRing = (x, y) => Math.min(...J.map(([cx, cy], i) => sdCircle(x, y, cx, cy, i === 0 ? 78 : 66)));
const jointCore = (x, y) => Math.min(...J.map(([cx, cy], i) => sdCircle(x, y, cx, cy, i === 0 ? 46 : 38)));

// "KG" 글자(획 두께 반지름 30): 글자 높이 230, 아래쪽 가운데
const S = 30, TOP = 660, BOT = 880;
function letters(x, y) {
  const kx = 330;
  const k = Math.min(
    sdSegment(x, y, kx, TOP, kx, BOT, S),
    sdSegment(x, y, kx + 6, (TOP + BOT) / 2 + 10, kx + 150, TOP, S),
    sdSegment(x, y, kx + 50, (TOP + BOT) / 2 - 20, kx + 160, BOT, S),
  );
  const gcx = 640, gcy = (TOP + BOT) / 2, gR = (BOT - TOP) / 2;
  const g = Math.min(
    // 원호: 오른쪽 아래(30°)에서 아래·왼쪽·위를 돌아 오른쪽 위(320°)까지.
    // y가 아래로 커지므로 각도가 커지는 쪽이 화면에서 시계 방향이다.
    sdArc(x, y, gcx, gcy, gR, S, Math.PI * (30 / 180), Math.PI * (320 / 180)),
    // 가로 막대와 오른쪽 짧은 세로 획(원호 끝 y = gcy + gR·sin30° = gcy + 0.5·gR 까지)
    sdSegment(x, y, gcx + 20, gcy + 10, gcx + gR, gcy + 10, S),
    sdSegment(x, y, gcx + gR, gcy + 10, gcx + gR, gcy + gR * 0.5, S),
  );
  return Math.min(k, g);
}

// ---------- 그리기 ----------
/** size×size로 그린 PNG. 도형은 기준 크기(N) 좌표로 두고 픽셀 중심만 옮겨 잰다 */
function render(size) {
  const k = N / size;
  const rgba = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const px = (x + 0.5) * k, py = (y + 0.5) * k;
      // 바탕
      const bgA = cover(sdRoundRect(px, py, N / 2, N / 2, 448, 448, 200), k);
      const t = py / N;
      let c = BG_TOP.map((v, i) => v + (BG_BOTTOM[i] - v) * t);
      // 위에 덮기(곱해지지 않은 알파, 바탕 안에서만)
      const over = (col, cov) => {
        c = c.map((v, i) => v + (col[i] - v) * cov);
      };
      if (bgA > 0) {
        // 뼈 그림자(살짝 아래로)
        over([18, 22, 48], cover(Math.min(boneShape(px, py - 14), jointRing(px, py - 14)), k) * 0.35);
        over(BONE, cover(boneShape(px, py), k));
        over(BONE, cover(jointRing(px, py), k));
        over(JOINT, cover(jointCore(px, py), k));
        over(TEXT, cover(letters(px, py), k));
      }
      const o = (y * size + x) * 4;
      rgba[o] = Math.round(c[0]); rgba[o + 1] = Math.round(c[1]); rgba[o + 2] = Math.round(c[2]);
      rgba[o + 3] = Math.round(bgA * 255);
    }
  }
  return encodePng(size, size, rgba);
}

const webDir = join(here, "..", "app", "public", "icons");
mkdirSync(webDir, { recursive: true });
const outputs = [[join(here, "icon.png"), N], [join(webDir, "icon-192.png"), 192], [join(webDir, "icon-512.png"), 512]];
for (const [file, size] of outputs) {
  writeFileSync(file, render(size));
  console.log(`아이콘을 만들었습니다: ${file}`);
}

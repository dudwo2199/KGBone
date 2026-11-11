// 시험용 파일 생성기. 이미지는 코드로 그리고, Spine JSON은 공개 형식 문서(docs/SOURCES.md S1) 구조대로 직접 만든다.
// 실행: node tools/gen-samples.mjs
import { writeFileSync, mkdirSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "samples");

// ---------- PNG ----------
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
function png(w, h, shade) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const [r, g, b, a] = shade(x, y);
      const o = y * (w * 4 + 1) + 1 + x * 4;
      raw[o] = r; raw[o + 1] = g; raw[o + 2] = b; raw[o + 3] = a;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8; ihdr[9] = 6; ihdr[10] = 0; ihdr[11] = 0; ihdr[12] = 0;
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0)),
  ]);
}
const rect = (w, h, col) => png(w, h, (x, y) => {
  const edge = x < 2 || y < 2 || x >= w - 2 || y >= h - 2;
  return edge ? [col[0] / 2, col[1] / 2, col[2] / 2, 255] : [...col, 255];
});
const circle = (d, col) => png(d, d, (x, y) => {
  const r = d / 2, dx = x + 0.5 - r, dy = y + 0.5 - r, dist = Math.hypot(dx, dy);
  if (dist > r) return [0, 0, 0, 0];
  return dist > r - 2 ? [col[0] / 2, col[1] / 2, col[2] / 2, 255] : [...col, 255];
});
const capsule = (w, h, col) => png(w, h, (x, y) => {
  const r = h / 2, cx = Math.min(Math.max(x + 0.5, r), w - r), dist = Math.hypot(x + 0.5 - cx, y + 0.5 - r);
  return dist > r ? [0, 0, 0, 0] : [...col, 255];
});

function write(rel, data) {
  const p = join(root, rel);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, data);
}
const images = {
  "head": circle(64, [240, 200, 160]),
  "body": rect(48, 96, [80, 120, 200]),
  "arm": capsule(64, 20, [230, 180, 140]),
  "leg": capsule(72, 24, [60, 60, 90]),
  "tail": capsule(128, 32, [200, 120, 60]),
  "gem": circle(32, [80, 220, 120]),
  "hat-red": rect(56, 24, [220, 50, 50]),
  "hat-blue": rect(56, 24, [50, 80, 220]),
  "glow": circle(48, [255, 240, 120]),
};
for (const [name, data] of Object.entries(images)) write(`images/${name}.png`, data);

// ---------- Spine JSON (문서 구조) ----------
const json = (o) => JSON.stringify(o, null, "\t") + "\n";
const head = (version, extra = {}) => ({ hash: "kgbone-sample", spine: version, x: -100, y: 0, width: 200, height: 300, images: "../images/", audio: "", ...extra });

// 1) 본만
const bonesOnly = {
  skeleton: head("4.3.00"),
  bones: [
    { name: "root" },
    { name: "hip", parent: "root", y: 100, color: "ff0000ff" },
    { name: "spine", parent: "hip", length: 80, rotation: 90 },
    { name: "neck", parent: "spine", length: 20, x: 80, rotation: -10, scaleX: 1.5, inherit: "noScale" },
    { name: "arm", parent: "spine", length: 50, x: 60, rotation: 180, shearX: 10, shearY: -5 },
    { name: "only-t", parent: "spine", x: 30, inherit: "onlyTranslation" },
  ],
};
write("spine43/bones-only.json", json(bonesOnly));

// 2) 메시 + 가중치
const meshWeights = {
  skeleton: head("4.3.00"),
  bones: [
    { name: "root" },
    { name: "tail1", parent: "root", length: 64 },
    { name: "tail2", parent: "tail1", length: 64, x: 64 },
  ],
  slots: [{ name: "tail", bone: "tail1", attachment: "tail" }],
  skins: [{
    name: "default",
    attachments: {
      tail: {
        tail: {
          type: "mesh", uvs: [0, 1, 0.5, 1, 1, 1, 1, 0, 0.5, 0, 0, 0],
          triangles: [0, 1, 5, 1, 4, 5, 1, 2, 4, 2, 3, 4],
          // 정점마다: 본 수, (본 번호, x, y, 가중치)…
          vertices: [
            1, 1, 0, -16, 1,
            2, 1, 64, -16, 0.5, 2, 0, -16, 0.5,
            1, 2, 64, -16, 1,
            1, 2, 64, 16, 1,
            2, 1, 64, 16, 0.5, 2, 0, 16, 0.5,
            1, 1, 0, 16, 1,
          ],
          hull: 6, edges: [0, 2, 2, 4, 4, 6, 6, 8, 8, 10, 10, 0], width: 128, height: 32,
        },
      },
    },
  }],
  animations: {
    wag: {
      bones: { tail2: { rotate: [{ value: 0, curve: [0.25, 0, 0.75, 30] }, { time: 1, value: 30 }] } },
      attachments: { default: { tail: { tail: { deform: [{}, { time: 0.5, offset: 2, vertices: [0, 4, 0, 4] }, { time: 1 }] } } } },
    },
  },
};
write("spine43/mesh-weights.json", json(meshWeights));

// 3) IK
const ik = {
  skeleton: head("4.3.00"),
  bones: [
    { name: "root" },
    { name: "thigh", parent: "root", length: 50, rotation: -90 },
    { name: "shin", parent: "thigh", length: 50, x: 50 },
    { name: "target", parent: "root", x: 30, y: -60, color: "ff3f00ff" },
    { name: "single", parent: "root", length: 40, x: -50 },
    { name: "aim", parent: "root", x: -50, y: 40 },
  ],
  ik: [
    { name: "leg-ik", order: 0, bones: ["thigh", "shin"], target: "target", bendPositive: false },
    { name: "aim-ik", order: 1, bones: ["single"], target: "aim", mix: 0.5, compress: true, stretch: true },
  ],
  animations: {
    reach: { ik: { "leg-ik": [{ mix: 1 }, { time: 1, mix: 0, softness: 5, bendPositive: false }] } },
  },
};
write("spine43/ik.json", json(ik));

// 4) 트랜스폼
const transform = {
  skeleton: head("4.3.00"),
  bones: [
    { name: "root" },
    { name: "src", parent: "root", x: 20, y: 30, rotation: 45 },
    { name: "follow", parent: "root", x: -20 },
    { name: "follow-local", parent: "root", x: -40 },
  ],
  transform: [
    { name: "tc-world", order: 0, bones: ["follow"], target: "src", rotation: 10, x: 5, mixRotate: 1, mixX: 0.5, mixScaleX: 0, mixShearY: 0 },
    { name: "tc-local", order: 1, bones: ["follow-local"], target: "src", local: true, relative: true, mixRotate: 0.5, mixX: 0, mixY: 0, mixScaleX: 0, mixShearY: 0 },
  ],
  animations: { mixing: { transform: { "tc-world": [{ mixRotate: 1 }, { time: 1, mixRotate: 0, mixX: 0 }] } } },
};
write("spine43/transform.json", json(transform));

// 5) 패스
const path = {
  skeleton: head("4.3.00"),
  bones: [
    { name: "root" },
    { name: "p1", parent: "root", length: 20 },
    { name: "p2", parent: "p1", length: 20, x: 20 },
    { name: "p3", parent: "p2", length: 20, x: 20 },
  ],
  slots: [{ name: "path", bone: "root", attachment: "path" }],
  skins: [{ name: "default", attachments: { path: { path: { type: "path", closed: false, lengths: [100], vertexCount: 6, vertices: [-10, 0, 0, 0, 10, 0, 90, 0, 100, 0, 110, 0] } } } }],
  path: [{ name: "follow-path", order: 0, bones: ["p1", "p2", "p3"], target: "path", spacingMode: "length", rotateMode: "chain", position: 0.1 }],
  animations: { slide: { path: { "follow-path": { position: [{ value: 0 }, { time: 2, value: 0.6 }] } } } },
};
write("spine43/path.json", json(path));

// 6) 물리
const physics = {
  skeleton: head("4.3.00", { fps: 30 }),
  bones: [
    { name: "root" },
    { name: "body", parent: "root", length: 60, rotation: 90 },
    { name: "hair", parent: "body", length: 40, x: 60, rotation: 180 },
  ],
  physics: [{ name: "hair-phys", order: 0, bone: "hair", rotate: 1, x: 0.5, inertia: 0.8, strength: 120, damping: 0.9, mass: 1.5, gravity: 50, wind: 10 }],
  animations: {
    shake: {
      bones: { body: { translate: [{ x: 0 }, { time: 0.5, x: 40 }, { time: 1, x: 0 }] } },
      physics: { "hair-phys": { mix: [{ value: 1 }, { time: 1, value: 0.5 }] }, "": { reset: [{ time: 1 }] } },
    },
  },
};
write("spine43/physics.json", json(physics));

// 7) 클리핑
const clipping = {
  skeleton: head("4.3.00"),
  bones: [{ name: "root" }],
  slots: [
    { name: "clip", bone: "root", attachment: "clip" },
    { name: "body", bone: "root", attachment: "body" },
    { name: "after", bone: "root", attachment: "gem" },
  ],
  skins: [{
    name: "default",
    attachments: {
      clip: { clip: { type: "clipping", end: "body", vertexCount: 4, vertices: [-20, -20, 20, -20, 20, 20, -20, 20], color: "ce3a3aff" } },
      body: { body: { width: 48, height: 96 } },
      after: { gem: { x: 40, width: 32, height: 32 } },
    },
  }],
};
write("spine43/clipping.json", json(clipping));

// 8) 스킨 여러 개 + 블렌드·어두운 색
const skins = {
  skeleton: head("4.3.00"),
  bones: [
    { name: "root" },
    { name: "head", parent: "root", y: 60 },
    { name: "hat-bone", parent: "head", y: 40, skin: true },
  ],
  slots: [
    { name: "head", bone: "head", attachment: "head" },
    { name: "hat", bone: "hat-bone", attachment: "hat", color: "ffffffcc", dark: "202020" },
    { name: "glow", bone: "head", attachment: "glow", blend: "additive" },
  ],
  skins: [
    { name: "default", attachments: { head: { head: { width: 64, height: 64 } }, glow: { glow: { width: 48, height: 48 } } } },
    { name: "red", bones: ["hat-bone"], attachments: { hat: { hat: { name: "hat-red", width: 56, height: 24 } } } },
    { name: "blue", bones: ["hat-bone"], attachments: { hat: { hat: { name: "hat-blue", width: 56, height: 24, rotation: 5 } } } },
  ],
};
write("spine43/skins.json", json(skins));

// 9) 이벤트 (+ 모르는 필드 보존 시험용 "x-custom")
const events = {
  skeleton: head("4.3.00"),
  bones: [{ name: "root" }],
  events: { footstep: { int: 1, float: 0.5, string: "왼발", audio: "step.wav", volume: 0.8, balance: -0.2 }, hit: {} },
  animations: {
    walk: {
      events: [{ time: 0.25, name: "footstep" }, { time: 0.75, name: "footstep", string: "오른발", int: 2 }, { time: 1, name: "hit" }],
      "x-custom": { keep: [1, 2.50, 3] },
    },
  },
  "x-custom-root": { note: "모르는 필드는 내보낼 때 그대로 남아야 한다", value: 1.250 },
};
// 숫자 표기 보존 시험을 위해 2.50, 1.250을 문자열로 직접 박는다.
write("spine43/events.json", json(events).replace('"keep": [\n\t\t\t\t\t1,\n\t\t\t\t\t2.5,', '"keep": [\n\t\t\t\t\t1,\n\t\t\t\t\t2.50,').replace('"value": 1.25', '"value": 1.250'));

// 10) 4.2 형식 (전부 들어간 캐릭터)
const character42 = {
  skeleton: head("4.2.43"),
  bones: [
    { name: "root" },
    { name: "hip", parent: "root", y: 120 },
    { name: "torso", parent: "hip", length: 80, rotation: 90 },
    { name: "head", parent: "torso", length: 40, x: 90, rotation: -90 },
    { name: "arm-l", parent: "torso", length: 60, x: 70, rotation: 150 },
    { name: "leg-l", parent: "hip", length: 60, rotation: -100 },
    { name: "foot-target", parent: "root", x: -10, y: 10 },
    { name: "leg-l2", parent: "leg-l", length: 50, x: 60 },
  ],
  slots: [
    { name: "leg", bone: "leg-l", attachment: "leg" },
    { name: "body", bone: "torso", attachment: "body" },
    { name: "arm", bone: "arm-l", attachment: "arm" },
    { name: "head", bone: "head", attachment: "head" },
  ],
  ik: [{ name: "foot", order: 1, bones: ["leg-l", "leg-l2"], target: "foot-target" }],
  transform: [{ name: "arm-follow", order: 2, bones: ["arm-l"], target: "head", mixRotate: 0.3, mixX: 0, mixScaleX: 0, mixShearY: 0 }],
  skins: [{
    name: "default",
    attachments: {
      leg: { leg: { x: 30, rotation: 0, width: 72, height: 24 } },
      body: { body: { x: 40, rotation: -90, width: 48, height: 96 } },
      arm: { arm: { x: 30, width: 64, height: 20 } },
      head: { head: { y: 20, width: 64, height: 64 } },
    },
  }],
  events: { step: { int: 3 } },
  animations: {
    idle: {
      bones: {
        torso: {
          rotate: [{ value: 0, curve: "stepped" }, { time: 0.5, value: 5 }, { time: 1, value: 0 }],
          scale: [{ x: 1, y: 1 }, { time: 1, x: 1.1 }],
          translatex: [{ value: 0 }, { time: 1, value: 3 }],
        },
        head: { shear: [{ time: 0.2, x: 3, y: 0 }], inherit: [{ time: 0.5, inherit: "noRotationOrReflection" }] },
      },
      slots: {
        head: { rgba: [{ color: "ffffffff" }, { time: 1, color: "ff8080ff" }], attachment: [{ time: 0.9, name: null }] },
        body: { alpha: [{ value: 1 }, { time: 1, value: 0.5 }] },
      },
      drawOrder: [{ time: 0.5, offsets: [{ slot: "arm", offset: -2 }] }],
      events: [{ time: 0.5, name: "step" }],
    },
  },
};
write("spine42/character-4.2.json", json(character42));

// ---------- atlas ----------
write("spine42/character-4.2.atlas",
  ["character-4.2.png", "size:256,256", "filter:Linear,Linear", "pma:true",
    "arm", "bounds:2,2,64,20",
    "body", "bounds:68,2,48,96",
    "head", "bounds:118,2,64,64", "offsets:0,0,64,64",
    "leg", "bounds:2,24,24,72", "rotate:90",
    ""].join("\n"));

// ---------- .kgbone ----------
const k = (skeleton, editor = {}) => JSON.stringify({ formatVersion: 1, generator: "KGBone sample", imagesPath: "../images", skeleton, editor }, null, "\t") + "\n";
const fullSkeleton = {
  fps: 30,
  bones: [
    { name: "root" },
    { name: "몸통", parent: "root", length: 80, y: 100, rotation: 90, color: "00ff00ff" },
    { name: "꼬리1", parent: "root", length: 64, x: -20, y: 80, rotation: 180 },
    { name: "꼬리2", parent: "꼬리1", length: 64, x: 64 },
    { name: "목표", parent: "root", x: 60, y: 40 },
    { name: "팔", parent: "몸통", length: 40, x: 60, rotation: -60 },
    { name: "손", parent: "팔", length: 30, x: 40 },
    { name: "다이얼", parent: "root", x: 100, y: 100, inherit: "onlyTranslation" },
    { name: "보석본", parent: "몸통", x: 80 },
  ],
  slots: [
    { name: "꼬리", bone: "꼬리1", attachment: "tail" },
    { name: "몸", bone: "몸통", attachment: "body", dark: "101010" },
    { name: "보석", bone: "보석본", attachment: "gem", blend: "additive" },
  ],
  constraints: [
    { type: "ik", name: "팔IK", bones: ["팔", "손"], target: "목표", bendPositive: false, volume: true },
    { type: "transform", name: "보석회전", bones: ["보석본"], source: "다이얼", localSource: true, localTarget: true, clamp: true, mixX: 0, mixY: 0, mixScaleX: 0, mixScaleY: 0, mixShearY: 0,
      properties: [{ from: "x", offset: 0, to: [{ property: "rotate", offset: 0, max: 90, scale: 2 }] }] },
    { type: "physics", name: "꼬리물리", bone: "꼬리2", rotate: 1, strength: 80, damping: 0.8, gravity: 30, volume: true },
    { type: "slider", name: "흔들기", animation: "꼬리흔들기", time: 0.5, mix: 0.75, bone: "다이얼", property: "y", propertyFrom: 0, propertyTo: 100 },
  ],
  skins: [{
    name: "default",
    attachments: {
      "꼬리": { tail: { type: "mesh", width: 128, height: 32, uvs: [0, 1, 1, 1, 1, 0, 0, 0], triangles: [0, 1, 2, 0, 2, 3], vertices: [0, -16, 128, -16, 128, 16, 0, 16], hull: 4 } },
      "몸": { body: { type: "region", x: 40, rotation: -90, width: 48, height: 96 } },
      "보석": { gem: { type: "region", width: 32, height: 32 }, "점": { type: "point", x: 5, y: 5, rotation: 30 } },
    },
  }],
  events: [{ name: "발소리", string: "쿵" }],
  animations: [
    {
      name: "꼬리흔들기",
      bones: { "꼬리1": { rotate: [{ time: 0, v: [0], curve: [0.3, 0, 0.7, 20] }, { time: 1, v: [20] }] } },
      constraints: { "보석회전": { transform: [{ time: 0, v: [1, 0, 0, 0, 0, 0] }, { time: 1, v: [0.5, 0, 0, 0, 0, 0] }] } },
      deform: { default: { "꼬리": { tail: [{ time: 0 }, { time: 0.5, offset: 2, v: [0, 5, 0, 5] }, { time: 1 }] } } },
      events: [{ time: 0.5, name: "발소리" }],
    },
  ],
};
write("kgbone/full-4.3.kgbone", k(fullSkeleton, { mode: "animate", view: { x: 10, y: -20, zoom: 1.5 }, activeAnimation: "꼬리흔들기", treeExpanded: ["bone:root", "bone:몸통"] }));
write("kgbone/bones-only.kgbone", k({ bones: [{ name: "root" }, { name: "a", parent: "root", length: 50, x: 10, y: 20, rotation: 30, scaleX: 2 }, { name: "b", parent: "a", length: 25, x: 50, rotation: -30 }] }));
// 숫자 표기·키 순서 보존 시험: 1.50 같은 표기와 일부러 바꾼 키 순서
write("kgbone/notation.kgbone",
  '{\n\t"skeleton": {\n\t\t"bones": [\n\t\t\t{\n\t\t\t\t"x": 1.50,\n\t\t\t\t"name": "root",\n\t\t\t\t"rotation": 10.0\n\t\t\t},\n\t\t\t{\n\t\t\t\t"name": "child",\n\t\t\t\t"parent": "root",\n\t\t\t\t"length": 1e2\n\t\t\t}\n\t\t],\n\t\t"x-future": { "keep": true }\n\t},\n\t"formatVersion": 1,\n\t"imagesPath": "./images"\n}\n');
// 구버전(formatVersion 0): 제약이 종류별 배열 + order
write("kgbone/legacy-v0.kgbone", JSON.stringify({
  imagesPath: "../images",
  skeleton: {
    bones: [{ name: "root" }, { name: "a", parent: "root", length: 30 }, { name: "b", parent: "a", length: 30, x: 30 }, { name: "t", parent: "root", x: 40, y: 20 }],
    ik: [{ name: "ik2", order: 1, bones: ["a", "b"], target: "t" }],
    transform: [{ name: "tc", order: 0, bones: ["t"], source: "root", mixRotate: 0 }],
  },
}, null, "\t") + "\n");
console.log("samples 생성 완료:", root);

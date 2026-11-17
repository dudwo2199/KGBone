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
// 유니티 런타임 1단계 정답 비교용: 제약·클리핑·시퀀스 없이 1단계 지원 기능을 한 파일에 모두 쓴다
// (상속 5종 + 반사 부모 + inherit 키, 회전·크기 region, 비가중치·가중치 메시 + deform, 링크드 메시, 블렌드 4종,
//  어두운 색 + rgba2·rgb2, 첨부물 키(비움 포함), 그리기 순서 키, 스킨 2개 + 스킨 전용 본, 이벤트, 계단·베지어 곡선)
const W = (a, ax, ay, aw, b, bx, by, bw) => [2, a, ax, ay, aw, b, bx, by, bw];
const unityStage1 = {
  fps: 30,
  bones: [
    { name: "root" },                                                                                    // 0
    { name: "몸", parent: "root", length: 60, y: 60, rotation: 90, scaleX: 1.2 },                         // 1
    { name: "팔-보통", parent: "몸", length: 40, x: 50, y: 10, rotation: -30, scaleY: 0.8, shearX: 5 },      // 2
    { name: "팔-이동만", parent: "몸", length: 30, x: 50, y: -10, rotation: 20, inherit: "onlyTranslation" }, // 3
    { name: "꼬리-회전없음", parent: "몸", length: 50, y: 20, rotation: 160, shearY: 10, inherit: "noRotationOrReflection" }, // 4
    { name: "모자-크기없음", parent: "몸", length: 20, x: 70, rotation: 10, inherit: "noScale" },             // 5
    { name: "반사부모", parent: "root", length: 30, x: -80, y: 40, rotation: 15, scaleX: -1 },              // 6
    { name: "반사-크기없음", parent: "반사부모", length: 20, x: 30, inherit: "noScale" },                     // 7
    { name: "반사-크기반사없음", parent: "반사부모", length: 20, x: 30, y: 10, inherit: "noScaleOrReflection" }, // 8
    { name: "반사-회전반사없음", parent: "반사부모", length: 15, x: 20, y: -10, rotation: 30, inherit: "noRotationOrReflection" }, // 9
    { name: "가중치A", parent: "root", length: 30, x: 60 },                                                // 10
    { name: "가중치B", parent: "가중치A", length: 30, x: 30, rotation: 20 },                                // 11
    { name: "빨강전용", parent: "몸", length: 10, x: 80, y: 20, skinRequired: true },                       // 12
  ],
  slots: [
    { name: "꼬리", bone: "꼬리-회전없음", attachment: "tail" },
    { name: "몸", bone: "몸", attachment: "body", color: "FFEEDDFF", dark: "202040" },
    { name: "팔", bone: "팔-보통", attachment: "arm", blend: "multiply" },
    { name: "꼬리복사", bone: "반사부모", attachment: "tail-link" },
    { name: "가중치", bone: "가중치A", attachment: "leg", blend: "screen" },
    { name: "모자", bone: "모자-크기없음", attachment: "hat-red", blend: "additive" },
    { name: "보석", bone: "빨강전용", attachment: "gem" },
    { name: "빛", bone: "반사-크기없음", attachment: "glow", dark: "000000" },
    { name: "머리", bone: "반사-크기반사없음", attachment: "head" },
    { name: "손", bone: "반사-회전반사없음", attachment: "gem" },
  ],
  skins: [
    {
      name: "default",
      attachments: {
        "꼬리": { tail: { type: "mesh", width: 128, height: 32, uvs: [0, 1, 1, 1, 1, 0, 0, 0], triangles: [0, 1, 2, 0, 2, 3], vertices: [0, -16, 128, -16, 128, 16, 0, 16], hull: 4 } },
        "몸": { body: { type: "region", x: 30, rotation: 30, scaleX: 2, width: 48, height: 96 } },
        "팔": { arm: { type: "region", x: 20, width: 64, height: 20, color: "FFFFFFC0" } },
        "꼬리복사": {
          tail: { type: "mesh", width: 128, height: 32, uvs: [0, 1, 1, 1, 1, 0, 0, 0], triangles: [0, 1, 2, 0, 2, 3], vertices: [0, -16, 128, -16, 128, 16, 0, 16], hull: 4 },
          "tail-link": { type: "linkedmesh", path: "tail", parent: "tail", timelines: true, color: "C0FFC0FF" },
        },
        "가중치": { leg: { type: "mesh", path: "leg", width: 72, height: 24, weighted: true, hull: 6,
          uvs: [0, 1, 0.5, 1, 0.5, 0, 0, 0, 1, 1, 1, 0], triangles: [0, 1, 2, 0, 2, 3, 1, 4, 5, 1, 5, 2],
          vertices: [...W(10, 0, -12, 1, 11, -30, -12, 0), ...W(10, 30, -12, 0.5, 11, 0, -12, 0.5), ...W(10, 30, 12, 0.5, 11, 0, 12, 0.5),
            ...W(10, 0, 12, 1, 11, -30, 12, 0), ...W(10, 60, -12, 0, 11, 30, -12, 1), ...W(10, 60, 12, 0, 11, 30, 12, 1)] } },
        "모자": { "hat-red": { type: "region", y: 10, width: 56, height: 24 } },
        "보석": { gem: { type: "region", width: 32, height: 32 } },
        "빛": { glow: { type: "region", width: 48, height: 48 } },
        "머리": { head: { type: "region", x: 10, width: 64, height: 64 } },
        "손": { gem: { type: "region", width: 16, height: 16 } },
      },
    },
    { name: "빨강", bones: ["빨강전용"], attachments: { "모자": { "hat-red": { type: "region", x: 5, y: 10, width: 56, height: 24, color: "FF8080FF" } } } },
  ],
  events: [{ name: "발", int: 3, float: 0.5, string: "쿵" }, { name: "숨" }],
  animations: [
    {
      name: "걷기",
      bones: {
        "몸": {
          rotate: [{ time: 0, v: [0], curve: [0.25, 0, 0.75, 10] }, { time: 0.5, v: [10], curve: "stepped" }, { time: 1, v: [0] }],
          translate: [{ time: 0, v: [0, 0] }, { time: 1, v: [10, -5] }],
          scale: [{ time: 0, v: [1, 1], curve: [0.2, 1, 0.3, 1.2, 0.1, 1, 0.4, 0.9] }, { time: 0.5, v: [1.2, 0.9] }, { time: 1, v: [1, 1] }],
          shear: [{ time: 0, v: [0, 0] }, { time: 1, v: [5, -5] }],
        },
        "팔-보통": { translatex: [{ time: 0, v: [0] }, { time: 1, v: [8] }], scaley: [{ time: 0, v: [1] }, { time: 1, v: [1.5] }], sheary: [{ time: 0, v: [0] }, { time: 1, v: [10] }] },
        "팔-이동만": { inherit: [{ time: 0, s: "onlyTranslation" }, { time: 0.5, s: "normal" }] },
        "꼬리-회전없음": { rotate: [{ time: 0, v: [0], curve: [0.2, 0, 0.4, -30] }, { time: 0.6, v: [-30] }], shearx: [{ time: 0, v: [0] }, { time: 1, v: [10] }],
          translatey: [{ time: 0, v: [0] }, { time: 1, v: [5] }], scalex: [{ time: 0, v: [1] }, { time: 1, v: [0.5] }] },
      },
      slots: {
        "몸": { rgba: [{ time: 0, v: [1, 1, 1, 1] }, { time: 1, v: [0.5, 0.2, 0.2, 0.8] }] },
        "팔": { alpha: [{ time: 0, v: [1] }, { time: 1, v: [0.3] }] },
        "모자": { rgb: [{ time: 0, v: [1, 1, 1] }, { time: 1, v: [0, 1, 0] }] },
        "빛": { rgba2: [{ time: 0, v: [1, 1, 1, 1, 0, 0, 0] }, { time: 1, v: [1, 0.5, 0.5, 1, 0.3, 0.1, 0.1] }] },
        "머리": { rgb2: [{ time: 0, v: [1, 1, 1, 0, 0, 0] }, { time: 1, v: [0.5, 0.5, 1, 0.2, 0.2, 0.4] }] },
        "손": { attachment: [{ time: 0, s: "gem" }, { time: 0.4 }, { time: 0.8, s: "gem" }] },
      },
      deform: {
        default: {
          "꼬리": { tail: [{ time: 0 }, { time: 0.5, offset: 2, v: [0, 5, 0, 5], curve: [0.25, 0, 0.75, 1] }, { time: 1 }] },
          "꼬리복사": { tail: [{ time: 0 }, { time: 1, v: [3, 0, 3, 0, 3, 0, 3, 0] }] },
          "가중치": { leg: [{ time: 0 }, { time: 1, offset: 4, v: [0, 6, 0, 6, 0, -6, 0, -6] }] },
        },
      },
      drawOrder: [
        { time: 0.25, offsets: [{ slot: "꼬리", offset: 3 }] },
        { time: 0.75, offsets: [{ slot: "머리", offset: -5 }, { slot: "팔", offset: 1 }] },
      ],
      events: [{ time: 0, name: "발" }, { time: 0.5, name: "발", int: 7, string: "쾅" }, { time: 1, name: "숨" }],
    },
    { name: "정지", bones: { "몸": { rotate: [{ time: 0, v: [5] }] } }, events: [{ time: 0, name: "숨" }] },
  ],
};
write("kgbone/unity-stage1.kgbone", k(unityStage1));

// 유니티 런타임 2단계 시험 파일: 슬라이더(시점 값·본 연결 월드/로컬·loop/clamp·additive·자기 키 건너뛰기·time/mix 키)
const lin = (pairs) => pairs.map(([time, v]) => ({ time, v: Array.isArray(v) ? v : [v] }));
const unityStage2Slider = {
  fps: 30,
  bones: [
    { name: "root" },
    { name: "팔", parent: "root", length: 40, rotation: 10 },
    { name: "다리", parent: "root", length: 30, x: 20, rotation: -20 },
    { name: "꼬리", parent: "root", length: 25, x: -20, rotation: 180 },
    { name: "목", parent: "root", length: 20, y: 30, rotation: 90 },
    { name: "다이얼", parent: "root", x: 100, y: 40 },
    { name: "다이얼-자식", parent: "다이얼", x: 10, rotation: 30 },
  ],
  constraints: [
    // ① 본 없이 time 값(키로 움직임)
    { type: "slider", name: "시점값", animation: "휘두르기", time: 0.4, mix: 0.75 },
    // ② 본 연결: 월드 y(부모 이동이 섞임)
    { type: "slider", name: "본월드", animation: "차기", bone: "다이얼", property: "y", propertyFrom: 0, propertyTo: 100, mix: 1 },
    // ③ 본 연결: 로컬 회전, loop(범위 넘는 값이 감김)
    { type: "slider", name: "본로컬반복", animation: "흔들기", bone: "다이얼-자식", property: "rotate", local: true, propertyFrom: 0, propertyTo: 20, loop: true, mix: 0.5 },
    // ④ additive, 범위 밖은 잘림
    { type: "slider", name: "더하기", animation: "끄덕", time: 5, additive: true, mix: 0.6 },
  ],
  animations: [
    // 휘두르기 안에 자기 자신(시점값)의 키: 슬라이더가 적용할 때는 건너뛴다
    { name: "휘두르기", bones: { "팔": { rotate: lin([[0, 0], [1, 90]]) } }, constraints: { "시점값": { mix: lin([[0, 0]]) } } },
    { name: "차기", bones: { "다리": { rotate: lin([[0, 0], [2, -60]]), translate: lin([[0, [0, 0]], [2, [10, 5]]]) } } },
    { name: "흔들기", bones: { "꼬리": { rotate: lin([[0, 0], [0.5, 40], [1, -40]]) } } },
    { name: "끄덕", bones: { "목": { rotate: lin([[0, 0], [1, 25]]), scale: lin([[0, [1, 1]], [1, [1.2, 0.9]]]) } } },
    // 재생하는 애니메이션: 슬라이더 값·본을 움직인다
    {
      name: "조종",
      bones: { "다이얼": { translate: lin([[0, [0, -40]], [1, [0, 30]], [2, [0, 120]]]) }, "다이얼-자식": { rotate: lin([[0, 0], [2, 50]]) } },
      constraints: { "시점값": { time: lin([[0, 0], [2, 1.2]]), mix: lin([[0, 1], [2, 0.3]]) }, "더하기": { mix: lin([[0, 0.2], [2, 1]]) } },
    },
  ],
};
write("kgbone/unity-stage2-slider.kgbone", k(unityStage2Slider));

// 유니티 런타임 2단계 시험 파일: 제약 순서(IK → 트랜스폼 → 슬라이더, 뒤가 앞 결과를 읽음), 스킨 전용 제약, 제약 키
const unityStage2Mixed = {
  fps: 30,
  bones: [
    { name: "root" },
    { name: "어깨", parent: "root", length: 40, y: 50 },
    { name: "팔꿈치", parent: "어깨", length: 35, x: 40 },
    { name: "목표", parent: "root", x: 60, y: 10 },
    { name: "따라쟁이", parent: "root", length: 20, x: -40, y: 20 },
    { name: "모자", parent: "root", length: 15, y: 100 },
    { name: "장식", parent: "root", length: 10, x: -60, skinRequired: true },
  ],
  constraints: [
    { type: "ik", name: "팔IK", bones: ["어깨", "팔꿈치"], target: "목표", bendPositive: false, softness: 4 },
    // 팔꿈치(IK 결과)의 월드 회전을 따라쟁이가 따라간다
    { type: "transform", name: "따라하기", bones: ["따라쟁이"], source: "팔꿈치", mixX: 0, mixY: 0, mixScaleX: 0, mixScaleY: 0, mixShearY: 0 },
    // 따라쟁이(트랜스폼 결과) 월드 회전으로 모자 애니메이션 시점을 정한다
    { type: "slider", name: "모자기울기", animation: "모자", bone: "따라쟁이", property: "rotate", propertyFrom: -90, propertyTo: 90, mix: 1 },
    // 스킨 전용 제약: "장식" 스킨에서만
    { type: "transform", name: "장식따라", skinRequired: true, bones: ["장식"], source: "모자", localSource: true, localTarget: true, mixRotate: 0.5, mixX: 0, mixY: 0, mixScaleX: 0, mixScaleY: 0, mixShearY: 0 },
  ],
  skins: [{ name: "default" }, { name: "장식", bones: ["장식"], constraints: ["장식따라"] }],
  animations: [
    { name: "모자", bones: { "모자": { rotate: lin([[0, -30], [1, 30]]) } } },
    {
      name: "뻗기",
      bones: { "목표": { translate: lin([[0, [0, 0]], [1, [20, 50]], [2, [-50, 30]]]) } },
      constraints: { "팔IK": { ik: [{ time: 0, v: [1, 4, -1, 0, 0] }, { time: 1, v: [0.5, 0, 1, 0, 1], curve: "stepped" }, { time: 2, v: [1, 10, 1, 0, 0] }] }, "따라하기": { transform: lin([[0, [1, 0, 0, 0, 0, 0]], [2, [0.2, 0, 0, 0, 0, 0]]]) } },
    },
  ],
};
write("kgbone/unity-stage2-mixed.kgbone", k(unityStage2Mixed));

// 유니티 런타임 2단계 시험 파일: 패스(열림·닫힘, 일정 속도 끔, 가중치 패스, 패스 변형 키, 위치·간격·회전 방식 4×3, offsetRotation, 제약 키)
const chain = (prefix, parent, n, len, x = 0, y = 0) =>
  Array.from({ length: n }, (_, i) => ({ name: `${prefix}${i}`, parent: i ? `${prefix}${i - 1}` : parent, length: len, ...(i ? { x: len } : { x, y }) }));
// 정점 3개씩 [앞 제어, 점, 뒤 제어]
const wavy = [-20, 30, 0, 0, 20, -30, 40, 40, 60, 50, 80, 60, 120, -20, 140, -10, 160, 0];
const ring = [0, -60, 40, -40, 60, 0, 60, 40, 40, 60, 0, 60, -40, 40, -60, 0, -60, -40, -40, -60];
const unityStage2Path = {
  fps: 30,
  bones: [
    { name: "root" },
    { name: "패스본", parent: "root", x: -50, y: -20, rotation: 10, scaleX: 1.1 },
    { name: "고리본", parent: "root", x: 200, y: 100 },
    { name: "가중A", parent: "root", x: -100, y: 120 },
    { name: "가중B", parent: "가중A", x: 120, rotation: 20 },
    ...chain("뱀", "root", 4, 18),
    ...chain("고리", "root", 3, 25, 200, 100),
    ...chain("줄", "root", 3, 20, -100, 120),
    ...chain("고정", "root", 2, 15, 0, -100),
  ],
  slots: [
    { name: "물결", bone: "패스본", attachment: "물결" },
    { name: "고리", bone: "고리본", attachment: "고리" },
    { name: "줄", bone: "가중A", attachment: "줄" },
  ],
  skins: [
    {
      name: "default",
      attachments: {
        "물결": { "물결": { type: "path", vertexCount: 9, vertices: wavy, lengths: [100, 200, 300] } },
        "고리": { "고리": { type: "path", closed: true, constantSpeed: false, vertexCount: 12, vertices: ring, lengths: [1, 2, 3, 4] } },
        "줄": {
          "줄": {
            type: "path", weighted: true, vertexCount: 6, lengths: [1],
            // 점마다 [본 수, (본 번호, x, y, 가중치)…]: 앞 3개는 가중A, 뒤 3개는 가중A·가중B 섞임
            vertices: [1, 3, -20, 0, 1, 1, 3, 0, 0, 1, 1, 3, 30, 10, 1, 2, 3, 90, -10, 0.5, 4, -30, -10, 0.5, 2, 3, 120, 0, 0.3, 4, 0, 0, 0.7, 1, 4, 20, 0, 1],
          },
        },
      },
    },
  ],
  constraints: [
    { type: "path", name: "뱀길", bones: ["뱀0", "뱀1", "뱀2", "뱀3"], target: "물결", position: 0.05, spacing: 4, rotateMode: "tangent", offsetRotation: 5 },
    { type: "path", name: "고리길", bones: ["고리0", "고리1", "고리2"], target: "고리", spacingMode: "percent", spacing: 0.3, position: 0.9, rotateMode: "chainScale", mixRotate: 0.8, mixX: 0.9, mixY: 1 },
    { type: "path", name: "줄길", bones: ["줄0", "줄1", "줄2"], target: "줄", positionMode: "fixed", position: 10, spacingMode: "proportional", spacing: 0.25, rotateMode: "chain" },
    { type: "path", name: "고정간격", bones: ["고정0", "고정1"], target: "물결", positionMode: "fixed", position: -30, spacingMode: "fixed", spacing: 500 },
  ],
  animations: [
    {
      name: "기어가기",
      bones: { "패스본": { rotate: lin([[0, 0], [2, 30]]) }, "가중B": { rotate: lin([[0, 0], [1, 40], [2, -20]]) } },
      deform: { default: { "물결": { "물결": [{ time: 0 }, { time: 1, offset: 6, v: [0, 30, 0, 30, 0, 30] }, { time: 2 }] } } },
      constraints: {
        "뱀길": { position: lin([[0, 0.05], [2, 0.6]]), spacing: lin([[0, 4], [2, 12]]), mix: lin([[0, [1, 1, 1]], [2, [0.5, 1, 0.8]]]) },
        "고리길": { position: lin([[0, 0.9], [2, 1.7]]) },
      },
    },
  ],
};
write("kgbone/unity-stage2-path.kgbone", k(unityStage2Path));

// 유니티 런타임 2단계 시험 파일: 물리(이동·회전·크기·기울기·부피, 바람·중력, fps 30, 속도 한계, 길이 0인 본, reset 키, 이름 "" 키, 사슬)
const unityStage2Physics = {
  fps: 30,
  referenceScale: 80,
  bones: [
    { name: "root" },
    { name: "몸", parent: "root", length: 40, y: 20, rotation: 90 },
    { name: "머리카락", parent: "몸", length: 35, x: 40, rotation: 150 },
    { name: "귀걸이", parent: "몸", x: 30, y: 10 },
    { name: "꼬리0", parent: "root", length: 20, x: -10, rotation: 200 },
    { name: "꼬리1", parent: "꼬리0", length: 20, x: 20 },
    { name: "망토", parent: "몸", length: 50, x: 10, y: -15, rotation: 180 },
  ],
  constraints: [
    { type: "physics", name: "머리", bone: "머리카락", rotate: 1, scaleX: 0.4, shearX: 0.3, volume: true, inertia: 0.8, strength: 120, damping: 0.8, gravity: 20 },
    { type: "physics", name: "귀", bone: "귀걸이", x: 1, y: 1, fps: 30, strength: 60, damping: 0.5, mass: 2, gravity: 40 },
    { type: "physics", name: "꼬리앞", bone: "꼬리0", rotate: 1, x: 0.5, inertia: 1, strength: 80, wind: 10 },
    { type: "physics", name: "꼬리뒤", bone: "꼬리1", rotate: 1, strength: 50, damping: 2, limit: 40 },
    { type: "physics", name: "망토", bone: "망토", rotate: 0.7, y: 1, strength: 30, gravity: 10, mix: 0.6 },
  ],
  animations: [
    {
      name: "뛰기",
      bones: {
        "몸": { translate: lin([[0, [0, 0]], [0.25, [30, 15]], [0.5, [60, 0]], [0.75, [30, 15]], [1, [0, 0]]]), rotate: lin([[0, 0], [0.5, 25], [1, 0]]) },
        "꼬리0": { rotate: lin([[0, 0], [0.5, -40], [1, 0]]) },
      },
      constraints: {
        "머리": { reset: [{ time: 0.6 }], strength: lin([[0, 120], [1, 200]]) },
        "": { wind: lin([[0, 0], [1, 15]]) },
        "망토": { mix: lin([[0, 0.6], [1, 1]]), gravity: lin([[0, 10], [1, 30]]) },
      },
    },
  ],
};
write("kgbone/unity-stage2-physics.kgbone", k(unityStage2Physics));
// 유니티 런타임 3단계 시험 파일: 시퀀스(모드 7종, 간격 0, 자리수 0·2, 키 없는 셋업 프레임, 메시 시퀀스)
const seqColors = [[230, 80, 60], [240, 170, 40], [90, 200, 90], [70, 140, 230]];
seqColors.forEach((c, i) => write(`images/seq/불꽃${i + 1}.png`, circle(24, c)));
[0, 1, 2].forEach((i) => write(`images/seq/별${String(i).padStart(2, "0")}.png`, rect(20, 20, seqColors[i])));
const seqModes = ["hold", "once", "loop", "pingpong", "onceReverse", "loopReverse", "pingpongReverse"];
const flame = (x, setup = 0) => ({ 불꽃: { path: "seq/불꽃", x, width: 24, height: 24, sequence: { count: 4, start: 1, digits: 0, setup } } });
const seqSlots = [...seqModes.map((m) => `s-${m}`), "s-간격0", "s-자리수", "s-키없음", "s-메시"];
const unityStage3Sequence = {
  fps: 30,
  bones: [{ name: "root" }],
  slots: seqSlots.map((name) => ({ name, bone: "root", attachment: name === "s-자리수" ? "별" : "불꽃" })),
  skins: [
    {
      name: "default",
      attachments: {
        ...Object.fromEntries(seqModes.map((m, i) => [`s-${m}`, flame(i * 30)])),
        "s-간격0": flame(210),
        "s-자리수": { 별: { path: "seq/별", x: 240, width: 20, height: 20, sequence: { count: 3, start: 0, digits: 2, setup: 1 } } },
        "s-키없음": flame(270, 2),
        "s-메시": {
          불꽃: {
            type: "mesh", path: "seq/불꽃", width: 24, height: 24, uvs: [0, 1, 1, 1, 1, 0, 0, 0], triangles: [0, 1, 2, 0, 2, 3],
            vertices: [288, -12, 312, -12, 312, 12, 288, 12], hull: 4, sequence: { count: 4, start: 1, digits: 0, setup: 0 },
          },
        },
      },
    },
  ],
  animations: [
    {
      name: "넘기기",
      bones: { root: { rotate: lin([[0, 0], [1, 0]]) } },
      sequence: {
        default: {
          ...Object.fromEntries(seqModes.map((m) => [`s-${m}`, { 불꽃: [{ time: 0, s: m, v: [0, 0.1] }, ...(m === "loop" ? [{ time: 0.6, s: m, v: [1, 0.15] }] : [])] }])),
          "s-간격0": { 불꽃: [{ time: 0, s: "loop", v: [2, 0] }] },
          "s-자리수": { 별: [{ time: 0.1, s: "once", v: [0, 0.2] }] },
          "s-메시": { 불꽃: [{ time: 0, s: "pingpong", v: [1, 0.1] }] },
        },
      },
    },
  ],
};
write("kgbone/unity-stage3-sequence.kgbone", k(unityStage3Sequence));

// 유니티 런타임 3단계 시험 파일: 클리핑(보통·반전·볼록, 오목 다각형, 가중치 메시·가중치 클립 다각형, end 지정·미지정, 클립 정점 deform 키)
const unityStage3Clipping = {
  fps: 30,
  bones: [
    { name: "root" },
    { name: "팔", parent: "root", length: 60, x: -40, rotation: 20 },
    { name: "손", parent: "팔", length: 40, x: 60, rotation: -30 },
  ],
  slots: [
    { name: "클립-보통", bone: "root", attachment: "클립" },
    { name: "몸1", bone: "root", attachment: "body" },
    { name: "다리1", bone: "팔", attachment: "가중치" },
    { name: "클립-볼록", bone: "root", attachment: "클립" },
    { name: "머리1", bone: "root", attachment: "head" },
    { name: "꼬리1", bone: "팔", attachment: "tail" },
    { name: "끝없음전", bone: "root" },
    { name: "클립-반전", bone: "root", attachment: "클립" },
    { name: "몸2", bone: "손", attachment: "body" },
    { name: "머리2", bone: "root", attachment: "head" },
  ],
  skins: [
    {
      name: "default",
      attachments: {
        // 오목(L자) 다각형, 다리1까지
        "클립-보통": { 클립: { type: "clipping", end: "다리1", vertexCount: 6, vertices: [-30, -40, 40, -40, 40, -10, 0, -10, 0, 50, -30, 50] } },
        "몸1": { body: { x: 10, y: 0, rotation: 15, width: 48, height: 96 } },
        "다리1": {
          가중치: { type: "mesh", path: "leg", width: 72, height: 24, weighted: true, hull: 4,
            uvs: [0, 1, 1, 1, 1, 0, 0, 0], triangles: [0, 1, 2, 0, 2, 3],
            vertices: [...W(1, 0, -12, 1, 2, -60, -12, 0), ...W(1, 60, -12, 0.3, 2, 0, -12, 0.7), ...W(1, 60, 12, 0.3, 2, 0, 12, 0.7), ...W(1, 0, 12, 1, 2, -60, 12, 0)] },
        },
        // 가중치 클립 다각형 + 볼록 옵션(오목한 오각형의 껍질로 자름), 꼬리1까지
        "클립-볼록": { 클립: { type: "clipping", end: "꼬리1", convex: true, weighted: true, vertexCount: 5,
          vertices: [...W(0, -50, -30, 0.5, 1, -10, -50, 0.5), ...W(0, 50, -30, 1, 1, 90, -50, 0), ...W(0, 50, 40, 1, 1, 90, 20, 0), ...W(0, 0, 10, 0.5, 1, 40, -10, 0.5), ...W(0, -50, 40, 1, 1, -10, 20, 0)] } },
        "머리1": { head: { x: 20, y: 20 } },
        "꼬리1": { tail: { x: 30, rotation: -10 } },
        // 반전 + 볼록(조각 하나), end 없음(마지막 슬롯까지). 오목 다각형 반전은 조각 경계에 넓이≈0 조각이 생겨 유니티(Mono)와 끝자리가 갈리므로 dotnet 단위 테스트(m18)로만 본다
        "클립-반전": { 클립: { type: "clipping", inverse: true, convex: true, vertexCount: 4, vertices: [-15, -15, 15, -15, 15, 15, -15, 15] } },
        "몸2": { body: { x: 0, y: 0, width: 48, height: 96 } },
        "머리2": { head: { x: -10, y: 5 } },
      },
    },
  ],
  animations: [
    {
      name: "흔들기",
      bones: { "팔": { rotate: lin([[0, 0], [0.5, 40], [1, -20]]) }, "손": { rotate: lin([[0, 0], [1, 60]]) } },
      deform: { default: { "클립-보통": { 클립: [{ time: 0 }, { time: 1, offset: 2, v: [10, 5, 0, 20] }] } } },
    },
  ],
};
write("kgbone/unity-stage3-clipping.kgbone", k(unityStage3Clipping));

// 유니티 런타임 3단계 시험 파일(정답 대상 아님): 트랙·섞기 컴포넌트 테스트용. 걷기·달리기(다리·팔), 공격(팔만, 이벤트 "타격"), 대기
const unityStage3Tracks = {
  fps: 30,
  bones: [
    { name: "root" },
    { name: "몸", parent: "root", length: 60, y: 50, rotation: 90 },
    { name: "팔", parent: "몸", length: 40, x: 50, rotation: -100 },
    { name: "다리", parent: "root", length: 50, rotation: -90 },
  ],
  slots: [
    { name: "다리", bone: "다리", attachment: "leg" },
    { name: "몸", bone: "몸", attachment: "body" },
    { name: "팔", bone: "팔", attachment: "arm" },
    { name: "무기", bone: "팔", attachment: "칼" },
    { name: "갑옷", bone: "몸", attachment: "갑옷" },
  ],
  skins: [
    {
      name: "default",
      attachments: {
        "다리": { leg: { x: 36, rotation: 0, width: 72, height: 24 } },
        "몸": { body: { x: 30, rotation: -90, width: 48, height: 96 } },
        "팔": { arm: { x: 20, width: 64, height: 20 } },
        "무기": { 칼: { path: "hat-red", x: 40, width: 56, height: 24 }, 도끼: { path: "hat-blue", x: 40, width: 56, height: 24 } },
      },
    },
    // 무기 바꾸기용: 몸 스킨(갑옷), 칼 스킨(무기 칼을 꼬리 그림으로 바꿈)
    { name: "몸", attachments: { "갑옷": { 갑옷: { path: "gem", x: 30, width: 32, height: 32 } } } },
    { name: "칼", attachments: { "무기": { 칼: { path: "tail", x: 60, width: 128, height: 32 } } } },
  ],
  events: [{ name: "발" }, { name: "타격", int: 3 }, { name: "끝" }],
  animations: [
    { name: "걷기", bones: { "다리": { rotate: lin([[0, 0], [0.5, 30], [1, 0]]) }, "팔": { rotate: lin([[0, 0], [0.5, -20], [1, 0]]) } }, events: [{ time: 0.5, name: "발" }] },
    { name: "달리기", bones: { "다리": { rotate: lin([[0, 0], [0.3, 60], [0.6, 0]]) }, "팔": { rotate: lin([[0, 0], [0.3, -40], [0.6, 0]]) } } },
    { name: "공격", bones: { "팔": { rotate: lin([[0, 0], [0.2, 90], [0.4, 90]]) } }, events: [{ time: 0.2, name: "타격" }, { time: 0.4, name: "끝" }] },
    { name: "대기", bones: { "몸": { translate: lin([[0, [0, 0]], [1, [0, 4]]]) } } },
  ],
};
write("kgbone/unity-stage3-tracks.kgbone", k(unityStage3Tracks));

console.log("samples 생성 완료:", root);

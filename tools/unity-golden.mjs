// 유니티 런타임 정답 파일 생성기: KGBone 핵심(kgbone.wasm)이 계산한 포즈를 시험 파일·시점마다 저장한다.
// 유니티 런타임 계산부(C#)는 이 정답과 같아야 한다(unity/com.kgbone.runtime/Tests/Editor/GoldenTests.cs).
// 사용: npm run unity:golden   (먼저 npm run wasm 또는 npm run wasm:fetch)
// 출력: unity/com.kgbone.runtime/Tests/Golden/<이름>.kgbone, <이름>.json — 같은 입력이면 같은 바이트(결정적)
import { mkdirSync, readFileSync, writeFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { loadCore } from "./mcp/core.mjs";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(repo, "unity", "com.kgbone.runtime", "Tests", "Golden");

// 대상: 클리핑·시퀀스가 없는 시험 파일. 제약은 유니티 런타임이 계산하는 종류만(2단계 설계 문서 5.6)
const TARGETS = [
  ["bones-only", "samples/kgbone/bones-only.kgbone"],
  ["notation", "samples/kgbone/notation.kgbone"],
  ["unity-stage1", "samples/kgbone/unity-stage1.kgbone"],
  ["spine43-bones-only", "samples/spine43/bones-only.json"],
  ["spine43-mesh-weights", "samples/spine43/mesh-weights.json"],
  ["spine43-skins", "samples/spine43/skins.json"],
  ["spine43-events", "samples/spine43/events.json"],
  ["spine43-ik", "samples/spine43/ik.json"],
  ["spine43-transform", "samples/spine43/transform.json"],
  ["unity-stage2-slider", "samples/kgbone/unity-stage2-slider.kgbone"],
  ["unity-stage2-mixed", "samples/kgbone/unity-stage2-mixed.kgbone"],
  ["spine43-path", "samples/spine43/path.json"],
  ["unity-stage2-path", "samples/kgbone/unity-stage2-path.kgbone"],
  // 물리: 한 엔진으로 연속 요청한 결과를 프레임마다 저장한다(stateful, 아래 sequence)
  ["spine43-physics", "samples/spine43/physics.json", { stateful: true }],
  ["unity-stage2-physics", "samples/kgbone/unity-stage2-physics.kgbone", { stateful: true }],
];

/** 애니메이션 길이 d에 대한 요청들(시점, 반복 재생, 이벤트 구간) */
function requests(skin, anim, d) {
  const r = (time, lastTime, looping) => ({ animation: anim, time, lastTime, skin, looping });
  if (anim === null) return [r(0, null, false)];
  return [
    r(0, null, false), r(d / 3, null, false), r(d / 2, null, false), r((2 * d) / 3, null, false), r(d, null, false),
    r(1.5 * d, null, true),
    r(d / 2, 0, false),
    r(1.2 * d, 0.9 * d, true),
    r(0.1 * d, 0.9 * d, true),
  ];
}

/**
 * 물리 연속 요청(애니메이션마다, 시작 전에 물리 초기화): 1/60초 × 120프레임(반복 재생, lastTime 포함) →
 * "reset" 한 번 → 불규칙 간격(0.004, 0.033, 0.1, 0, 0.017) × 12 = 60프레임.
 */
function sequence(skin, anim) {
  const out = [];
  let t = 0;
  let last = null;
  const push = (physics, dt, extra = {}) => {
    const time = Math.fround(t + dt);
    out.push({ animation: anim, time, lastTime: last, skin, looping: true, physics, ...extra });
    last = time;
    t = time;
  };
  for (let i = 0; i < 120; i++) push(1 / 60, 1 / 60, i === 0 ? { resetPhysics: true } : {});
  push("reset", 0);
  for (let r = 0; r < 12; r++) for (const dt of [0.004, 0.033, 0.1, 0, 0.017]) push(dt, dt);
  return out;
}

const pick = (o) => ({ bones: o.bones, active: o.active, draw: o.draw, events: o.events });

const core = await loadCore();
mkdirSync(outDir, { recursive: true });
for (const f of readdirSync(outDir)) if (f.endsWith(".json") || f.endsWith(".kgbone")) rmSync(join(outDir, f));

let total = 0;
for (const [name, rel, opts = {}] of TARGETS) {
  const text = readFileSync(join(repo, rel), "utf8");
  if (rel.endsWith(".kgbone")) core.call({ method: "openKgbone", text });
  else core.call({ method: "importSpineJson", text });
  // C# 쪽은 .kgbone만 읽으므로 핵심이 저장한 텍스트를 함께 둔다
  const kgbone = core.call({ method: "saveKgbone" }).text;
  core.call({ method: "openKgbone", text: kgbone });
  const project = core.project();
  const skins = [null, ...project.skeleton.skins.map((s) => s.name).filter((s) => s !== "default")];
  const anims = project.skeleton.animations.map((a) => a.name);
  const durations = Object.fromEntries(
    anims.map((a) => {
      // 길이는 키 시간의 최댓값(핵심 duration과 같은 규칙)을 JSON에서 직접 잰다
      const an = project.skeleton.animations.find((x) => x.name === a);
      let d = 0;
      const see = (keys) => keys?.length && (d = Math.max(d, keys[keys.length - 1].time));
      for (const g of [an.bones, an.slots, an.constraints]) for (const tl of Object.values(g ?? {})) for (const keys of Object.values(tl)) see(keys);
      for (const g of [an.deform, an.sequence]) for (const s of Object.values(g ?? {})) for (const sl of Object.values(s)) for (const keys of Object.values(sl)) see(keys);
      for (const k of an.drawOrder ?? []) d = Math.max(d, k.time);
      for (const k of an.events ?? []) d = Math.max(d, k.time);
      return [a, d];
    }),
  );
  const cases = [];
  if (opts.stateful) {
    for (const skin of skins) {
      for (const anim of [null, ...anims]) {
        for (const request of sequence(skin, anim)) {
          if (request.resetPhysics) core.call({ method: "resetPhysics" });
          const output = core.call({ method: "pose", request });
          cases.push({ request, output: pick(output) });
        }
      }
    }
  }
  for (const skin of opts.stateful ? [] : skins) {
    for (const anim of [null, ...anims]) {
      for (const request of requests(skin, anim, anim ? durations[anim] : 0)) {
        const output = core.call({ method: "pose", request: { ...request, physics: "none" } });
        cases.push({ request, output: pick(output) });
      }
    }
  }
  writeFileSync(join(outDir, `${name}.kgbone`), kgbone);
  const body = cases.map((c) => "\t\t" + JSON.stringify(c)).join(",\n");
  const stateful = opts.stateful ? `\n\t"stateful": true,` : "";
  writeFileSync(join(outDir, `${name}.json`), `{\n\t"kgbone": ${JSON.stringify(name + ".kgbone")},\n\t"source": ${JSON.stringify(rel)},${stateful}\n\t"cases": [\n${body}\n\t]\n}\n`);
  total += cases.length;
  console.log(`${name}: 케이스 ${cases.length}개`);
}
console.log(`정답 파일 ${TARGETS.length}개, 케이스 ${total}개 → ${outDir}`);

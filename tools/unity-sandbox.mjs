// 확인용 유니티 프로젝트(unity/KGBoneSandbox)에 시험 파일을 복사한다. Assets/Samples/는 커밋하지 않는다.
// 사용: npm run unity:sandbox [-- 추가 폴더...]   예) npm run unity:sandbox -- ../../local/golem
//   - 저장소 samples/의 1단계 시험 파일(제약 없는 것)과 이미지 → Assets/Samples/kgbone/, Assets/Samples/images/
//   - 추가 폴더(.kgbone과 그 이미지 폴더가 든 폴더)는 Assets/Samples/<폴더 이름>/으로 통째로 복사
import { cpSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
const dest = join(repo, "unity", "KGBoneSandbox", "Assets", "Samples");
const STAGE1 = ["bones-only.kgbone", "unity-stage1.kgbone"];

rmSync(dest, { recursive: true, force: true });
mkdirSync(join(dest, "kgbone"), { recursive: true });
for (const f of STAGE1) cpSync(join(repo, "samples", "kgbone", f), join(dest, "kgbone", f));
cpSync(join(repo, "samples", "images"), join(dest, "images"), { recursive: true });
console.log(`시험 파일 ${STAGE1.length}개와 이미지 ${readdirSync(join(dest, "images")).length}장 → ${dest}`);

for (const extra of process.argv.slice(2)) {
  const src = resolve(extra);
  if (!existsSync(src)) {
    console.error(`폴더가 없습니다: ${src}`);
    process.exit(1);
  }
  cpSync(src, join(dest, basename(src)), { recursive: true, filter: (p) => !p.includes(`${basename(src)}\\frames`) && !p.endsWith(".gif") });
  console.log(`추가: ${src} → ${join(dest, basename(src))}`);
}

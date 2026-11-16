// 확인용 유니티 프로젝트(unity/KGBoneSandbox)에서 EditMode 테스트를 돌리고 결과를 요약한다.
// 사용: npm run unity:editmode [-- 거르기]   (UNITY_EXE로 유니티 경로를 바꿀 수 있다. 같은 프로젝트를 에디터로 열어 두면 실패한다)
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..");
const project = join(repo, "unity", "KGBoneSandbox");
const exe = process.env.UNITY_EXE || "C:/Program Files/Unity/Hub/Editor/6000.3.17f1/Editor/Unity.exe";
const results = join(project, "Logs", "editmode.xml");
const log = join(project, "Logs", "editmode.log");
const filter = process.argv[2];
if (existsSync(results)) rmSync(results);

const args = ["-batchmode", "-nographics", "-projectPath", project, "-runTests", "-testPlatform", "EditMode", "-testResults", results, "-logFile", log];
if (filter) args.push("-testFilter", filter);
const r = spawnSync(exe, args, { stdio: "inherit" });

if (!existsSync(results)) {
  const text = existsSync(log) ? readFileSync(log, "utf8") : "";
  const errors = [...new Set(text.split(/\r?\n/).filter((l) => /error CS|Scripts have compiler errors|Exception/.test(l)))].slice(0, 20);
  console.error(`테스트 결과 파일이 없습니다(종료 코드 ${r.status}). 로그: ${log}\n` + errors.join("\n"));
  process.exit(1);
}
const xml = readFileSync(results, "utf8");
const run = /<test-run[^>]*total="(\d+)"[^>]*passed="(\d+)"[^>]*failed="(\d+)"/.exec(xml);
const failed = [...xml.matchAll(/<test-case[^>]*fullname="([^"]+)"[^>]*result="Failed"[^>]*>[\s\S]*?<message><!\[CDATA\[([\s\S]*?)\]\]><\/message>/g)];
for (const [, name, msg] of failed) console.log(`실패 ${name}\n  ${msg.trim().split("\n")[0]}`);
console.log(run ? `EditMode: 전체 ${run[1]}, 통과 ${run[2]}, 실패 ${run[3]}` : "결과를 읽지 못했습니다");
process.exit(run && run[3] === "0" ? 0 : 1);

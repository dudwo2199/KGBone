// KGBone MCP 서버 배포 묶음(zip) 만들기. 저장소를 통째로 받지 않고 이것만 풀어 쓰게 한다.
// 사용: node tools/pack-mcp.mjs [버전, 기본 package.json]   → dist-mcp/kgbone-mcp-<버전>.zip
// 묶음 안 구조(서버가 ../../interface, ../../app/public을 찾으므로 그대로 둔다):
//   kgbone-mcp/README.md, tools/mcp/*.mjs(테스트 제외), interface/commands.json, app/public/kgbone.wasm
// app/public/kgbone.wasm이 먼저 있어야 한다(npm run wasm). zip은 외부 도구 없이 Node로 쓴다.
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { crc32, deflateRawSync } from 'node:zlib';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const version = (process.argv[2] ?? JSON.parse(readFileSync(join(repo, 'package.json'), 'utf8')).version).replace(/^mcp-v/, '');
const ROOT = 'kgbone-mcp';

const wasm = join(repo, 'app', 'public', 'kgbone.wasm');
if (!existsSync(wasm)) {
  console.error('app/public/kgbone.wasm이 없습니다. 먼저 npm run wasm(또는 npm run wasm:fetch)을 실행하세요.');
  process.exit(1);
}

/** [묶음 안 경로, 내용] */
const entries = [];
for (const f of readdirSync(join(repo, 'tools', 'mcp')).sort()) {
  if (f.endsWith('.mjs') && !f.endsWith('.test.mjs')) entries.push([`tools/mcp/${f}`, readFileSync(join(repo, 'tools', 'mcp', f))]);
}
entries.push(['interface/commands.json', readFileSync(join(repo, 'interface', 'commands.json'))]);
entries.push(['app/public/kgbone.wasm', readFileSync(wasm)]);
entries.unshift(['README.md', Buffer.from(readme(version), 'utf8')]);

const out = join(repo, 'dist-mcp');
mkdirSync(out, { recursive: true });
const zipPath = join(out, `kgbone-mcp-${version}.zip`);
writeFileSync(zipPath, zip(entries.map(([p, data]) => [`${ROOT}/${p}`, data])));
writeFileSync(join(out, 'NOTES.md'), notes(version));
console.log(`만듦: ${zipPath} (파일 ${entries.length}개)`);

// ---------- zip(Deflate, UTF-8 이름) ----------
function zip(files) {
  const locals = [];
  const centrals = [];
  let offset = 0;
  // 1980-01-01 00:00 (재현 가능한 묶음)
  const time = 0;
  const date = (0 << 9) | (1 << 5) | 1;
  for (const [name, data] of files) {
    const nameBuf = Buffer.from(name, 'utf8');
    const packed = deflateRawSync(data, { level: 9 });
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6); // UTF-8 이름
    local.writeUInt16LE(8, 8); // deflate
    local.writeUInt16LE(time, 10);
    local.writeUInt16LE(date, 12);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(packed.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(nameBuf.length, 26);
    local.writeUInt16LE(0, 28);
    locals.push(local, nameBuf, packed);

    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50, 0);
    central.writeUInt16LE(20, 4);
    central.writeUInt16LE(20, 6);
    central.writeUInt16LE(0x0800, 8);
    central.writeUInt16LE(8, 10);
    central.writeUInt16LE(time, 12);
    central.writeUInt16LE(date, 14);
    central.writeUInt32LE(crc, 16);
    central.writeUInt32LE(packed.length, 20);
    central.writeUInt32LE(data.length, 24);
    central.writeUInt16LE(nameBuf.length, 28);
    central.writeUInt32LE(offset, 42);
    centrals.push(central, nameBuf);
    offset += local.length + nameBuf.length + packed.length;
  }
  const centralBuf = Buffer.concat(centrals);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(files.length, 8);
  end.writeUInt16LE(files.length, 10);
  end.writeUInt32LE(centralBuf.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, centralBuf, end]);
}

// ---------- 안내 글 ----------
function readme(v) {
  return `# KGBone MCP 서버 ${v}

AI 프로그램(Claude Code, Claude 데스크톱, Cursor, Codex, VS Code Copilot, Gemini CLI 등)이
KGBone 문서를 편집하게 해 주는 MCP 서버입니다. KGBone 창(도구 → MCP 연결)을 이으면 열린 창의 문서를 고칩니다.

## 필요한 것
- Node.js 22 이상 (https://nodejs.org)
- KGBone 화면: https://dudwo2199.github.io/KGBone/ 를 Edge로 열어 "앱 설치"

## 설치
1. 이 zip을 원하는 곳에 풉니다. 예: \`D:/tools/kgbone-mcp\`
   (안의 폴더 구조는 바꾸지 마세요. 서버가 \`interface/\`, \`app/public/\`을 찾습니다)
2. AI 프로그램에 등록합니다. 아래 \`D:/tools/kgbone-mcp\`은 푼 폴더, \`D:/내작업\`은 AI가 파일을 읽고 쓸 폴더로 바꿉니다.

**Claude Code** (모든 폴더에서 쓰기)
\`\`\`
claude mcp add kgbone -s user -e KGBONE_MCP_ROOTS=D:/내작업 -- node D:/tools/kgbone-mcp/tools/mcp/server.mjs
\`\`\`

**Claude 데스크톱 / Cursor(.cursor/mcp.json) / Gemini CLI(~/.gemini/settings.json)**
\`\`\`json
{
  "mcpServers": {
    "kgbone": {
      "command": "node",
      "args": ["D:/tools/kgbone-mcp/tools/mcp/server.mjs"],
      "env": { "KGBONE_MCP_ROOTS": "D:/내작업" }
    }
  }
}
\`\`\`

**VS Code (.vscode/mcp.json)**
\`\`\`json
{ "servers": { "kgbone": { "type": "stdio", "command": "node", "args": ["D:/tools/kgbone-mcp/tools/mcp/server.mjs"], "env": { "KGBONE_MCP_ROOTS": "D:/내작업" } } } }
\`\`\`

**Codex (~/.codex/config.toml)**
\`\`\`toml
[mcp_servers.kgbone]
command = "node"
args = ["D:/tools/kgbone-mcp/tools/mcp/server.mjs"]
env = { KGBONE_MCP_ROOTS = "D:/내작업" }
\`\`\`

## 쓰기
1. AI 프로그램을 켭니다(MCP 서버가 함께 켜져 127.0.0.1:47821에 연결 창구를 엽니다).
2. KGBone에서 도구 → MCP 연결… → [연결]. "창을 열 때 자동으로 연결"을 켜 두면 다음부터 생략.
3. 말로 시킵니다. 예: "팔 본 두 개 만들고 화면 맞춰줘", "지금 화면 찍어서 보여줘", "D:/내작업/char.kgbone 열어줘".
- 연결 상태는 AI에게 "KGBone 연결 상태 확인해줘".
- 창을 잇지 않으면 서버 안의 창 없는 문서를 고칩니다(.kgbone으로 저장해 KGBone에서 엽니다).

## 환경 변수
| 이름 | 뜻 |
|---|---|
| KGBONE_MCP_ROOTS | 파일을 읽고 쓸 폴더 추가(\`;\`로 구분). 기본은 서버를 띄운 폴더뿐 |
| KGBONE_MCP_PORT | 연결 창구 포트(기본 47821, 창의 포트 칸과 같아야 함) |
| KGBONE_MCP_LIVE=0 | 창 연결 끄기 |
| KGBONE_MCP_ORIGINS | 창 연결을 허용할 웹 주소 추가 |

## 새 판으로 바꿀 때
zip을 같은 곳에 덮어 풀고 AI 프로그램의 MCP 서버를 다시 켭니다(Claude Code는 /mcp에서 다시 연결 또는 새 세션, 데스크톱 앱은 다시 시작).

자세한 설명: https://github.com/dudwo2199/KGBone/blob/main/manual/09-ai-connect.md
`;
}

function notes(v) {
  return `KGBone MCP 서버 ${v}

- \`kgbone-mcp-${v}.zip\`을 풀고, 안의 README.md대로 AI 프로그램에 등록합니다(Node.js 22 이상 필요).
- KGBone 화면(https://dudwo2199.github.io/KGBone/)에서 도구 → MCP 연결을 누르면 AI가 열린 창의 문서를 고칩니다.
- 설명서: manual/09-ai-connect.md
`;
}

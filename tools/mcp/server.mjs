// KGBone MCP 서버: 표준 입출력으로 한 줄에 JSON-RPC 메시지 하나씩 주고받는다.
// kgbone.wasm을 Node에서 직접 돌리므로 exe가 필요 없다(스마트 앱 컨트롤에 걸리지 않는다).
// KGBone 창(도구 → MCP 연결)이 붙어 있으면 그 창의 문서를, 아니면 이 서버 안의 창 없는 문서를 고친다(link.mjs).
// 실행: node tools/mcp/server.mjs   (등록: 저장소 루트의 .mcp.json)
// 환경 변수: KGBONE_MCP_ROOTS(파일 허용 폴더 추가), KGBONE_MCP_PORT(창 연결 포트, 기본 47821),
//           KGBONE_MCP_LIVE=0(창 연결 끄기), KGBONE_MCP_ORIGINS(창 연결을 허용할 웹 주소 추가), KGBONE_WASM(wasm 경로)
import { createInterface } from "node:readline";
import { pathToFileURL } from "node:url";
import { loadCore } from "./core.mjs";
import { Link } from "./link.mjs";
import { LiveSession } from "./live.mjs";
import { allowedRoots, EditorSession } from "./session.mjs";
import { TargetGuard } from "./target.mjs";
import { callTool, listTools } from "./tools.mjs";

const PROTOCOL_VERSIONS = ["2025-06-18", "2025-03-26", "2024-11-05"];
const INSTRUCTIONS =
  "KGBone은 2D 스켈레탈 애니메이션 에디터다. 이 서버는 문서 하나를 들고 있다. " +
  "흐름: doc_new 또는 doc_open → command_list로 명령 찾기 → command_run / command_batch로 편집 → doc_inspect·pose·render로 확인 → doc_save. " +
  "사용자가 KGBone 창에서 도구 → MCP 연결을 눌러 두면 열린 창의 문서를 고친다(화면에 바로 보임, view_screenshot으로 화면 확인). 연결 상태는 app_status. " +
  "좌표는 y가 위로 커지고 각도는 도(반시계)다. 파일은 서버를 띄운 폴더 안에서만 읽고 쓴다.";

/**
 * 요청 하나를 처리해 응답(알림이면 null)을 돌려준다. 테스트에서 직접 부른다.
 * session: 세션 또는 세션을 고르는 함수. ctx.link: 창 연결, ctx.onInitialize(clientInfo)
 */
export async function handleMessage(session, msg, ctx = {}) {
  const reply = (result) => ({ jsonrpc: "2.0", id: msg.id, result });
  const fail = (code, message) => ({ jsonrpc: "2.0", id: msg.id ?? null, error: { code, message } });
  if (!msg || msg.jsonrpc !== "2.0" || typeof msg.method !== "string") return fail(-32600, "잘못된 요청입니다");
  // id가 없으면 알림: 답하지 않는다
  if (msg.id === undefined) return null;
  switch (msg.method) {
    case "initialize": {
      const asked = msg.params?.protocolVersion;
      ctx.onInitialize?.(msg.params?.clientInfo);
      return reply({
        protocolVersion: PROTOCOL_VERSIONS.includes(asked) ? asked : PROTOCOL_VERSIONS[0],
        capabilities: { tools: {} },
        serverInfo: { name: "kgbone", version: "0.2.0" },
        instructions: INSTRUCTIONS,
      });
    }
    case "ping":
      return reply({});
    case "tools/list":
      return reply({ tools: listTools() });
    case "tools/call": {
      const s = typeof session === "function" ? session() : session;
      // 창 연결이 끊기거나 붙어 대상 문서가 바뀌었으면 알린다(ctx.guard: TargetGuard)
      const turn = ctx.guard?.check(s.where, msg.params?.name) ?? {};
      if (turn.block) return reply({ content: [{ type: "text", text: turn.block }], isError: true });
      const result = await callTool(s, msg.params?.name, msg.params?.arguments, ctx);
      if (turn.note) result.content = [{ type: "text", text: turn.note }, ...result.content];
      return reply(result);
    }
    default:
      return fail(-32601, `없는 메서드입니다: ${msg.method}`);
  }
}

export async function createSession(wasmPath) {
  const session = new EditorSession(await loadCore(wasmPath));
  session.newDocument();
  return session;
}

/** MCP 클라이언트가 알려 준 이름(Claude Code, Cursor, Codex …) */
function clientName(info) {
  const n = typeof info?.name === "string" ? info.name.trim() : "";
  return n ? n.slice(0, 60) : null;
}

async function main() {
  // wasm을 못 찾아도 서버는 뜬다. 도구를 부를 때마다 다시 찾아보므로, 나중에 wasm을 넣으면 다시 연결하지 않아도 된다
  let session = null;
  let startError = null;
  const ensureSession = async () => {
    if (session) return;
    try {
      session = await createSession();
      startError = null;
    } catch (e) {
      startError = e instanceof Error ? e.message : String(e);
    }
  };
  await ensureSession();

  // 창 연결: 기본으로 켠다(KGBONE_MCP_LIVE=0이면 끈다)
  const link = process.env.KGBONE_MCP_LIVE === "0" ? null : new Link({ name: "AI", roots: allowedRoots() });
  if (link) void link.start();
  let live = null;
  const pick = () => {
    if (link?.pageConnected && session) return (live ??= new LiveSession(link, session));
    return session;
  };
  const ctx = { link, guard: new TargetGuard(), onInitialize: (info) => link?.setName(clientName(info)) };

  // 요청은 받은 차례대로 하나씩 처리한다(창에 보내는 명령의 순서가 섞이지 않게)
  let queue = Promise.resolve();
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
  lines.on("line", (line) => {
    if (!line.trim()) return;
    queue = queue.then(async () => {
      let out;
      try {
        const msg = JSON.parse(line);
        if (msg.method === "tools/call") await ensureSession();
        const needsCore = msg.method === "tools/call" && msg.id !== undefined && msg.params?.name !== "app_status";
        out =
          !session && needsCore
            ? { jsonrpc: "2.0", id: msg.id, result: { content: [{ type: "text", text: startError }], isError: true } }
            : await handleMessage(session ? pick : () => ({ where: "headless" }), msg, ctx);
      } catch {
        out = { jsonrpc: "2.0", id: null, error: { code: -32700, message: "JSON을 읽지 못했습니다" } };
      }
      if (out) process.stdout.write(`${JSON.stringify(out)}\n`);
    });
  });
  lines.on("close", () => {
    void queue.then(() => {
      link?.close();
      process.exit(0);
    });
  });
}

// 직접 실행했을 때만 서버를 띄운다(테스트에서 불러올 때는 띄우지 않는다)
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();

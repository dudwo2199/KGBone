// KGBone MCP 서버(창 없음): 표준 입출력으로 한 줄에 JSON-RPC 메시지 하나씩 주고받는다.
// kgbone.wasm을 Node에서 직접 돌리므로 exe가 필요 없다(스마트 앱 컨트롤에 걸리지 않는다).
// 실행: node tools/mcp/server.mjs   (등록: 저장소 루트의 .mcp.json)
import { createInterface } from "node:readline";
import { pathToFileURL } from "node:url";
import { loadCore } from "./core.mjs";
import { EditorSession } from "./session.mjs";
import { callTool, listTools } from "./tools.mjs";

const PROTOCOL_VERSIONS = ["2025-06-18", "2025-03-26", "2024-11-05"];
const INSTRUCTIONS =
  "KGBone은 2D 스켈레탈 애니메이션 에디터다. 이 서버는 창 없이 문서 하나를 들고 있다. " +
  "흐름: doc_new 또는 doc_open → command_list로 명령 찾기 → command_run / command_batch로 편집 → doc_inspect·pose·render로 확인 → doc_save. " +
  "좌표는 y가 위로 커지고 각도는 도(반시계)다. 파일은 서버를 띄운 폴더 안에서만 읽고 쓴다.";

/** 요청 하나를 처리해 응답(알림이면 null)을 돌려준다. 테스트에서 직접 부른다 */
export function handleMessage(session, msg) {
  const reply = (result) => ({ jsonrpc: "2.0", id: msg.id, result });
  const fail = (code, message) => ({ jsonrpc: "2.0", id: msg.id ?? null, error: { code, message } });
  if (!msg || msg.jsonrpc !== "2.0" || typeof msg.method !== "string") return fail(-32600, "잘못된 요청입니다");
  // id가 없으면 알림: 답하지 않는다
  if (msg.id === undefined) return null;
  switch (msg.method) {
    case "initialize": {
      const asked = msg.params?.protocolVersion;
      return reply({
        protocolVersion: PROTOCOL_VERSIONS.includes(asked) ? asked : PROTOCOL_VERSIONS[0],
        capabilities: { tools: {} },
        serverInfo: { name: "kgbone", version: "0.1.0" },
        instructions: INSTRUCTIONS,
      });
    }
    case "ping":
      return reply({});
    case "tools/list":
      return reply({ tools: listTools() });
    case "tools/call":
      return reply(callTool(session, msg.params?.name, msg.params?.arguments));
    default:
      return fail(-32601, `없는 메서드입니다: ${msg.method}`);
  }
}

export async function createSession(wasmPath) {
  const session = new EditorSession(await loadCore(wasmPath));
  session.newDocument();
  return session;
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
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of lines) {
    if (!line.trim()) continue;
    let out;
    try {
      const msg = JSON.parse(line);
      if (msg.method === "tools/call") await ensureSession();
      out =
        !session && msg.method === "tools/call" && msg.id !== undefined
          ? { jsonrpc: "2.0", id: msg.id, result: { content: [{ type: "text", text: startError }], isError: true } }
          : handleMessage(session, msg);
    } catch {
      out = { jsonrpc: "2.0", id: null, error: { code: -32700, message: "JSON을 읽지 못했습니다" } };
    }
    if (out) process.stdout.write(`${JSON.stringify(out)}\n`);
  }
}

// 직접 실행했을 때만 서버를 띄운다(테스트에서 불러올 때는 띄우지 않는다)
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();

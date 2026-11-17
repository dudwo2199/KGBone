// 창 연결(link.mjs) 테스트: 중계소, 여러 MCP 서버 동시 연결, 웹 주소 검사, 중계소 이어받기,
// 그리고 stdio 서버가 창이 붙었을 때 창으로 일을 보내는지(가짜 창: 이 테스트 안의 핵심으로 답한다).
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { request } from "node:http";
import { createServer } from "node:net";
import { dirname, join } from "node:path";
import { after, test } from "node:test";
import { fileURLToPath } from "node:url";
import { loadCore } from "./core.mjs";
import { allowedOrigin, Link } from "./link.mjs";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const cleanups = [];
after(() => cleanups.forEach((f) => f()));

const freePort = () =>
  new Promise((resolve) => {
    const s = createServer();
    s.listen(0, "127.0.0.1", () => {
      const { port } = s.address();
      s.close(() => resolve(port));
    });
  });

const until = async (cond, ms = 5000) => {
  const end = Date.now() + ms;
  while (!cond()) {
    if (Date.now() > end) throw new Error("기다리다 시간 초과");
    await new Promise((r) => setTimeout(r, 20));
  }
};

/** 가짜 KGBone 창: handler(method, params, client)로 답한다 */
async function fakePage(port, handler, { takeover = false } = {}) {
  const ws = new WebSocket(`ws://127.0.0.1:${port}/`);
  const page = { ws, clients: [], calls: [], fs: new Map(), nextFs: 1, events: [] };
  ws.addEventListener("message", async (ev) => {
    const m = JSON.parse(String(ev.data));
    if (m.type === "busy" || m.type === "replaced") page.events.push(m.type);
    if (m.type === "clients") page.clients = m.clients.map((c) => c.name);
    if (m.type === "fs-result") page.fs.get(m.id)?.(m);
    if (m.type !== "call") return;
    page.calls.push({ client: m.client, method: m.method });
    try {
      ws.send(JSON.stringify({ type: "result", id: m.id, result: await handler(m.method, m.params, m.client) }));
    } catch (e) {
      ws.send(JSON.stringify({ type: "result", id: m.id, error: e.message }));
    }
  });
  await new Promise((r) => ws.addEventListener("open", r));
  ws.send(JSON.stringify({ type: "hello", role: "page", takeover }));
  page.fsCall = (op, args) =>
    new Promise((resolve) => {
      const id = page.nextFs++;
      page.fs.set(id, resolve);
      ws.send(JSON.stringify({ type: "fs", id, op, ...args }));
    });
  cleanups.push(() => ws.close());
  return page;
}

test("mcp_link_allowed_origins", () => {
  assert.equal(allowedOrigin(undefined), true);
  assert.equal(allowedOrigin("http://localhost:1420"), true);
  assert.equal(allowedOrigin("http://127.0.0.1:1421"), true);
  assert.equal(allowedOrigin("http://tauri.localhost"), true);
  assert.equal(allowedOrigin("tauri://localhost"), true);
  assert.equal(allowedOrigin("https://dudwo2199.github.io"), true);
  assert.equal(allowedOrigin("https://evil.example"), false);
  assert.equal(allowedOrigin("http://localhost.evil.example"), false);
  assert.equal(allowedOrigin("https://evil.example", ["https://evil.example"]), true);
});

test("mcp_link_hub_routes_calls_from_several_clients_and_refuses_other_sites", async () => {
  const port = await freePort();
  const a = new Link({ name: "Claude Code", roots: [repo], port, origins: [] });
  const b = new Link({ name: "Cursor", roots: [], port, origins: [] });
  cleanups.push(() => a.close(), () => b.close());
  await a.start();
  await b.start();
  assert.equal(a.role, "hub");
  assert.equal(b.role, "client");

  // 창이 없으면 바로 알려 준다
  await assert.rejects(a.call("echo"), /연결돼 있지 않습니다/);

  const page = await fakePage(port, (method, params, client) => ({ method, client, params }));
  await until(() => a.pageConnected && b.pageConnected && page.clients.length === 2);
  assert.deepEqual(page.clients, ["Claude Code", "Cursor"]);

  const [ra, rb] = await Promise.all([a.call("echo", { n: 1 }), b.call("echo", { n: 2 })]);
  assert.deepEqual(ra, { method: "echo", client: "Claude Code", params: { n: 1 } });
  assert.deepEqual(rb, { method: "echo", client: "Cursor", params: { n: 2 } });

  // 이름은 MCP initialize 뒤에 바뀐다
  b.setName("Codex");
  await until(() => page.clients.includes("Codex"));

  // 창이 읽는 디스크 파일: 허용 폴더 안만
  const ok = await page.fsCall("readText", { path: `disk:${join(repo, "package.json")}` });
  assert.match(ok.result, /"kgbone"/);
  const refused = await page.fsCall("readText", { path: `disk:${join(repo, "..", "밖.txt")}` });
  assert.match(refused.error, /허용한 폴더 밖/);

  // 다른 웹사이트에서 온 연결은 거절(브라우저는 Origin을 숨길 수 없다)
  const status = await new Promise((resolve) => {
    const req = request({
      host: "127.0.0.1",
      port,
      path: "/",
      headers: { Connection: "Upgrade", Upgrade: "websocket", "Sec-WebSocket-Version": "13", "Sec-WebSocket-Key": "dGhlIHNhbXBsZSBub25jZQ==", Origin: "https://evil.example" },
    });
    req.on("response", (res) => resolve(res.statusCode));
    req.on("upgrade", () => resolve(101));
    req.end();
  });
  assert.equal(status, 403);
});

test("mcp_link_auto_connect_waits_and_explicit_connect_takes_over", async () => {
  const port = await freePort();
  const a = new Link({ name: "A", port, origins: [] });
  cleanups.push(() => a.close());
  await a.start();
  const first = await fakePage(port, () => "첫 창");
  await until(() => a.pageConnected);
  // 자동 연결(takeover 없음)은 붙어 있는 창을 밀어내지 않는다
  const auto = await fakePage(port, () => "자동 창");
  await until(() => auto.events.includes("busy"));
  assert.equal(await a.call("x"), "첫 창");
  // 사용자가 [연결]을 누른 창은 가져간다
  const explicit = await fakePage(port, () => "누른 창", { takeover: true });
  await until(() => first.events.includes("replaced"));
  await until(() => explicit.clients.length === 1);
  assert.equal(await a.call("x"), "누른 창");
});

test("mcp_link_client_takes_over_when_hub_closes", async () => {
  const port = await freePort();
  const a = new Link({ name: "A", port, origins: [] });
  const b = new Link({ name: "B", port, origins: [] });
  cleanups.push(() => a.close(), () => b.close());
  await a.start();
  await b.start();
  assert.equal(b.role, "client");
  a.close();
  await until(() => b.role === "hub");
  const page = await fakePage(port, () => "ok");
  await until(() => b.pageConnected);
  assert.equal(await b.call("x"), "ok");
  assert.deepEqual(page.clients, ["B"]);
});

test("mcp_stdio_server_edits_connected_window_document", async () => {
  const port = await freePort();
  // 가짜 창의 문서: 실제 핵심으로 답한다
  const core = await loadCore();
  core.call({ method: "new" });
  const child = spawn(process.execPath, [join(repo, "tools", "mcp", "server.mjs")], {
    cwd: repo,
    stdio: ["pipe", "pipe", "inherit"],
    env: { ...process.env, KGBONE_MCP_PORT: String(port), KGBONE_MCP_LIVE: "1" },
  });
  cleanups.push(() => child.kill());
  const replies = new Map();
  let buf = "";
  child.stdout.on("data", (d) => {
    buf += d;
    let i;
    while ((i = buf.indexOf("\n")) >= 0) {
      const r = JSON.parse(buf.slice(0, i));
      replies.set(r.id, r);
      buf = buf.slice(i + 1);
    }
  });
  let nextId = 1;
  const rpc = async (method, params) => {
    const id = nextId++;
    child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method, params })}\n`);
    await until(() => replies.has(id));
    return replies.get(id);
  };
  const tool = async (name, args = {}) => {
    const r = (await rpc("tools/call", { name, arguments: args })).result;
    const t = r.content.find((c) => c.type === "text")?.text ?? "";
    let data = t;
    try {
      data = JSON.parse(t);
    } catch {
      // 글자 오류
    }
    return { ok: !r.isError, data };
  };

  await rpc("initialize", { protocolVersion: "2025-06-18", clientInfo: { name: "시험 AI", version: "1" } });
  // 중계소가 뜰 때까지
  for (let i = 0; i < 100 && (await tool("app_status")).data.role !== "중계소"; i++) await new Promise((r) => setTimeout(r, 30));
  // 창이 없을 때는 창 없는 문서
  const before = await tool("doc_inspect");
  assert.equal(before.ok, true);
  assert.match(before.data.where, /창 없음/);
  assert.equal((await tool("view_screenshot")).ok, false);

  const uiCalls = [];
  const page = await fakePage(port, (method, params) => {
    switch (method) {
      case "core":
        return core.call(params.request);
      case "doc.info":
        return { filePath: null, imagesDir: "disk:C:/images", images: [], nameStem: "제목 없음" };
      case "ui.run":
        uiCalls.push(params.id);
        return null;
      default:
        throw new Error(`모르는 요청: ${method}`);
    }
  });
  await until(() => page.clients.includes("시험 AI"));
  const status = await tool("app_status");
  assert.equal(status.data.window, true);
  assert.equal(status.data.where, "window");

  const made = await tool("command_run", { id: "bone.create", args: { parent: "root", name: "팔" } });
  assert.ok(made.ok, JSON.stringify(made.data));
  assert.deepEqual(
    core.project().skeleton.bones.map((b) => b.name),
    ["root", "팔"],
  );
  assert.match((await tool("doc_inspect")).data.where, /창\(/);
  // 창이 붙어 있으면 화면 명령도 된다
  assert.ok((await tool("command_run", { id: "view.fit" })).ok);
  assert.deepEqual(uiCalls, ["view.fit"]);
  assert.ok((await tool("command_list", { filter: "view.fit" })).data.commands.some((c) => c.scope === "ui"));
  assert.ok(page.calls.every((c) => c.client === "시험 AI"));
  child.stdin.end();
  await new Promise((r) => child.on("close", r));
});

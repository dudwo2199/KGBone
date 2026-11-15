// KGBone MCP 서버 테스트(Node 내장 테스트 실행기). 실행: npm run test:mcp
// app/public/kgbone.wasm이 있어야 한다(npm run wasm 또는 npm run wasm:fetch).
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { after, before, test } from "node:test";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";
import { readPng } from "./png.mjs";
import { createSession, handleMessage } from "./server.mjs";
import { listTools } from "./tools.mjs";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const sample = join(repo, "samples", "kgbone", "full-4.3.kgbone");
/** 저장 시험용 폴더: 허용한 폴더(저장소) 안이어야 한다 */
let tmp;
let session;
let nextId = 1;

before(async () => {
  process.chdir(repo);
  mkdirSync(join(repo, "test-results"), { recursive: true });
  tmp = mkdtempSync(join(repo, "test-results", "mcp-"));
  session = await createSession();
});
after(() => rmSync(tmp, { recursive: true, force: true }));

/** 도구 호출 → { ok, text(JSON이면 풀어서), content } */
function call(name, args = {}) {
  const res = handleMessage(session, { jsonrpc: "2.0", id: nextId++, method: "tools/call", params: { name, arguments: args } });
  const content = res.result.content;
  const raw = content.find((c) => c.type === "text")?.text ?? "";
  let data = raw;
  try {
    data = JSON.parse(raw);
  } catch {
    // 오류 메시지는 그냥 글자
  }
  return { ok: !res.result.isError, data, content };
}

test("mcp_initialize_and_tool_list", () => {
  const init = handleMessage(session, { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-03-26" } });
  assert.equal(init.result.protocolVersion, "2025-03-26");
  assert.equal(init.result.serverInfo.name, "kgbone");
  // 알림에는 답하지 않는다
  assert.equal(handleMessage(session, { jsonrpc: "2.0", method: "notifications/initialized" }), null);
  const names = listTools().map((t) => t.name);
  for (const n of ["doc_new", "doc_open", "doc_save", "doc_inspect", "command_list", "command_run", "command_batch", "pose", "render", "export_spine"]) {
    assert.ok(names.includes(n), n);
  }
  assert.equal(handleMessage(session, { jsonrpc: "2.0", id: 2, method: "nope" }).error.code, -32601);
});

test("mcp_open_unedited_save_is_byte_identical", () => {
  assert.ok(call("doc_open", { path: sample }).ok);
  const out = join(tmp, "same.kgbone");
  assert.ok(call("doc_save", { path: out }).ok);
  assert.ok(readFileSync(out).equals(readFileSync(sample)));
});

test("mcp_command_run_edits_and_undo_restores", () => {
  call("doc_new");
  const made = call("command_run", { id: "bone.create", args: { parent: "root", name: "팔", x: 10, y: 20 } });
  assert.ok(made.ok, JSON.stringify(made.data));
  assert.equal(made.data.state.dirty, true);
  const bone = call("doc_inspect", { path: "skeleton.bones.팔" });
  assert.equal(bone.data.parent, "root");
  assert.equal(bone.data.x, 10);
  assert.ok(call("command_run", { id: "edit.undo" }).ok);
  assert.deepEqual(call("doc_inspect").data.bones.map((b) => b.name), ["root"]);
});

test("mcp_command_run_rejects_non_core_and_unknown", () => {
  const platform = call("command_run", { id: "file.open" });
  assert.equal(platform.ok, false);
  assert.match(platform.data, /doc_open/);
  assert.equal(call("command_run", { id: "nope.nope" }).ok, false);
  // 핵심이 돌려준 오류도 그대로 전한다
  assert.equal(call("command_run", { id: "bone.delete", args: { name: "없는본" } }).ok, false);
});

test("mcp_command_batch_is_one_undo_step_and_rolls_back_on_failure", () => {
  call("doc_new");
  const ok = call("command_batch", {
    label: "팔 만들기",
    commands: [
      { id: "bone.create", args: { parent: "root", name: "a" } },
      { id: "bone.create", args: { parent: "a", name: "b" } },
    ],
  });
  assert.ok(ok.ok, JSON.stringify(ok.data));
  assert.equal(ok.data.state.undoLabel, "팔 만들기");
  call("command_run", { id: "edit.undo" });
  assert.deepEqual(call("doc_inspect").data.bones.map((b) => b.name), ["root"]);

  const bad = call("command_batch", {
    label: "실패할 묶음",
    commands: [
      { id: "bone.create", args: { parent: "root", name: "c" } },
      { id: "bone.delete", args: { name: "없는본" } },
    ],
  });
  assert.equal(bad.ok, false);
  assert.match(bad.data, /2번째/);
  assert.deepEqual(call("doc_inspect").data.bones.map((b) => b.name), ["root"]);
});

test("mcp_pose_and_render_return_bones_and_png", () => {
  call("doc_open", { path: sample });
  const pose = call("pose", {});
  assert.ok(pose.ok);
  assert.equal(pose.data.bones[0].name, "root");
  assert.equal(typeof pose.data.bones[1].world.x, "number");
  const r = call("render", { scale: 0.5 });
  assert.ok(r.ok, JSON.stringify(r.data));
  const image = r.content.find((c) => c.type === "image");
  assert.equal(image.mimeType, "image/png");
  const { width, height } = readPng(Buffer.from(image.data, "base64"));
  assert.ok(width > 1 && height > 1);
  assert.deepEqual(r.data.warnings, []);
});

test("mcp_export_spine_writes_json_atlas_png", () => {
  call("doc_open", { path: sample });
  const r = call("export_spine", { dir: join(tmp, "out"), name: "char" });
  assert.ok(r.ok, JSON.stringify(r.data));
  const exts = r.data.files.map((f) => f.slice(f.lastIndexOf(".")));
  for (const e of [".json", ".atlas", ".png"]) assert.ok(exts.includes(e), e);
});

test("mcp_paths_outside_allowed_roots_are_refused", () => {
  const outside = join(repo, "..", "밖.kgbone");
  const r = call("doc_save", { path: outside });
  assert.equal(r.ok, false);
  assert.match(r.data, /허용한 폴더 밖/);
});

test("mcp_png_reader_extracts_size_and_alpha", () => {
  // 2×1 RGBA: 불투명 빨강, 반투명 파랑
  const raw = Buffer.from([0, 255, 0, 0, 255, 0, 0, 255, 128]);
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    return Buffer.concat([len, Buffer.from(type, "ascii"), data, Buffer.alloc(4)]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(2, 0);
  ihdr.writeUInt32BE(1, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const png = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
  const info = readPng(png);
  assert.equal(info.width, 2);
  assert.equal(info.height, 1);
  assert.deepEqual([...info.alpha], [255, 128]);
  assert.throws(() => readPng(Buffer.from("not a png, definitely not a png at all")));
});

test("mcp_stdio_server_answers_line_by_line", async () => {
  const child = spawn(process.execPath, [join(repo, "tools", "mcp", "server.mjs")], { cwd: repo, stdio: ["pipe", "pipe", "inherit"] });
  let out = "";
  child.stdout.on("data", (d) => (out += d));
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id: 1, method: "initialize", params: {} })}\n`);
  child.stdin.write("이건 JSON이 아님\n");
  child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id: 2, method: "tools/call", params: { name: "doc_inspect", arguments: {} } })}\n`);
  child.stdin.end();
  await new Promise((resolve) => child.on("close", resolve));
  const lines = out.trim().split("\n").map((l) => JSON.parse(l));
  assert.equal(lines.length, 3);
  assert.equal(lines[0].result.serverInfo.name, "kgbone");
  assert.equal(lines[1].error.code, -32700);
  assert.equal(JSON.parse(lines[2].result.content[0].text).bones[0].name, "root");
});

test("mcp_registered_in_project_mcp_json", () => {
  const cfg = JSON.parse(readFileSync(join(repo, ".mcp.json"), "utf8"));
  assert.equal(cfg.mcpServers.kgbone.command, "node");
  assert.deepEqual(cfg.mcpServers.kgbone.args, ["tools/mcp/server.mjs"]);
});

test("mcp_server_loads_wasm_later_without_reconnect", async () => {
  // wasm이 없는 저장소 사본: tools/mcp + interface/commands.json만
  const fake = join(tmp, "fake-repo");
  mkdirSync(join(fake, "tools", "mcp"), { recursive: true });
  mkdirSync(join(fake, "interface"), { recursive: true });
  for (const f of readdirSync(join(repo, "tools", "mcp"))) {
    if (f.endsWith(".mjs")) copyFileSync(join(repo, "tools", "mcp", f), join(fake, "tools", "mcp", f));
  }
  copyFileSync(join(repo, "interface", "commands.json"), join(fake, "interface", "commands.json"));

  const child = spawn(process.execPath, [join(fake, "tools", "mcp", "server.mjs")], { cwd: fake, stdio: ["pipe", "pipe", "inherit"], env: { ...process.env, KGBONE_WASM: "" } });
  const replies = [];
  let buf = "";
  child.stdout.on("data", (d) => {
    buf += d;
    let i;
    while ((i = buf.indexOf("\n")) >= 0) {
      replies.push(JSON.parse(buf.slice(0, i)));
      buf = buf.slice(i + 1);
    }
  });
  const ask = async (id) => {
    child.stdin.write(`${JSON.stringify({ jsonrpc: "2.0", id, method: "tools/call", params: { name: "doc_inspect", arguments: {} } })}\n`);
    while (!replies.some((r) => r.id === id)) await new Promise((r) => setTimeout(r, 20));
    return replies.find((r) => r.id === id).result;
  };
  try {
    const before = await ask(1);
    assert.equal(before.isError, true);
    assert.match(before.content[0].text, /kgbone\.wasm/);
    mkdirSync(join(fake, "app", "public"), { recursive: true });
    copyFileSync(join(repo, "app", "public", "kgbone.wasm"), join(fake, "app", "public", "kgbone.wasm"));
    const later = await ask(2);
    assert.ok(!later.isError, later.content[0].text);
    assert.equal(JSON.parse(later.content[0].text).bones[0].name, "root");
  } finally {
    child.stdin.end();
    await new Promise((resolve) => child.on("close", resolve));
  }
});

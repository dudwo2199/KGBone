// 창 연결(AI 연결): 열려 있는 KGBone 창(브라우저·설치형 웹 앱·Tauri)과 이 MCP 서버를 잇는다.
// 브라우저 페이지는 서버를 열 수 없으므로, 먼저 뜬 MCP 서버가 127.0.0.1:포트에 중계소(hub)를 열고
// 창(page)과 나중에 뜬 다른 MCP 서버(relay, 예: Cursor·Codex가 띄운 것)가 그 중계소에 붙는다.
// 중계소가 꺼지면 남은 MCP 서버 하나가 이어받는다.
//
// 메시지(한 프레임에 JSON 하나):
//   relay→hub  {type:"hello", role:"relay", name, roots}   {type:"call", id, method, params}
//   hub→relay  {type:"page", connected}   {type:"result", id, result | error}
//   page→hub   {type:"hello", role:"page"}   {type:"result", id, result | error}   {type:"fs", id, op, ...}
//   hub→page   {type:"clients", clients:[{id,name}]}   {type:"call", id, client, method, params}
//              {type:"fs-result", id, result | error}   {type:"fs-changed", dir}   {type:"replaced"}
import { existsSync, mkdirSync, readdirSync, readFileSync, watch, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { dirname, join } from "node:path";
import { isInside } from "./session.mjs";
import { acceptUpgrade } from "./ws.mjs";

export const DEFAULT_PORT = 47821;
/** 기본으로 허용하는 웹 판 주소(GitHub Pages) */
const PAGES_ORIGINS = ["https://dudwo2199.github.io"];
const CALL_TIMEOUT = 120_000;
const DISK = "disk:";

/**
 * 붙어도 되는 곳인지: 브라우저는 Origin을 반드시 보내므로, 아무 웹사이트나 붙는 것을 막는다.
 * Origin이 없으면 브라우저가 아닌 연결(다른 MCP 서버)로 본다.
 */
export function allowedOrigin(origin, extra = []) {
  if (!origin) return true;
  if ([...PAGES_ORIGINS, ...extra].includes(origin)) return true;
  try {
    const u = new URL(origin);
    if (u.protocol === "tauri:") return true;
    const local = u.hostname === "localhost" || u.hostname === "127.0.0.1" || u.hostname === "[::1]" || u.hostname.endsWith(".localhost");
    return local && (u.protocol === "http:" || u.protocol === "https:");
  } catch {
    return false;
  }
}

function extraOrigins() {
  return (process.env.KGBONE_MCP_ORIGINS || "")
    .split(/[;,\s]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

export function configuredPort() {
  const p = Number(process.env.KGBONE_MCP_PORT);
  return Number.isInteger(p) && p > 0 && p < 65536 ? p : DEFAULT_PORT;
}

const errText = (e) => (e instanceof Error ? e.message : String(e));

/** PNG 머리에서 크기만 읽는다 */
function pngSize(bytes) {
  if (bytes.length < 24 || bytes.readUInt32BE(0) !== 0x89504e47) return { width: 0, height: 0 };
  return { width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) };
}

function listPngFiles(dir, prefix = "") {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.isDirectory()) out.push(...listPngFiles(join(dir, e.name), `${prefix}${e.name}/`));
    else if (e.name.toLowerCase().endsWith(".png")) out.push([prefix + e.name.slice(0, -4), join(dir, e.name)]);
  }
  return out.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
}

/** 창이 디스크 파일을 읽고 쓸 때(창의 경로는 'disk:' + 절대 경로). 허용 폴더 밖은 거절 */
class DiskService {
  #watchers = new Map();
  constructor(rootsFn, onChanged) {
    this.rootsFn = rootsFn;
    this.onChanged = onChanged;
  }

  #full(path) {
    if (typeof path !== "string" || !path.startsWith(DISK)) throw new Error(`디스크 경로가 아닙니다: ${path}`);
    const full = path.slice(DISK.length);
    const roots = this.rootsFn();
    if (!isInside(roots, full)) throw new Error(`허용한 폴더 밖입니다: ${full} (허용: ${roots.join(", ")})`);
    return full;
  }

  run(m) {
    switch (m.op) {
      case "readText":
        return readFileSync(this.#full(m.path), "utf8");
      case "readBinary":
        return readFileSync(this.#full(m.path)).toString("base64");
      case "writeText": {
        const f = this.#full(m.path);
        mkdirSync(dirname(f), { recursive: true });
        writeFileSync(f, m.text, "utf8");
        return null;
      }
      case "writeBinary": {
        const f = this.#full(m.path);
        mkdirSync(dirname(f), { recursive: true });
        writeFileSync(f, Buffer.from(m.base64, "base64"));
        return null;
      }
      case "listImages":
        return listPngFiles(this.#full(m.dir)).map(([name, file]) => {
          const bytes = readFileSync(file);
          return { name, ...pngSize(bytes), base64: bytes.toString("base64") };
        });
      case "watch": {
        const full = this.#full(m.dir);
        if (!this.#watchers.has(m.dir) && existsSync(full)) {
          try {
            this.#watchers.set(m.dir, watch(full, { recursive: true }, () => this.onChanged(m.dir)));
          } catch {
            // 지켜보기를 못 하는 곳이면 손으로 다시 불러온다
          }
        }
        return null;
      }
      case "unwatch":
        this.#watchers.get(m.dir)?.close();
        this.#watchers.delete(m.dir);
        return null;
      default:
        throw new Error(`없는 파일 작업입니다: ${m.op}`);
    }
  }

  closeAll() {
    for (const w of this.#watchers.values()) w.close();
    this.#watchers.clear();
  }
}

/**
 * 이 MCP 서버의 창 연결. start()로 중계소를 열거나(포트가 비었으면) 이미 열린 중계소에 붙는다.
 * call(method, params)는 창에 일을 시키고 결과를 기다린다.
 */
export class Link {
  role = "off";
  pageConnected = false;
  #name;
  #roots;
  #port;
  #origins;
  #closed = false;
  #nextId = 1;
  #pending = new Map();
  // 중계소일 때
  #server = null;
  #page = null;
  #relays = new Map();
  #nextRelay = 1;
  #forwarded = new Map();
  #disk = null;
  #conns = new Set();
  // 다른 중계소에 붙었을 때
  #ws = null;

  constructor({ name = "AI", roots = [], port = configuredPort(), origins = extraOrigins() } = {}) {
    this.#name = name;
    this.#roots = roots;
    this.#port = port;
    this.#origins = origins;
  }

  get port() {
    return this.#port;
  }

  async start() {
    while (!this.#closed) {
      if (await this.#tryHub()) return;
      if (await this.#tryClient()) return;
      await new Promise((r) => setTimeout(r, 300 + Math.random() * 400));
    }
  }

  setName(name) {
    if (!name || name === this.#name) return;
    this.#name = name;
    if (this.role === "hub") this.#sendClients();
    else if (this.role === "client") this.#ws?.send(JSON.stringify({ type: "hello", role: "relay", name, roots: this.#roots }));
  }

  status() {
    return {
      port: this.#port,
      role: this.role === "hub" ? "중계소" : this.role === "client" ? "중계소에 붙음" : "꺼짐",
      window: this.pageConnected,
      clients: this.role === "hub" ? this.#clientList().map((c) => c.name) : undefined,
    };
  }

  /** 창에 일 시키기 */
  call(method, params = {}, timeout = CALL_TIMEOUT) {
    if (!this.pageConnected) return Promise.reject(new Error("KGBone 창이 연결돼 있지 않습니다"));
    const id = this.#nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new Error(`창에서 ${Math.round(timeout / 1000)}초 동안 답이 없습니다(창에 대화상자가 열려 있는지 확인하세요)`));
      }, timeout);
      this.#pending.set(id, { resolve, reject, timer });
      if (this.role === "hub") this.#toPage({ type: "call", id: `0:${id}`, client: this.#name, method, params });
      else this.#ws.send(JSON.stringify({ type: "call", id, method, params }));
    });
  }

  #settle(id, m) {
    const p = this.#pending.get(id);
    if (!p) return;
    this.#pending.delete(id);
    clearTimeout(p.timer);
    if (m.error !== undefined) p.reject(new Error(m.error));
    else p.resolve(m.result);
  }

  #rejectAll(message) {
    for (const [id, p] of this.#pending) {
      clearTimeout(p.timer);
      p.reject(new Error(message));
      this.#pending.delete(id);
    }
  }

  close() {
    this.#closed = true;
    this.#rejectAll("연결을 닫았습니다");
    this.#disk?.closeAll();
    for (const c of this.#conns) c.close();
    this.#server?.close();
    this.#ws?.close();
    this.role = "off";
  }

  // ---------- 중계소 ----------

  #tryHub() {
    return new Promise((resolve) => {
      const server = createServer((req, res) => {
        res.writeHead(200, { "content-type": "application/json; charset=utf-8" });
        res.end(JSON.stringify({ app: "kgbone-mcp", window: this.pageConnected }));
      });
      server.on("upgrade", (req, socket, head) => {
        const conn = acceptUpgrade(req, socket, head, (r) => allowedOrigin(r.headers.origin, this.#origins));
        if (conn) this.#onConn(conn, !!req.headers.origin);
      });
      server.once("error", () => resolve(false));
      server.listen(this.#port, "127.0.0.1", () => {
        if (this.#closed) {
          server.close();
          return resolve(false);
        }
        this.#server = server;
        this.role = "hub";
        this.#disk = new DiskService(
          () => [...this.#roots, ...[...this.#relays.values()].flatMap((r) => r.roots)],
          (dir) => this.#toPage({ type: "fs-changed", dir }),
        );
        resolve(true);
      });
    });
  }

  #toPage(m) {
    this.#page?.send(JSON.stringify(m));
  }

  #clientList() {
    return [{ id: "0", name: this.#name }, ...[...this.#relays].filter(([, r]) => r.name).map(([id, r]) => ({ id, name: r.name }))];
  }

  #sendClients() {
    this.#toPage({ type: "clients", clients: this.#clientList() });
  }

  #setPage(connected) {
    this.pageConnected = connected;
    for (const r of this.#relays.values()) r.conn.send(JSON.stringify({ type: "page", connected }));
  }

  #onConn(conn, fromBrowser) {
    this.#conns.add(conn);
    let kind = null;
    let relayId = null;
    conn.on("message", (text) => {
      let m;
      try {
        m = JSON.parse(text);
      } catch {
        return;
      }
      if (m.type === "hello" && m.role === "page") {
        if (this.#page && this.#page !== conn) {
          this.#page.send(JSON.stringify({ type: "replaced" }));
          this.#page.close();
        }
        kind = "page";
        this.#page = conn;
        this.#setPage(true);
        this.#sendClients();
        return;
      }
      if (m.type === "hello" && m.role === "relay" && !fromBrowser) {
        kind = "relay";
        if (!relayId) relayId = String(this.#nextRelay++);
        this.#relays.set(relayId, { conn, name: String(m.name || "AI"), roots: Array.isArray(m.roots) ? m.roots.map(String) : [] });
        conn.send(JSON.stringify({ type: "page", connected: this.pageConnected }));
        this.#sendClients();
        return;
      }
      if (kind === "page") this.#fromPage(m);
      else if (kind === "relay") this.#fromRelay(relayId, m);
    });
    conn.on("close", () => {
      this.#conns.delete(conn);
      if (kind === "page" && this.#page === conn) {
        this.#page = null;
        this.#setPage(false);
        this.#rejectAll("KGBone 창 연결이 끊겼습니다");
        for (const [fid, { relayId: rid, id }] of this.#forwarded) {
          this.#relays.get(rid)?.conn.send(JSON.stringify({ type: "result", id, error: "KGBone 창 연결이 끊겼습니다" }));
          this.#forwarded.delete(fid);
        }
      }
      if (kind === "relay") {
        this.#relays.delete(relayId);
        this.#sendClients();
      }
    });
  }

  #fromPage(m) {
    if (m.type === "result" && typeof m.id === "string") {
      const [rid, rest] = [m.id.slice(0, m.id.indexOf(":")), m.id.slice(m.id.indexOf(":") + 1)];
      if (rid === "0") return this.#settle(Number(rest), m);
      const f = this.#forwarded.get(m.id);
      if (!f) return;
      this.#forwarded.delete(m.id);
      this.#relays.get(f.relayId)?.conn.send(JSON.stringify({ type: "result", id: f.id, result: m.result, error: m.error }));
      return;
    }
    if (m.type === "fs") {
      let reply;
      try {
        reply = { type: "fs-result", id: m.id, result: this.#disk.run(m) };
      } catch (e) {
        reply = { type: "fs-result", id: m.id, error: errText(e) };
      }
      this.#toPage(reply);
    }
  }

  #fromRelay(relayId, m) {
    if (m.type !== "call") return;
    const r = this.#relays.get(relayId);
    if (!this.#page) return r?.conn.send(JSON.stringify({ type: "result", id: m.id, error: "KGBone 창이 연결돼 있지 않습니다" }));
    const fid = `${relayId}:${m.id}`;
    this.#forwarded.set(fid, { relayId, id: m.id });
    this.#toPage({ type: "call", id: fid, client: r?.name ?? "AI", method: m.method, params: m.params });
  }

  // ---------- 다른 중계소에 붙기 ----------

  #tryClient() {
    return new Promise((resolve) => {
      if (typeof WebSocket === "undefined") return resolve(false);
      let ws;
      try {
        ws = new WebSocket(`ws://127.0.0.1:${this.#port}/`);
      } catch {
        return resolve(false);
      }
      let opened = false;
      ws.addEventListener("open", () => {
        opened = true;
        this.#ws = ws;
        this.role = "client";
        ws.send(JSON.stringify({ type: "hello", role: "relay", name: this.#name, roots: this.#roots }));
        resolve(true);
      });
      ws.addEventListener("message", (ev) => {
        let m;
        try {
          m = JSON.parse(String(ev.data));
        } catch {
          return;
        }
        if (m.type === "page") this.pageConnected = !!m.connected;
        else if (m.type === "result") this.#settle(m.id, m);
      });
      ws.addEventListener("close", () => {
        if (!opened) return resolve(false);
        this.#ws = null;
        this.role = "off";
        this.pageConnected = false;
        this.#rejectAll("중계소 연결이 끊겼습니다");
        // 중계소가 꺼졌으면 이어받는다
        if (!this.#closed) void this.start();
      });
      ws.addEventListener("error", () => {});
    });
  }
}

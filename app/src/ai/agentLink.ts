// MCP 연결: 이 창을 이 PC의 KGBone MCP 서버 중계소(ws://127.0.0.1:포트)에 잇는다.
// 브라우저 페이지는 서버를 열 수 없으므로 창이 중계소에 붙으러 간다. 중계소는 AI 프로그램(Claude Code, Cursor, Codex …)이
// 띄운 MCP 서버(tools/mcp/server.mjs)가 연다. 메시지 형식은 tools/mcp/link.mjs 머리말과 같다.
import { Store } from '../state/store';

export const DEFAULT_AGENT_PORT = 47821;

/** busy: 다른 창이 연결돼 있어 기다림(그 창이 끊기면 이어받는다) */
export type AgentLinkStatus = 'off' | 'connecting' | 'connected' | 'waiting' | 'busy';

export interface AgentActivity {
  client: string;
  what: string;
  at: number;
  ok: boolean;
}

export interface AgentLinkState {
  status: AgentLinkStatus;
  port: number;
  /** 창을 열 때 자동으로 연결 */
  auto: boolean;
  /** 붙어 있는 AI 프로그램 이름 */
  clients: string[];
  last: AgentActivity | null;
  error: string | null;
}

/** 창에 온 일 하나를 처리한다(agentHandler.ts) */
export type AgentHandler = (method: string, params: Record<string, unknown>, client: string) => Promise<unknown>;

/** 창 쪽 WebSocket(테스트에서 바꿔 끼운다) */
export interface SocketLike {
  readonly readyState: number;
  send(data: string): void;
  close(): void;
  onopen: ((ev: unknown) => void) | null;
  onclose: ((ev: unknown) => void) | null;
  onerror: ((ev: unknown) => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
}

interface KeyValue {
  getItem(k: string): string | null;
  setItem(k: string, v: string): void;
}

const KEY = 'kgbone.agentLink';
const RETRY_MS = 3000;
const OPEN = 1;

function safeStorage(): KeyValue | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage;
  } catch {
    return null;
  }
}

/** 요청을 사람이 읽을 이름으로(대화상자의 최근 작업 표시) */
function describe(method: string, params: Record<string, unknown>): string {
  if (method === 'core') {
    const r = (params.request ?? {}) as Record<string, unknown>;
    return r.method === 'execute' ? String(r.id) : String(r.method);
  }
  if (method === 'ui.run') return String(params.id);
  return method;
}

/** 조용히 오가는 읽기 요청(최근 작업에 남기지 않는다) */
const QUIET = new Set(['project', 'state', 'history.begin', 'history.end', 'doc.info']);

export class AgentLink extends Store<AgentLinkState> {
  private ws: SocketLike | null = null;
  private want = false;
  /** 다음 인사에서 붙어 있는 다른 창의 연결을 가져올지(사용자가 [연결]을 눌렀을 때만) */
  private takeover = false;
  private busy = false;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private handler: AgentHandler | null = null;
  private queue: Promise<void> = Promise.resolve();
  private nextFs = 1;
  private fsPending = new Map<number, { resolve(v: unknown): void; reject(e: Error): void }>();
  private watchers = new Map<string, Set<() => void>>();

  constructor(
    private makeSocket: (url: string) => SocketLike = (url) => new WebSocket(url) as unknown as SocketLike,
    private storage: KeyValue | null = safeStorage(),
  ) {
    super({ status: 'off', port: DEFAULT_AGENT_PORT, auto: false, clients: [], last: null, error: null });
    try {
      const saved = JSON.parse(this.storage?.getItem(KEY) ?? 'null') as { port?: number; auto?: boolean } | null;
      if (saved) this.set({ port: validPort(saved.port) ?? DEFAULT_AGENT_PORT, auto: !!saved.auto });
    } catch {
      // 저장값이 없거나 깨짐
    }
  }

  setHandler(h: AgentHandler): void {
    this.handler = h;
  }

  /** 창을 열 때: 자동 연결이 켜져 있으면 잇는다 */
  autoStart(): void {
    if (this.get().auto) this.start(undefined, false);
  }

  /** 사용자가 [연결]을 누름: 다른 창이 연결돼 있으면 그 연결을 가져온다 */
  connect(port?: number): void {
    this.start(port, true);
  }

  private start(port: number | undefined, takeover: boolean): void {
    const p = validPort(port) ?? this.get().port;
    this.set({ port: p, error: null });
    this.save();
    this.want = true;
    this.takeover = takeover;
    this.busy = false;
    this.closeSocket();
    this.open();
  }

  disconnect(): void {
    this.want = false;
    this.closeSocket();
    this.set({ status: 'off', clients: [] });
  }

  setAuto(auto: boolean): void {
    this.set({ auto });
    this.save();
  }

  /** 중계소에 붙어 있는가(디스크 파일을 읽고 쓸 수 있는가) */
  isConnected(): boolean {
    return this.get().status === 'connected';
  }

  /** 중계소를 거쳐 디스크 파일 작업(경로는 'disk:' + 절대 경로) */
  fs<T = unknown>(op: string, args: Record<string, unknown>): Promise<T> {
    const ws = this.ws;
    if (!ws || ws.readyState !== OPEN || this.get().status !== 'connected') {
      return Promise.reject(new Error('MCP 연결이 끊겨 있어 디스크 파일을 읽고 쓸 수 없습니다(도구 → MCP 연결)'));
    }
    const id = this.nextFs++;
    return new Promise<T>((resolve, reject) => {
      this.fsPending.set(id, { resolve: resolve as (v: unknown) => void, reject });
      ws.send(JSON.stringify({ type: 'fs', id, op, ...args }));
    });
  }

  /** 디스크 폴더 지켜보기 */
  watch(dir: string, cb: () => void): () => void {
    let set = this.watchers.get(dir);
    if (!set) {
      set = new Set();
      this.watchers.set(dir, set);
      void this.fs('watch', { dir }).catch(() => undefined);
    }
    set.add(cb);
    return () => {
      const s = this.watchers.get(dir);
      if (!s) return;
      s.delete(cb);
      if (!s.size) {
        this.watchers.delete(dir);
        void this.fs('unwatch', { dir }).catch(() => undefined);
      }
    };
  }

  private save(): void {
    try {
      this.storage?.setItem(KEY, JSON.stringify({ port: this.get().port, auto: this.get().auto }));
    } catch {
      // 저장 못 해도 동작
    }
  }

  private closeSocket(): void {
    if (this.retry) clearTimeout(this.retry);
    this.retry = null;
    const ws = this.ws;
    this.ws = null;
    if (ws) {
      ws.onopen = ws.onclose = ws.onerror = null;
      ws.onmessage = null;
      try {
        ws.close();
      } catch {
        // 이미 닫힘
      }
    }
    this.failFs('MCP 연결이 끊겼습니다');
  }

  private failFs(message: string): void {
    for (const p of this.fsPending.values()) p.reject(new Error(message));
    this.fsPending.clear();
  }

  private open(): void {
    const cur = this.get().status;
    this.set({ status: cur === 'waiting' || cur === 'busy' ? cur : 'connecting' });
    let ws: SocketLike;
    try {
      ws = this.makeSocket(`ws://127.0.0.1:${this.get().port}/`);
    } catch (e) {
      this.set({ status: 'off', error: e instanceof Error ? e.message : String(e) });
      this.want = false;
      return;
    }
    this.ws = ws;
    ws.onopen = () => {
      ws.send(JSON.stringify({ type: 'hello', role: 'page', app: 'kgbone', takeover: this.takeover }));
      // 가져오기는 한 번만: 끊겼다 다시 붙을 때는 다른 창을 밀어내지 않는다
      this.takeover = false;
      this.busy = false;
      this.set({ status: 'connected', error: null });
      // 다시 붙었으면 지켜보던 폴더를 다시 건다
      for (const dir of this.watchers.keys()) void this.fs('watch', { dir }).catch(() => undefined);
    };
    ws.onmessage = (ev) => this.onMessage(String(ev.data));
    ws.onerror = () => undefined;
    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      this.failFs('MCP 연결이 끊겼습니다');
      if (!this.want) return this.set({ status: 'off', clients: [] });
      // AI 프로그램(MCP 서버)이 아직 없거나 꺼졌다(또는 다른 창이 연결돼 있다): 기다렸다가 다시 붙는다
      this.set({ status: this.busy ? 'busy' : 'waiting', clients: [] });
      this.retry = setTimeout(() => {
        this.retry = null;
        if (this.want) this.open();
      }, RETRY_MS);
    };
  }

  private onMessage(text: string): void {
    let m: Record<string, unknown>;
    try {
      m = JSON.parse(text) as Record<string, unknown>;
    } catch {
      return;
    }
    switch (m.type) {
      case 'clients':
        this.set({ clients: ((m.clients as { name: string }[]) ?? []).map((c) => c.name) });
        return;
      case 'busy':
        this.busy = true;
        this.set({ status: 'busy', clients: [] });
        return;
      case 'replaced':
        this.want = false;
        this.closeSocket();
        this.set({ status: 'off', clients: [], error: '다른 KGBone 창이 MCP 연결을 가져가서 이 창의 연결을 끊었습니다' });
        return;
      case 'fs-result': {
        const p = this.fsPending.get(m.id as number);
        if (!p) return;
        this.fsPending.delete(m.id as number);
        if (m.error !== undefined) p.reject(new Error(String(m.error)));
        else p.resolve(m.result);
        return;
      }
      case 'fs-changed':
        for (const cb of this.watchers.get(String(m.dir)) ?? []) cb();
        return;
      case 'call':
        this.queue = this.queue.then(() => this.handleCall(m));
        return;
    }
  }

  /** 받은 차례대로 하나씩 처리한다 */
  private async handleCall(m: Record<string, unknown>): Promise<void> {
    const ws = this.ws;
    const method = String(m.method);
    const params = (m.params ?? {}) as Record<string, unknown>;
    const client = String(m.client ?? 'AI');
    let reply: Record<string, unknown>;
    try {
      if (!this.handler) throw new Error('창이 아직 준비되지 않았습니다');
      const result = await this.handler(method, params, client);
      reply = { type: 'result', id: m.id, result: result ?? null };
    } catch (e) {
      reply = { type: 'result', id: m.id, error: e instanceof Error ? e.message : String(e) };
    }
    const what = describe(method, params);
    if (!QUIET.has(what)) this.set({ last: { client, what, at: Date.now(), ok: reply.error === undefined } });
    if (ws && ws === this.ws && ws.readyState === OPEN) ws.send(JSON.stringify(reply));
  }
}

function validPort(p: unknown): number | null {
  const n = Number(p);
  return Number.isInteger(n) && n > 0 && n < 65536 ? n : null;
}

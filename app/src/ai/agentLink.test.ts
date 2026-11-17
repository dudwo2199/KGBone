// MCP 연결(창 쪽) 단위 테스트: 가짜 WebSocket으로 인사·명령 처리·차례·파일 작업·끊김 후 기다리기를 본다.
import { describe, expect, it, vi } from 'vitest';
import { DiskPlatform } from '../platform/diskPlatform';
import type { Platform } from '../platform/platform';
import { AgentLink, type SocketLike } from './agentLink';

class FakeSocket implements SocketLike {
  readyState = 0;
  sent: Record<string, unknown>[] = [];
  onopen: ((ev: unknown) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  constructor(readonly url: string) {}
  send(data: string) {
    this.sent.push(JSON.parse(data));
  }
  close() {
    this.readyState = 3;
  }
  // 시험용
  open() {
    this.readyState = 1;
    this.onopen?.({});
  }
  drop() {
    this.readyState = 3;
    this.onclose?.({});
  }
  receive(m: unknown) {
    this.onmessage?.({ data: JSON.stringify(m) });
  }
}

function setup() {
  const sockets: FakeSocket[] = [];
  const store = new Map<string, string>();
  const link = new AgentLink(
    (url) => {
      const s = new FakeSocket(url);
      sockets.push(s);
      return s;
    },
    { getItem: (k) => store.get(k) ?? null, setItem: (k, v) => void store.set(k, v) },
  );
  return { link, sockets, store };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

describe('MCP 연결', () => {
  it('f_n01_connect_says_hello_and_lists_clients', () => {
    const { link, sockets } = setup();
    link.connect(50000);
    expect(sockets[0].url).toBe('ws://127.0.0.1:50000/');
    expect(link.get().status).toBe('connecting');
    sockets[0].open();
    expect(sockets[0].sent[0]).toEqual({ type: 'hello', role: 'page', app: 'kgbone', takeover: true });
    expect(link.get().status).toBe('connected');
    sockets[0].receive({ type: 'clients', clients: [{ id: '0', name: 'Claude Code' }, { id: '1', name: 'Cursor' }] });
    expect(link.get().clients).toEqual(['Claude Code', 'Cursor']);
  });

  it('f_n01_calls_run_in_order_and_reply_with_result_or_error', async () => {
    const { link, sockets } = setup();
    const order: string[] = [];
    link.setHandler(async (method, params) => {
      order.push(`${method}:start`);
      if (method === 'slow') await new Promise((r) => setTimeout(r, 20));
      order.push(`${method}:end`);
      if (method === 'bad') throw new Error('실패함');
      return { echo: params };
    });
    link.connect();
    sockets[0].open();
    sockets[0].receive({ type: 'call', id: '0:1', client: 'Codex', method: 'slow', params: { a: 1 } });
    sockets[0].receive({ type: 'call', id: '1:1', client: 'Cursor', method: 'bad', params: {} });
    await vi.waitFor(() => expect(sockets[0].sent.length).toBe(3));
    // 앞의 일이 끝난 뒤에 다음 일을 시작한다
    expect(order).toEqual(['slow:start', 'slow:end', 'bad:start', 'bad:end']);
    expect(sockets[0].sent[1]).toEqual({ type: 'result', id: '0:1', result: { echo: { a: 1 } } });
    expect(sockets[0].sent[2]).toEqual({ type: 'result', id: '1:1', error: '실패함' });
    expect(link.get().last).toMatchObject({ client: 'Cursor', what: 'bad', ok: false });
  });

  it('f_n01_waits_and_retries_when_no_ai_program_is_running', () => {
    vi.useFakeTimers();
    try {
      const { link, sockets } = setup();
      link.connect();
      sockets[0].drop();
      expect(link.get().status).toBe('waiting');
      vi.advanceTimersByTime(3100);
      expect(sockets.length).toBe(2);
      sockets[1].open();
      expect(link.get().status).toBe('connected');
      link.disconnect();
      expect(link.get().status).toBe('off');
      vi.advanceTimersByTime(10000);
      expect(sockets.length).toBe(2);
    } finally {
      vi.useRealTimers();
    }
  });

  it('f_n01_auto_connect_does_not_take_over_and_waits_when_busy', () => {
    vi.useFakeTimers();
    try {
      const { link, sockets, store } = setup();
      link.setAuto(true);
      const auto = new AgentLink(
        (url) => {
          const s = new FakeSocket(url);
          sockets.push(s);
          return s;
        },
        { getItem: (k) => store.get(k) ?? null, setItem: () => undefined },
      );
      auto.autoStart();
      const s0 = sockets.at(-1)!;
      s0.open();
      expect(s0.sent[0]).toMatchObject({ type: 'hello', takeover: false });
      s0.receive({ type: 'busy' });
      s0.drop();
      expect(auto.get().status).toBe('busy');
      vi.advanceTimersByTime(3100);
      const s1 = sockets.at(-1)!;
      expect(s1).not.toBe(s0);
      s1.open();
      expect(s1.sent[0]).toMatchObject({ takeover: false });
      expect(auto.get().status).toBe('connected');
    } finally {
      vi.useRealTimers();
    }
  });

  it('f_n01_replaced_by_other_window_turns_off', () => {
    const { link, sockets } = setup();
    link.connect();
    sockets[0].open();
    sockets[0].receive({ type: 'replaced' });
    expect(link.get().status).toBe('off');
    expect(link.get().error).toMatch(/다른 KGBone 창/);
  });

  it('f_n01_port_and_auto_are_remembered', () => {
    const { link, store } = setup();
    link.connect(51000);
    link.setAuto(true);
    const again = new AgentLink(() => new FakeSocket('x'), { getItem: (k) => store.get(k) ?? null, setItem: () => undefined });
    expect(again.get().port).toBe(51000);
    expect(again.get().auto).toBe(true);
  });

  it('f_n01_disk_paths_go_through_link_other_paths_to_base', async () => {
    const { link, sockets } = setup();
    link.connect();
    sockets[0].open();
    const base = { kind: 'web', readText: vi.fn(async () => '메모리') } as unknown as Platform;
    const p = new DiskPlatform(base, link);
    expect(await p.readText('/samples/a.kgbone')).toBe('메모리');
    const pending = p.readText('disk:D:\\work\\a.kgbone');
    await tick();
    const req = sockets[0].sent.at(-1)!;
    expect(req).toMatchObject({ type: 'fs', op: 'readText', path: 'disk:D:\\work\\a.kgbone' });
    sockets[0].receive({ type: 'fs-result', id: req.id, result: '디스크' });
    expect(await pending).toBe('디스크');
    // 끊기면 디스크 작업은 이유와 함께 실패한다
    link.disconnect();
    await expect(p.readText('disk:D:\\work\\a.kgbone')).rejects.toThrow(/MCP 연결/);
  });
});

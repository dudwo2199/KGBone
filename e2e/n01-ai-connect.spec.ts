// N01: 도구 → MCP 연결. 실제 MCP 서버(tools/mcp/server.mjs)를 표준 입출력으로 띄우고, 창을 그 중계소에 이은 뒤
// AI 쪽 도구 호출이 열린 창의 문서를 고치는지(화면에 보임, Ctrl+Z로 되돌림), 디스크 파일 열기·저장이 되는지 본다.
import { expect, test } from '@playwright/test';
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { join } from 'node:path';
import { focusBody, menu, openApp, treeItem } from './helpers';

const repo = process.cwd();

const freePort = () =>
  new Promise<number>((resolve) => {
    const s = createServer();
    s.listen(0, '127.0.0.1', () => {
      const { port } = s.address() as { port: number };
      s.close(() => resolve(port));
    });
  });

/** MCP 클라이언트(시험용 AI 프로그램 흉내) */
function mcpClient(child: ChildProcessWithoutNullStreams) {
  const replies = new Map<number, { result?: { content: { type: string; text?: string; data?: string }[]; isError?: boolean } }>();
  let buf = '';
  let nextId = 1;
  child.stdout.on('data', (d) => {
    buf += d;
    let i;
    while ((i = buf.indexOf('\n')) >= 0) {
      const r = JSON.parse(buf.slice(0, i));
      replies.set(r.id, r);
      buf = buf.slice(i + 1);
    }
  });
  const rpc = async (method: string, params: unknown) => {
    const id = nextId++;
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
    await expect.poll(() => replies.has(id), { timeout: 20_000 }).toBe(true);
    return replies.get(id)!;
  };
  const tool = async (name: string, args: Record<string, unknown> = {}) => {
    const r = (await rpc('tools/call', { name, arguments: args })).result!;
    const t = r.content.find((c) => c.type === 'text')?.text ?? '';
    let data: any = t;
    try {
      data = JSON.parse(t);
    } catch {
      // 글자
    }
    return { ok: !r.isError, data, content: r.content };
  };
  return { rpc, tool };
}

test('e_n01_ai_connect_edits_open_window', async ({ page }) => {
  const port = await freePort();
  const out = join(repo, 'test-results', 'n01');
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const child = spawn(process.execPath, [join(repo, 'tools', 'mcp', 'server.mjs')], {
    cwd: repo,
    env: { ...process.env, KGBONE_MCP_PORT: String(port), KGBONE_MCP_LIVE: '1' },
  });
  try {
    const ai = mcpClient(child);
    await ai.rpc('initialize', { protocolVersion: '2025-06-18', clientInfo: { name: '시험 AI', version: '1' } });
    await expect.poll(async () => (await ai.tool('app_status')).data.role).toBe('중계소');

    await openApp(page);
    await menu(page, '도구', 'MCP 연결');
    const dialog = page.getByRole('dialog', { name: 'MCP 연결' });
    await dialog.getByLabel('포트').fill(String(port));
    await dialog.getByRole('button', { name: '연결', exact: true }).click();
    await expect(dialog.getByTestId('ai-status')).toContainText('연결됨');
    await expect(dialog.getByTestId('ai-clients')).toHaveText('시험 AI');
    await dialog.getByRole('button', { name: '닫기' }).click();
    await expect(page.getByTestId('status-ai')).toContainText('MCP 연결됨');

    // AI가 본을 만들면 창의 트리에 바로 보인다
    await expect.poll(async () => (await ai.tool('app_status')).data.window).toBe(true);
    const made = await ai.tool('command_run', { id: 'bone.create', args: { parent: 'root', name: '팔' } });
    expect(made.ok, JSON.stringify(made.data)).toBe(true);
    await expect(treeItem(page, 'bone', '팔')).toBeVisible();
    // 창에서 Ctrl+Z로 되돌린다(같은 되돌리기 기록)
    await focusBody(page);
    await page.keyboard.press('Control+z');
    await expect(treeItem(page, 'bone', '팔')).toHaveCount(0);
    expect((await ai.tool('doc_inspect')).data.bones.map((b: { name: string }) => b.name)).toEqual(['root']);

    // 화면 명령도 쓸 수 있다
    expect((await ai.tool('command_run', { id: 'view.fit' })).ok).toBe(true);
    // 대화상자를 여는 화면 명령은 기다리지 않고 닫은 뒤 실패로 알린다
    const dlg = await ai.tool('command_run', { id: 'help.shortcuts' });
    expect(dlg.ok).toBe(false);
    expect(dlg.data).toMatch(/대화상자/);
    await expect(page.getByRole('dialog')).toHaveCount(0);

    // 디스크의 시험 파일 열기(이미지 폴더도 중계소를 거쳐 읽는다)
    const opened = await ai.tool('doc_open', { path: join('samples', 'kgbone', 'full-4.3.kgbone') });
    expect(opened.ok, JSON.stringify(opened.data)).toBe(true);
    await expect(treeItem(page, 'bone', '몸통')).toBeVisible();
    await expect.poll(() => page.evaluate(() => (window as any).__kgbone.images.get().entries.length)).toBeGreaterThan(0);

    // 디스크에 저장: 무편집 저장은 바이트 동일
    const saved = await ai.tool('doc_save', { path: join(out, 'same.kgbone') });
    expect(saved.ok, JSON.stringify(saved.data)).toBe(true);
    expect(existsSync(join(out, 'same.kgbone'))).toBe(true);
    expect(readFileSync(join(out, 'same.kgbone')).equals(readFileSync(join(repo, 'samples', 'kgbone', 'full-4.3.kgbone')))).toBe(true);
    await expect(page).toHaveTitle(/same\.kgbone/);

    // 창 화면 찍기
    const shot = await ai.tool('view_screenshot');
    expect(shot.ok, JSON.stringify(shot.data)).toBe(true);
    expect(shot.content.find((c) => c.type === 'image')?.data?.length ?? 0).toBeGreaterThan(1000);

    // 허용 폴더 밖은 거절
    const refused = await ai.tool('doc_save', { path: join(repo, '..', '밖.kgbone') });
    expect(refused.ok).toBe(false);
  } finally {
    child.stdin.end();
    child.kill();
  }
});

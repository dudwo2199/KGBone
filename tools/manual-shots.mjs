// 사용 설명서(manual/) 스크린샷을 다시 찍는다. 개발 서버(npm run dev)를 띄워 둔 채 실행한다.
// 사용: node tools/manual-shots.mjs [주소, 기본 http://localhost:1420] [찍을 이름 일부]
// 메모리 파일 판(?mem=1)으로 열어 시험 이미지(samples/images)로 예제 인형을 만들며 찍는다. 설치된 Edge를 쓴다.
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { createServer } from 'node:net';
import { Link } from './mcp/link.mjs';

const repo = join(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(repo, 'manual', 'images');
mkdirSync(out, { recursive: true });
const base = process.argv[2] ?? 'http://localhost:1420';
const only = process.argv[3] ?? '';

const browser = await chromium.launch({ channel: 'msedge' });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, locale: 'ko-KR', deviceScaleFactor: 1 });
await page.goto(`${base}/?mem=1`);
await page.locator('.app[data-ready="true"]').waitFor({ timeout: 30_000 });

// ---------- 도우미 ----------
const run = (id, args = {}) =>
  page.evaluate(([id, args]) => {
    const r = window.__kgbone.doc.run(id, args);
    if (!r.ok) throw new Error(`${id}: ${JSON.stringify(r)}`);
    return r.value ?? null;
  }, [id, args]);
const cmd = async (id, args = {}) => {
  await page.evaluate(([id, args]) => window.__kgbone.run(id, args), [id, args]);
  await settle(150);
};
const select = (items) => page.evaluate((items) => window.__kgbone.ui.select(items), items);
const ui = (patch) => page.evaluate((patch) => window.__kgbone.ui.set(patch), patch);
const blur = () => page.evaluate(() => document.activeElement?.blur());
const settle = (ms = 300) => page.waitForTimeout(ms);
const tree = (kind, name) => page.locator(`.tree-body [role="treeitem"][data-kind="${kind}"][data-name="${name}"]`).first();
const viewport = page.getByTestId('viewport');

async function shot(name, target, clip) {
  await settle();
  if (only && !name.includes(only)) return;
  const path = join(out, `${name}.png`);
  if (target) await target.screenshot({ path });
  else await page.screenshot({ path, ...(clip ? { clip } : {}) });
  console.log('찍음', name);
}

/** 화면 위에 번호 표시를 붙인다 */
async function callouts(list) {
  await page.evaluate((list) => {
    for (const [n, sel, dx, dy] of list) {
      const el = document.querySelector(sel);
      if (!el) throw new Error('없음: ' + sel);
      const r = el.getBoundingClientRect();
      const d = document.createElement('div');
      d.className = 'manual-callout';
      d.textContent = String(n);
      Object.assign(d.style, {
        position: 'fixed', left: `${r.left + dx}px`, top: `${r.top + dy}px`, width: '28px', height: '28px', borderRadius: '14px',
        background: '#ff8a00', color: '#000', font: 'bold 16px sans-serif', display: 'flex', alignItems: 'center', justifyContent: 'center',
        zIndex: 99999, boxShadow: '0 0 0 2px #000', pointerEvents: 'none',
      });
      document.body.appendChild(d);
    }
  }, list);
}
const clearCallouts = () => page.evaluate(() => document.querySelectorAll('.manual-callout').forEach((e) => e.remove()));

async function openMenu(top, ...items) {
  await page.locator('.menubar-button', { hasText: top }).dispatchEvent('mousedown');
  for (const label of items) await page.locator('.menu-dropdown .menu-item', { hasText: label }).last().hover();
  await settle();
}
async function closeMenu() {
  await page.keyboard.press('Escape');
  await page.mouse.click(700, 450, { button: 'left' }).catch(() => {});
}

// ---------- 1. 화면 구성 ----------
await callouts([
  [1, '.menubar', 415, 0],
  [2, '.mode-switch', 145, 2],
  [3, '.viewport', 520, 300],
  [4, '.toolbar', 760, 3],
  [5, '.tree-panel', 60, 2],
  [6, '.props-panel', 60, 2],
  [7, '.statusbar', 220, -6],
]);
await shot('01-layout');
await clearCallouts();

// ---------- 2. 첫 캐릭터 ----------
await openMenu('파일');
await shot('02-file-menu', null, { x: 0, y: 0, width: 520, height: 420 });
await closeMenu();

// 이미지 폴더 펼치기
await page.locator('.tree-body [role="treeitem"]', { hasText: '이미지' }).locator('.tree-twisty').click();
await shot('02-images-tree', page.locator('.tree-panel'));

// 본: 몸통(위를 향함) → 머리 → 모자, 팔·다리
const bone = (a) => run('bone.create', a);
await bone({ parent: 'root', name: '몸통', x: 0, y: 70, rotation: 90, length: 90 });
await bone({ parent: '몸통', name: '머리', x: 92, y: 0, rotation: 0, length: 60 });
await bone({ parent: '머리', name: '모자', x: 62, y: 0, rotation: 0, length: 20 });
await bone({ parent: '몸통', name: '왼팔', x: 82, y: 26, rotation: 165, length: 60 });
await bone({ parent: '몸통', name: '오른팔', x: 82, y: -26, rotation: -165, length: 60 });
await bone({ parent: '몸통', name: '왼다리', x: 4, y: 12, rotation: 180, length: 68 });
await bone({ parent: '몸통', name: '오른다리', x: 4, y: -12, rotation: 180, length: 68 });
await cmd('tool.create');
await select([{ kind: 'bone', name: '왼팔' }]);
await cmd('view.fit');
await shot('02-bones');
await shot('02-bone-props', page.locator('.props-panel'));

// 슬롯 + 이미지 첨부물(그리기 순서: 뒤 → 앞)
const part = async (slot, boneName, image, props, name = image) => {
  await run('slot.create', { bone: boneName, name: slot });
  await run('attachment.create', { slot, name, type: 'region', props: name === image ? props : { path: image, ...props } });
};
await part('왼팔', '왼팔', 'arm', { x: 30, width: 64, height: 20 });
await part('왼다리', '왼다리', 'leg', { x: 34, width: 72, height: 24 });
await part('오른다리', '오른다리', 'leg', { x: 34, width: 72, height: 24 });
await part('몸통', '몸통', 'body', { x: 46, rotation: -90, width: 48, height: 96 });
await part('머리', '머리', 'head', { x: 30, rotation: -90, width: 64, height: 64 });
// 모자는 스킨으로 바꿔 끼우므로 첨부물 이름을 '모자'로 두고 이미지 경로만 지정한다
await part('모자', '모자', 'hat-red', { x: 6, rotation: -90, width: 56, height: 24 }, '모자');
await part('오른팔', '오른팔', 'arm', { x: 30, width: 64, height: 20 });
const frame = async () => {
  await cmd('view.fit');
  await cmd('view.zoomOut');
  await cmd('view.zoomOut');
};
await cmd('tool.translate');
await select([]);
await frame();
await shot('02-doll');

// 트리 오른쪽 클릭 메뉴
await tree('bone', '몸통').click({ button: 'right' });
await settle();
await shot('02-tree-menu', null, { x: 1000, y: 60, width: 440, height: 420 });
await page.keyboard.press('Escape');
await page.mouse.click(300, 300);
await select([]);

// 이미지 고르기 대화상자(슬롯을 고르고 만들기 › 첨부물 › 이미지)
await select([{ kind: 'slot', name: '오른팔' }]);
await page.locator('.menubar-button', { hasText: '만들기' }).dispatchEvent('mousedown');
await page.locator('.menu-dropdown .menu-item', { hasText: '첨부물' }).last().hover();
await page.locator('.menu-dropdown .menu-item', { hasText: /^이미지/ }).last().click();
const picker = page.getByRole('dialog', { name: '이미지 고르기' });
await picker.getByRole('option', { name: 'arm', exact: true }).click();
await shot('02-image-picker', picker);
await picker.getByRole('button', { name: '취소' }).click();

// 슬롯·첨부물 속성
await select([{ kind: 'slot', name: '몸통' }]);
await shot('02-slot-props', page.locator('.props-panel'));
await select([{ kind: 'attachment', name: 'body', slot: '몸통', skin: 'default' }]);
await shot('02-attachment-props', page.locator('.props-panel'));

// 그리기 순서 폴더
await page.locator('.tree-body [role="treeitem"]', { hasText: '그리기 순서' }).locator('.tree-twisty').click();
await page.locator('.tree-body [role="treeitem"]', { hasText: '이미지' }).locator('.tree-twisty').click();
await select([]);
await page.locator('.tree-body [role="treeitem"][data-kind="bone"][data-name="root"]').locator('.tree-twisty').click();
await shot('02-draw-order', page.locator('.tree-panel'));
await page.locator('.tree-body [role="treeitem"][data-kind="bone"][data-name="root"]').locator('.tree-twisty').click();
console.log('그리기 순서', await page.evaluate(() => window.__kgbone.doc.get().project.skeleton.slots.map((s) => s.name).join(',')));

// ---------- 3. 애니메이션 ----------
await cmd('animation.create');
const animName = await page.evaluate(() => window.__kgbone.doc.get().project.editor.activeAnimation);
await run('item.rename', { kind: 'animation', name: animName, newName: '손 흔들기' });
await run('animation.setActive', { name: '손 흔들기' });
await cmd('mode.toggle');
const key = async (t, boneName, v) => {
  await run('time.set', { time: t });
  await run('bone.setTransform', { name: boneName, ...v });
};
await key(0, '오른팔', { rotation: -165 });
await key(0.4, '오른팔', { rotation: -30 });
await key(0.6, '오른팔', { rotation: -60 });
await key(0.8, '오른팔', { rotation: -30 });
await key(1.2, '오른팔', { rotation: -165 });
await key(0, '머리', { rotation: 0 });
await key(0.6, '머리', { rotation: 10 });
await key(1.2, '머리', { rotation: 0 });
await run('time.set', { time: 0.4 });
await select([{ kind: 'bone', name: '오른팔' }]);
await cmd('tool.rotate');
await frame();
await shot('03-animate');
await shot('03-playback', page.locator('[data-panel="bottom"]'));

// 그래프: 오른팔 회전 키를 고르고 그래프 탭
await page.locator('[data-row$=":rotate"] [data-testid="dope-key"]').nth(1).click();
await cmd('key.setCurve', { curve: 'bezier' });
await cmd('view.showGraph');
await shot('03-graph', page.locator('[data-panel="bottom"]'));
await cmd('view.showDopesheet');

// 뼈대 속성(FPS, 이미지 폴더, 오니언 스킨 설정)
await page.locator('.tree-body [role="treeitem"]', { hasText: '뼈대' }).first().locator('.tree-label').click();
await shot('03-skeleton-props', page.locator('.props-panel'));

// 오니언 스킨
await cmd('view.toggleOnion');
await run('time.set', { time: 0.6 });
await select([]);
await shot('03-onion', viewport);
await cmd('view.toggleOnion');

// 스킨: 파란 모자
await cmd('mode.toggle');
await run('skin.create', { name: '파란모자' });
await run('skin.setActive', { name: '파란모자' });
await run('attachment.create', { slot: '모자', name: '모자', skin: '파란모자', type: 'region', props: { path: 'hat-blue', x: 6, rotation: -90, width: 56, height: 24 } });
await select([{ kind: 'skin', name: '파란모자' }]);
await frame();
await shot('06-skin');
await run('skin.setActive', { name: 'default' });

// 이벤트
await run('event.create', { name: '발소리', props: { int: 1, string: '쿵' } });
await select([{ kind: 'event', name: '발소리' }]);
await shot('06-event-props', page.locator('.props-panel'));

// IK: 오른팔 끝을 따라가는 목표 본
await run('bone.create', { parent: 'root', name: '손목표', x: 60, y: 120 });
await run('constraint.create', { type: 'ik', name: '오른팔IK', bones: ['오른팔'], target: '손목표' });
await select([{ kind: 'constraint', name: '오른팔IK' }]);
await frame();
await shot('05-ik');
await run('constraint.delete', { name: '오른팔IK' });
await run('bone.delete', { name: '손목표' });

// 대화상자: 내보내기, PNG 시퀀스, 단축키
await cmd('file.exportSpine');
await shot('07-export-spine', page.getByRole('dialog'));
await page.getByRole('dialog').getByRole('button', { name: '취소' }).click();
await cmd('file.exportPngSequence');
await shot('07-export-png', page.getByRole('dialog'));
await page.getByRole('dialog').getByRole('button', { name: '취소' }).click();
await cmd('help.shortcuts');
await shot('08-shortcuts', page.getByRole('dialog'));
await page.getByRole('dialog').getByRole('button', { name: '닫기' }).click();

// ---------- 4. 메시와 가중치(새 문서) ----------
await cmd('file.new');
const dlgNo = page.getByRole('button', { name: '저장 안 함' });
if (await dlgNo.isVisible().catch(() => false)) await dlgNo.click();
await settle(500);
// 꼬리 이미지(128×32)는 첨부물 원점을 가운데로 놓이므로, 본 사슬을 -64에서 시작한다
await run('bone.create', { parent: 'root', name: '꼬리1', x: -64, y: 0, length: 43 });
await run('bone.create', { parent: '꼬리1', name: '꼬리2', x: 43, y: 0, length: 43 });
await run('bone.create', { parent: '꼬리2', name: '꼬리3', x: 43, y: 0, length: 42 });
await run('slot.create', { bone: 'root', name: '꼬리' });
await run('attachment.create', { slot: '꼬리', name: 'tail', type: 'mesh', props: { width: 128, height: 32 } });
const tailSel = { kind: 'attachment', name: 'tail', slot: '꼬리', skin: 'default' };
// 휘어 보이도록 가로로 8칸 나눈 정점(아래 왼→오, 위 오→왼 = 외곽 순서)
const xs = [-64, -48, -32, -16, 0, 16, 32, 48, 64];
const ring = [...xs.map((x) => [x, -16]), ...[...xs].reverse().map((x) => [x, 16])];
await run('mesh.setVertices', {
  slot: '꼬리', name: 'tail',
  vertices: ring.flat(),
  uvs: ring.flatMap(([x, y]) => [(x + 64) / 128, (16 - y) / 32]),
  hull: ring.length,
});
// 칸마다 삼각형 두 개(아래 i, i+1 / 위 i, i+1). 위쪽 정점 번호는 거꾸로 붙어 있다
const n = xs.length;
const top = (i) => 2 * n - 1 - i;
const triangles = [];
for (let i = 0; i < n - 1; i++) triangles.push(i, i + 1, top(i + 1), i, top(i + 1), top(i));
await run('mesh.setTriangles', { slot: '꼬리', name: 'tail', triangles });
await select([tailSel]);
await frame();
await shot('04-mesh-props', page.locator('.props-panel'));
await page.getByTestId('mesh-edit-toggle').click();
await shot('04-mesh-edit', viewport);
await page.keyboard.press('Escape');
await run('mesh.bind', { slot: '꼬리', name: 'tail', bones: ['꼬리1', '꼬리2', '꼬리3'] });
await run('weights.auto', { slot: '꼬리', name: 'tail', bones: ['꼬리1', '꼬리2', '꼬리3'] });
await run('bone.setTransform', { name: '꼬리2', rotation: 30 });
await run('bone.setTransform', { name: '꼬리3', rotation: 35 });
await select([{ kind: 'bone', name: '꼬리3' }]);
await cmd('tool.rotate');
await shot('04-bend', viewport);

// ---------- 9. MCP 연결 ----------
// 이 스크립트 안에 MCP 서버 중계소를 열고(빈 포트), AI 프로그램 둘(Claude Code, Cursor)이 붙은 모습을 찍는다
const aiPort = await new Promise((resolve) => {
  const srv = createServer();
  srv.listen(0, '127.0.0.1', () => {
    const { port } = srv.address();
    srv.close(() => resolve(port));
  });
});
const hub = new Link({ name: 'Claude Code', port: aiPort, origins: [] });
const second = new Link({ name: 'Cursor', port: aiPort, origins: [] });
await hub.start();
await second.start();
await openMenu('도구', 'MCP 연결');
await shot('09-tools-menu', null, { x: 0, y: 0, width: 640, height: 220 });
await page.locator('.menu-dropdown .menu-item', { hasText: 'MCP 연결' }).last().click();
const aiDialog = page.getByRole('dialog', { name: 'MCP 연결' });
await aiDialog.getByLabel('포트').fill(String(aiPort));
await aiDialog.getByRole('button', { name: '연결', exact: true }).click();
await aiDialog.getByTestId('ai-clients').filter({ hasText: 'Cursor' }).waitFor();
// AI가 꼬리를 더 휘게 한다(창에 바로 보이고 최근 작업에 남는다)
await second.call('core', { request: { method: 'execute', id: 'bone.setTransform', args: { name: '꼬리3', rotation: 60 } } });
await aiDialog.getByTestId('ai-last').filter({ hasText: 'Cursor' }).waitFor();
// 포트 칸은 연결 중에 잠기므로 기본값으로 보이게 바꿔 찍는다(설명서의 기본 포트)
await page.evaluate(() => {
  const input = document.querySelector('[aria-label="포트"]');
  if (input) input.value = '47821';
  const st = document.querySelector('[data-testid="ai-status"]');
  if (st) st.textContent = st.textContent.replace(/127\.0\.0\.1:\d+/, '127.0.0.1:47821');
});
await shot('09-ai-connect', aiDialog);
await aiDialog.getByRole('button', { name: '닫기' }).click();
await callouts([[1, '[data-testid="status-ai"]', -34, -4]]);
await shot('09-status-ai', null, { x: 760, y: 860, width: 680, height: 40 });
await clearCallouts();
await shot('09-ai-window');
second.close();
hub.close();

await browser.close();

// I03/B04: 생성 도구로 뷰포트를 끌어 본 만들기, 회전 도구로 끌기 → 되돌리기 한 번에 원래대로.
import { expect, test } from '@playwright/test';
import { focusBody, openApp } from './helpers';

test('e_i03_viewport_create_and_rotate_drag', async ({ page }) => {
  await openApp(page);
  const vp = page.getByTestId('viewport');
  const box = (await vp.boundingBox())!;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  const bones = () => page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.bones);

  // 생성 도구(N): x=20에서 오른쪽으로 100px 끌기 → root 아래 길이 100 본(확대 100%)
  await focusBody(page);
  await page.keyboard.press('n');
  await page.mouse.move(cx + 20, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 70, cy, { steps: 3 });
  await page.mouse.move(cx + 120, cy, { steps: 3 });
  await page.mouse.up();
  await expect.poll(async () => (await bones()).length).toBe(2);
  const b = (await bones())[1];
  expect(b.parent).toBe('root');
  expect(b.length).toBeCloseTo(100, 0);
  expect(b.rotation).toBeCloseTo(0, 1);
  expect(b.x).toBeCloseTo(20, 0);

  // 회전 도구(C): 본 끝을 위로 끌기 → 회전 약 +45°
  await focusBody(page);
  await page.keyboard.press('c');
  await page.mouse.move(cx + 100, cy);
  await page.mouse.down();
  await page.mouse.move(cx + 100, cy - 40, { steps: 4 });
  await page.mouse.move(cx + 100, cy - 80, { steps: 4 });
  await page.mouse.up();
  const rotated = (await bones())[1].rotation;
  expect(rotated).toBeGreaterThan(30);

  // 되돌리기 한 번 = 끌기 한 번
  await focusBody(page);
  await page.keyboard.press('Control+z');
  await expect.poll(async () => (await bones())[1].rotation).toBeCloseTo(0, 1);
  expect((await bones()).length).toBe(2);
});

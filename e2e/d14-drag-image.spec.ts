// D14: 이동 도구(V)로 뷰포트의 그림을 끌면 첨부물 위치가 바뀌고, 본은 그대로, 되돌리기 한 번에 원래대로.
import { expect, test } from '@playwright/test';
import { focusBody, openApp } from './helpers';

test('e_d14_drag_image_moves_attachment', async ({ page }) => {
  await openApp(page);
  // 본 'b'(월드 100, 0, 길이 0)에 60×60 그림 'img'
  await page.evaluate(() => {
    const s = (window as any).__kgbone;
    s.doc.run('bone.create', { parent: 'root', name: 'b', x: 100, y: 0 });
    s.doc.run('slot.create', { bone: 'b', name: 's' });
    s.doc.run('attachment.create', { slot: 's', name: 'img', type: 'region', props: { width: 60, height: 60 } });
  });
  const att = () => page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.skins[0].attachments.s.img);
  const bone = () => page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.bones.find((b: { name: string }) => b.name === 'b'));
  expect(await att()).toMatchObject({ x: 0, y: 0 });

  const box = (await page.getByTestId('viewport').boundingBox())!;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  // 확대 100%, 화면 가운데 = 월드 (0, 0), y는 위로. 본에서 떨어진 그림 모서리 쪽 (120, 20)을 잡는다
  await focusBody(page);
  await page.keyboard.press('v');
  await page.mouse.move(cx + 120, cy - 20);
  await page.mouse.down();
  await page.mouse.move(cx + 135, cy - 30, { steps: 3 });
  await page.mouse.move(cx + 150, cy - 40, { steps: 3 });
  await page.mouse.up();

  await expect.poll(async () => (await att()).x).toBeCloseTo(30, 0);
  expect((await att()).y).toBeCloseTo(20, 0);
  expect(await bone()).toMatchObject({ x: 100, y: 0 });

  // 끌기 한 번 = 되돌리기 한 번
  await focusBody(page);
  await page.keyboard.press('Control+z');
  await expect.poll(async () => (await att()).x).toBe(0);
  expect((await att()).y).toBe(0);
});

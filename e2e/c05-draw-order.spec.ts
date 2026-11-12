// C05: 트리의 그리기 순서에서 슬롯을 끌어 순서 바꾸기(slot.reorder, 셋업 모드), 되돌리기 한 번.
import { expect, test } from '@playwright/test';
import { focusBody, openApp } from './helpers';

test('e_c05_drag_slot_in_draw_order', async ({ page }) => {
  await openApp(page);
  await page.evaluate(() => (window as any).__kgbone.files.open('/samples/kgbone/full-4.3.kgbone', { skipConfirm: true }));
  // 시험 파일은 애니메이트 모드로 저장돼 있다 → 셋업 모드로
  if ((await page.locator('.app').getAttribute('data-mode')) === 'animate') await page.getByTestId('mode-setup').click();
  await expect(page.locator('.app')).toHaveAttribute('data-mode', 'setup');
  const order = () => page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.slots.map((s: { name: string }) => s.name));
  expect(await order()).toEqual(['꼬리', '몸', '보석']);
  await page.locator('[role="treeitem"][data-name="그리기 순서"]').click();
  const row = (name: string) => page.locator(`[role="treeitem"][data-kind="slot"][data-name="${name}"]`).last();
  await row('보석').dragTo(row('꼬리'));
  await expect.poll(order).toEqual(['보석', '꼬리', '몸']);
  await focusBody(page);
  await page.keyboard.press('Control+z');
  await expect.poll(order).toEqual(['꼬리', '몸', '보석']);
});

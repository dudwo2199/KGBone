// D01: 트리의 이미지 항목에서 고른 슬롯에 그림 붙이기(속성 버튼, 오른쪽 클릭 메뉴).
// 이미지를 클릭해도 앞서 고른 슬롯이 붙일 곳으로 남아야 한다.
import { expect, test, type Page } from '@playwright/test';
import { openApp, treeItem } from './helpers';

const attachments = (page: Page, slot: string) =>
  page.evaluate((slot) => Object.keys((window as any).__kgbone.doc.get().project.skeleton.skins[0].attachments[slot] ?? {}), slot);

async function setup(page: Page) {
  await openApp(page);
  await page.evaluate(() => (window as any).__kgbone.doc.run('slot.create', { bone: 'root', name: 's' }));
  await page.locator('.tree-body [role="treeitem"]', { hasText: '이미지' }).locator('.tree-twisty').click();
  await treeItem(page, 'slot', 's').click();
}

test('e_d01_attach_image_from_tree_props_button', async ({ page }) => {
  await setup(page);
  await treeItem(page, 'image', 'head').click();
  await page.getByRole('button', { name: '선택한 슬롯에 붙이기' }).click();
  await expect.poll(() => attachments(page, 's')).toEqual(['head']);
  await expect(treeItem(page, 'attachment', 'head')).toBeVisible();
});

test('e_d01_delete_with_image_selected_keeps_slot', async ({ page }) => {
  // 이미지를 고르면 붙일 슬롯이 선택에 남지만, Delete가 그 슬롯을 지우면 안 된다(이미지는 지울 수 없는 항목)
  await setup(page);
  await treeItem(page, 'image', 'head').click();
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press('Delete');
  await page.waitForTimeout(200);
  expect(await page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.slots.map((x: { name: string }) => x.name))).toEqual(['s']);
});

test('e_d01_attach_image_from_tree_context_menu', async ({ page }) => {
  await setup(page);
  await treeItem(page, 'image', 'body').click({ button: 'right' });
  await page.locator('.context-menu').getByText('선택한 슬롯에 이미지 붙이기').click();
  await expect.poll(() => attachments(page, 's')).toEqual(['body']);
});

test('e_d01_attach_image_without_slot_warns', async ({ page }) => {
  await openApp(page);
  await page.locator('.tree-body [role="treeitem"]', { hasText: '이미지' }).locator('.tree-twisty').click();
  await treeItem(page, 'image', 'head').click();
  await page.getByRole('button', { name: '선택한 슬롯에 붙이기' }).click();
  await expect(page.getByTestId('status-message')).toContainText('슬롯을 먼저 고르세요');
  expect(await page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.slots.length)).toBe(0);
});

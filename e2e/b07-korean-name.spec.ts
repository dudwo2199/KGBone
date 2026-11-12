// B07/I02: 한글 이름 입력(조합 이벤트 흉내) → 이름 바꾸기 → 트리에 '몸통'. 조합 중 단축키는 무시된다.
import { expect, test } from '@playwright/test';
import { focusBody, openApp, treeItem } from './helpers';

test('e_b07_korean_rename_in_tree', async ({ page }) => {
  await openApp(page);
  await treeItem(page, 'bone', 'root').click();
  await focusBody(page);
  await page.keyboard.press('Control+Shift+N');
  await expect(treeItem(page, 'bone', 'bone')).toBeVisible();

  // F2 → 트리 안 이름 칸
  await focusBody(page);
  await page.keyboard.press('F2');
  const input = page.getByLabel('새 이름');
  await expect(input).toBeFocused();
  await input.press('Control+a');

  // 조합 시작 → 글자 넣기 → 조합 중 'k'(단축키) → 조합 끝
  await input.dispatchEvent('compositionstart', { data: '' });
  await page.keyboard.insertText('몸통');
  await input.dispatchEvent('keydown', { key: 'k', isComposing: true, keyCode: 229 });
  await input.dispatchEvent('compositionend', { data: '몸통' });
  await expect(input).toHaveValue('몸통');
  await input.press('Enter');

  await expect(treeItem(page, 'bone', '몸통')).toBeVisible();
  await expect(treeItem(page, 'bone', 'bone')).toHaveCount(0);
  // 조합 중 'k'는 키 지정 명령으로 가지 않았다(셋업 모드라 키가 생기면 안 됨)
  const anims = await page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.animations.length);
  expect(anims).toBe(0);
});

test('e_b07_korean_search_in_tree', async ({ page }) => {
  await openApp(page);
  await page.evaluate(() => (window as any).__kgbone.files.open('/samples/kgbone/full-4.3.kgbone', { skipConfirm: true }));
  await expect(treeItem(page, 'bone', '몸통')).toBeVisible();
  const search = page.getByTestId('tree-search');
  await search.click();
  await search.dispatchEvent('compositionstart', { data: '' });
  await page.keyboard.insertText('꼬리');
  await search.dispatchEvent('compositionend', { data: '꼬리' });
  await expect(search).toHaveValue('꼬리');
  await expect(treeItem(page, 'bone', '꼬리1')).toBeVisible();
  await expect(treeItem(page, 'bone', '팔')).toHaveCount(0);
});

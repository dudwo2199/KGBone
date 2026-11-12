// L01: 새 파일 → 본 추가 → 슬롯 → 이미지 붙이기 → 애니메이트 모드 → 키 지정 → 재생 → 저장 → 새로 만들기 → 다시 열기
import { expect, test } from '@playwright/test';
import { focusBody, menu, openApp, treeItem } from './helpers';

test('e_l01_full_flow', async ({ page }) => {
  await openApp(page);

  // 새로 만들기(메뉴)
  await menu(page, '파일', '새로 만들기');
  await expect(page.getByTestId('doc-title')).toHaveText('제목 없음');

  // 본 추가: Ctrl+Shift+N (root 아래 'bone')
  await treeItem(page, 'bone', 'root').click();
  await focusBody(page);
  await page.keyboard.press('Control+Shift+N');
  await expect(treeItem(page, 'bone', 'bone')).toBeVisible();
  await expect(treeItem(page, 'bone', 'bone')).toHaveAttribute('aria-selected', 'true');

  // 슬롯 만들기(메뉴) → 선택한 본 아래
  await menu(page, '만들기', '슬롯');
  await expect(treeItem(page, 'slot', 'slot')).toBeVisible();

  // 이미지 첨부물: 만들기 › 첨부물 › 이미지 → head
  await menu(page, '만들기', '첨부물', '이미지');
  const dlg = page.getByRole('dialog', { name: '이미지 고르기' });
  await dlg.getByRole('option', { name: 'head', exact: true }).click();
  await dlg.getByRole('button', { name: '붙이기' }).click();
  await expect(treeItem(page, 'attachment', 'head')).toBeVisible();
  await expect(page.getByTestId('doc-title')).toHaveText('제목 없음*');

  // 슬롯 셋업 첨부물로 보이는지(핵심이 그리기 목록을 돌려준다)
  const drawCount = await page.evaluate(() => (window as any).__kgbone.pose.pose().draw.length);
  expect(drawCount).toBe(1);

  // 애니메이션 만들기 → 애니메이트 모드(Tab)
  await menu(page, '만들기', '애니메이션');
  await treeItem(page, 'bone', 'bone').click();
  await focusBody(page);
  await page.keyboard.press('Tab');
  await expect(page.locator('.app')).toHaveAttribute('data-mode', 'animate');

  // 시점 0: 회전 30 (속성에서 값 바꾸기 → 키)
  const rot = page.getByTestId('bone-rotation');
  await rot.click();
  await rot.fill('30');
  await rot.press('Enter');
  // 시점 10프레임으로 이동 후 K로 키 지정, 회전 90
  const frame = page.getByTestId('time-frame');
  await frame.click();
  await frame.fill('10');
  await frame.press('Enter');
  await expect(page.getByTestId('time-seconds')).toHaveText('0.33초');
  await rot.click();
  await rot.fill('90');
  await rot.press('Enter');
  await focusBody(page);
  await page.keyboard.press('k');
  await expect(page.locator('[data-row$=":rotate"] [data-testid="dope-key"]')).toHaveCount(2);

  // 재생(D) → 잠시 뒤 정지(D)
  await page.keyboard.press('d');
  await expect(page.getByTestId('statusbar')).toContainText('재생 중');
  await page.waitForTimeout(400);
  await page.keyboard.press('d');
  await expect(page.getByTestId('statusbar')).not.toContainText('재생 중');

  // 저장(Ctrl+S) → 가상 파일 대화상자
  await page.keyboard.press('Control+s');
  const save = page.getByRole('dialog', { name: '다른 이름으로 저장' });
  await save.getByLabel('파일 경로').fill('/samples/l01-test.kgbone');
  await save.getByRole('button', { name: '저장' }).click();
  await expect(page.getByTestId('doc-title')).toHaveText('l01-test.kgbone');

  // 새로 만들기 → 본이 사라진다
  await menu(page, '파일', '새로 만들기');
  await expect(treeItem(page, 'bone', 'bone')).toHaveCount(0);

  // 다시 열기
  await menu(page, '파일', '열기…');
  const open = page.getByRole('dialog', { name: '열기' });
  await open.getByRole('option', { name: '/samples/l01-test.kgbone' }).click();
  await open.getByRole('button', { name: '열기' }).click();
  await expect(page.getByTestId('doc-title')).toHaveText('l01-test.kgbone');
  await expect(treeItem(page, 'bone', 'bone')).toBeVisible();
  await expect(treeItem(page, 'attachment', 'head')).toBeVisible();

  // 키 그대로: 애니메이트 모드로 저장했으므로 도프시트에 키 2개
  const anim = await page.evaluate(() => (window as any).__kgbone.doc.get().project.skeleton.animations[0]);
  expect(anim.bones.bone.rotate.map((k: { time: number }) => Math.round(k.time * 30))).toEqual([0, 10]);
  if ((await page.locator('.app').getAttribute('data-mode')) !== 'animate') {
    await focusBody(page);
    await page.keyboard.press('Tab');
  }
  await expect(page.locator('[data-row$=":rotate"] [data-testid="dope-key"]')).toHaveCount(2);
});

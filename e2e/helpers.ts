// E2E 공용 도우미
import { expect, type Page } from '@playwright/test';

/** 메모리 파일 판(WebPlatform)으로 앱을 열고 준비될 때까지 기다린다 */
export async function openApp(page: Page): Promise<void> {
  await page.goto('/?mem=1');
  await expect(page.locator('.app[data-ready="true"]')).toBeVisible({ timeout: 30_000 });
}

/** 메뉴 막대 → 항목(하위 메뉴 경로) */
export async function menu(page: Page, top: string, ...items: string[]): Promise<void> {
  await page.locator('.menubar-button', { hasText: top }).dispatchEvent('mousedown');
  for (const [i, label] of items.entries()) {
    const item = page.locator('.menu-dropdown .menu-item', { hasText: new RegExp(`^${escape(label)}`) }).last();
    if (i < items.length - 1) await item.hover();
    else await item.click();
  }
}

function escape(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function treeItem(page: Page, kind: string, name: string) {
  return page.locator(`.tree-body [role="treeitem"][data-kind="${kind}"][data-name="${name}"]`).first();
}

/** 뷰포트에 초점을 줘서 단축키가 입력칸으로 가지 않게 한다 */
export async function focusBody(page: Page): Promise<void> {
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
}

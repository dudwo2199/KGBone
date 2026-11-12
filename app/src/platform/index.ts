// 실행 환경에 맞는 Platform 고르기:
// Tauri 앱 → TauriPlatform / File System Access API가 있는 브라우저 → BrowserFsPlatform(실제 파일)
// 그 밖(또는 ?mem=1, 자동 테스트) → WebPlatform(메모리 파일).
import { browserFsAvailable, BrowserFsPlatform, type BrowserFsDialogs } from './browser-fs';
import type { Platform } from './platform';
import { WebPlatform, type WebDialogProvider } from './web';

export async function createPlatform(dialogs: WebDialogProvider & BrowserFsDialogs): Promise<Platform> {
  if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
    return (await import('./tauri')).createTauriPlatform();
  }
  const forceMemory = new URLSearchParams(location.search).has('mem') || navigator.webdriver === true;
  if (!forceMemory && browserFsAvailable()) return new BrowserFsPlatform(dialogs);
  return new WebPlatform(dialogs);
}

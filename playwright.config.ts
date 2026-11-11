import { defineConfig } from '@playwright/test';

// 이 PC는 스마트 앱 컨트롤 때문에 Playwright가 내려받는 브라우저가 막힐 수 있어, 설치된 Microsoft Edge를 쓴다.
export default defineConfig({
  testDir: 'e2e',
  timeout: 60_000,
  use: { channel: 'msedge', baseURL: 'http://localhost:1420', locale: 'ko-KR' },
  webServer: { command: 'npm run dev', url: 'http://localhost:1420', reuseExistingServer: true, timeout: 180_000 },
});

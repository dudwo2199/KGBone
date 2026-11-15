import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';
import { precachePlugin } from '../tools/vite-precache.mjs';

const root = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  root,
  // 상대 경로: GitHub Pages(/KGBone/ 아래), 내 PC 서버, Tauri 어디서든 같은 빌드가 돈다
  base: './',
  plugins: [react(), precachePlugin()],
  // Tauri가 쓰는 고정 포트
  server: { port: 1420, strictPort: true, fs: { allow: ['..'] } },
  clearScreen: false,
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts', 'src/**/*.test.tsx'] },
});

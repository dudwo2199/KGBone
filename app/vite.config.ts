import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('.', import.meta.url));

export default defineConfig({
  root,
  plugins: [react()],
  // Tauri가 쓰는 고정 포트
  server: { port: 1420, strictPort: true, fs: { allow: ['..'] } },
  clearScreen: false,
  build: { outDir: 'dist', emptyOutDir: true, target: 'es2022' },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts', 'src/**/*.test.tsx'] },
});

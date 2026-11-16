// 설치형 웹 앱(PWA) 구성 확인: 매니페스트, 아이콘, 서비스 워커 목록 채우기
import { describe, expect, it } from 'vitest';
import html from '../index.html?raw';
import manifestText from '../public/manifest.webmanifest?raw';
import swSource from '../public/sw.js?raw';
// @ts-expect-error 빌드 도구(.mjs)라 타입 선언이 없다
import { fillServiceWorker } from '../../tools/vite-precache.mjs';

const manifest = JSON.parse(manifestText);
/** app/public/icons 안에 실제로 있는 파일(매니페스트 기준 상대 경로) */
const iconFiles = Object.keys(import.meta.glob('../public/icons/*.png')).map((p) => p.replace('../public/', ''));

describe('설치형 웹 앱 구성', () => {
  it('pwa_manifest_is_installable_and_relative', () => {
    expect(manifest.name).toBe('KGBone');
    expect(manifest.display).toBe('standalone');
    // GitHub Pages는 /KGBone/ 아래에 올라가므로 경로가 모두 상대여야 한다
    for (const v of [manifest.start_url, manifest.scope, manifest.id]) expect(v).toBe('./');
    const sizes = manifest.icons.map((i: { sizes: string }) => i.sizes);
    expect(sizes).toEqual(expect.arrayContaining(['192x192', '512x512']));
    for (const icon of manifest.icons) {
      expect(icon.src.startsWith('/')).toBe(false);
      expect(iconFiles).toContain(icon.src);
    }
  });

  it('f_j09_pwa_manifest_declares_kgbone_file_handler', () => {
    const exts = manifest.file_handlers.flatMap((h: { accept: Record<string, string[]> }) => Object.values(h.accept).flat());
    expect(exts).toContain('.kgbone');
    expect(manifest.file_handlers[0].action).toBe('./');
    // Chrome은 file_handlers에 아이콘이 없으면 탐색기의 파일 아이콘을 비워 둔다(Edge는 앱 아이콘을 대신 씀)
    const icons = manifest.file_handlers[0].icons as { src: string; sizes: string }[];
    expect(icons.map((i) => i.sizes)).toEqual(expect.arrayContaining(['192x192', '512x512']));
    for (const icon of icons) expect(iconFiles).toContain(icon.src);
  });

  it('pwa_index_links_manifest', () => {
    expect(html).toContain('rel="manifest" href="./manifest.webmanifest"');
  });

  it('pwa_service_worker_gets_file_list_and_version', () => {
    const files: [string, string][] = [
      ['sw.js', 'x'],
      ['index.html', '<html>'],
      ['assets/a.js', 'a'],
      ['kgbone.wasm', 'w'],
    ];
    const out = fillServiceWorker(swSource, files) as string;
    expect(out).toContain('const FILES = ["assets/a.js","./","kgbone.wasm"];');
    const version = /const VERSION = "([0-9a-f]{12})";/.exec(out)?.[1];
    expect(version).toBeTruthy();
    // 내용이 바뀌면 판 번호도 바뀐다(옛 캐시가 지워진다)
    const changed = fillServiceWorker(swSource, [...files.slice(0, 3), ['kgbone.wasm', 'w2']]) as string;
    expect(changed).not.toContain(`"${version}"`);
    // 파일 순서는 결과에 영향을 주지 않는다
    expect(fillServiceWorker(swSource, [...files].reverse())).toBe(out);
  });

  it('pwa_service_worker_fill_fails_without_markers', () => {
    expect(() => fillServiceWorker('// 빈 파일', [])).toThrow();
  });
});

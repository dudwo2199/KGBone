// Tauri 앱 본체용 Platform 구현(interface/tauri-commands.md). 6단계 "실행 파일 포장" 담당이 채운다.
import type { Platform } from './platform';

export async function createTauriPlatform(): Promise<Platform> {
  throw new Error('Tauri 플랫폼이 아직 구현되지 않았습니다');
}

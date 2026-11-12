// 화면 전체가 함께 쓰는 서비스 묶음(React 컨텍스트로 내려준다).
import { createContext, useContext } from 'react';
import { Player } from './anim/player';
import { buildRegistry } from './commands/handlers';
import type { CommandRegistry } from './commands/registry';
import type { Core } from './core/bridge';
import { FileFlow } from './files/fileFlow';
import type { Platform } from './platform/platform';
import { SAMPLE_DIR, WebPlatform } from './platform/web';
import { DocStore } from './state/document';
import { ImageLibrary } from './state/images';
import { PoseService } from './state/pose';
import { pruneSelection } from './state/selection';
import { UiStore } from './state/ui';
import type { DialogService } from './ui/dialogs/service';

export interface Services {
  core: Core;
  platform: Platform;
  doc: DocStore;
  ui: UiStore;
  images: ImageLibrary;
  dialogs: DialogService;
  files: FileFlow;
  player: Player;
  pose: PoseService;
  commands: CommandRegistry<Services>;
  /** 명령 실행(메뉴·단축키·버튼 공용). 오류는 알림으로 */
  run(id: string, args?: Record<string, unknown>): void;
}

export function createServices(core: Core, platform: Platform, dialogs: DialogService): Services {
  const ui = new UiStore();
  const notify = ui.notify;
  const doc = new DocStore(core, notify, platform instanceof WebPlatform ? SAMPLE_DIR : '');
  const images = new ImageLibrary(platform, core, notify);
  const pose = new PoseService(core, doc, ui, notify);
  const player = new Player(doc, ui);
  const commands = buildRegistry();
  const s: Services = {
    core,
    platform,
    doc,
    ui,
    images,
    dialogs,
    pose,
    player,
    commands,
    files: null as unknown as FileFlow,
    run(id, args = {}) {
      commands.run(s, id, args).catch((e: unknown) => notify('error', e instanceof Error ? e.message : String(e)));
    },
  };
  s.files = new FileFlow({
    core,
    platform,
    doc,
    images,
    dialogs,
    notify,
    onDocumentReplaced: () => {
      player.stop();
      const ed = doc.get().project.editor;
      ui.set({ selection: [], selectedKeys: [], graphRow: null, view: { ...ed.view }, playTime: ed.time, loop: ed.playback.loop, speed: ed.playback.speed, onion: ed.onion.enabled });
    },
  });
  // 문서가 바뀌면 사라진 항목은 선택에서 뺀다
  doc.subscribe(() => {
    const sel = ui.get().selection;
    const next = pruneSelection(sel, doc.get().project);
    if (next.length !== sel.length) ui.set({ selection: next });
  });
  return s;
}

export const ServicesContext = createContext<Services | null>(null);

export function useServices(): Services {
  const s = useContext(ServicesContext);
  if (!s) throw new Error('서비스가 없습니다');
  return s;
}

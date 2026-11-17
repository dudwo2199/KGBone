// MCP 연결로 창에 온 일 처리: MCP 서버(tools/mcp/live.mjs)가 보내는 요청을 이 창의 핵심·파일 흐름·명령으로 실행한다.
// 편집은 모두 창의 문서·되돌리기 기록에 쌓여 화면에 바로 보이고 Ctrl+Z로 되돌릴 수 있다.
import type { Core, PngSequenceOptions, SpineExportOptions } from '../core/bridge';
import type { PoseRequest } from '../core/types';
import { DISK_PREFIX } from '../platform/diskPlatform';
import type { Services } from '../services';
import { DialogService } from '../ui/dialogs/service';
import type { AgentHandler } from './agentLink';

type Params = Record<string, unknown>;

/** Tauri 판은 디스크 경로를 바로 쓴다 */
function localPath(s: Services, p: string): string {
  return s.platform.kind === 'tauri' && p.startsWith(DISK_PREFIX) ? p.slice(DISK_PREFIX.length) : p;
}
/** 돌려줄 때는 MCP 서버가 알아보게 'disk:'를 붙인다 */
function remotePath(s: Services, p: string | null): string | null {
  if (!p) return p;
  return s.platform.kind === 'tauri' && /^([a-zA-Z]:[\\/]|\/)/.test(p) ? DISK_PREFIX + p : p;
}

/** 핵심 요청(api.rs의 method)을 창의 Core로 */
async function coreRequest(s: Services, req: Params): Promise<unknown> {
  const core: Core = s.core;
  switch (req.method) {
    case 'execute':
      try {
        return core.execute(String(req.id), (req.args ?? {}) as Params);
      } finally {
        s.doc.refresh();
      }
    case 'project':
      return core.project();
    case 'state':
      return core.state();
    case 'pose':
      return core.pose(req.request as PoseRequest);
    case 'exportPngSequence':
      await s.files.registerPngs();
      return core.exportPngSequence((req.options ?? {}) as PngSequenceOptions);
    case 'exportSpineBundle':
      await s.files.registerPngs();
      return core.exportSpineBundle((req.options ?? {}) as SpineExportOptions);
    default:
      throw new Error(`창에서 처리하지 않는 핵심 요청입니다: ${String(req.method)}`);
  }
}

/** 뷰포트 그림(PNG, base64) */
function screenshot(): { base64: string; width: number; height: number } {
  const canvas = document.querySelector<HTMLCanvasElement>('[data-testid="viewport"] canvas');
  if (!canvas) throw new Error('뷰포트를 찾지 못했습니다');
  const url = canvas.toDataURL('image/png');
  return { base64: url.slice(url.indexOf(',') + 1), width: canvas.width, height: canvas.height };
}

/**
 * 화면 명령 실행. 명령이 대화상자를 열면(사용자 입력 필요) 바로 취소하고 실패로 알린다 —
 * 그대로 두면 사용자가 닫을 때까지 AI가 기다리게 된다.
 */
export async function runUiCommand(s: Services, id: string, args: Params): Promise<void> {
  const before = new Set(s.dialogs.get().stack.map((d) => d.id));
  let opened: string | null = null;
  const unsubscribe = s.dialogs.subscribe(() => {
    for (const d of s.dialogs.get().stack) {
      if (before.has(d.id)) continue;
      before.add(d.id);
      opened ??= DialogService.titleOf(d.spec);
      s.dialogs.cancel(d);
    }
  });
  try {
    await s.commands.run(s, id, args);
  } finally {
    unsubscribe();
  }
  if (opened) {
    throw new Error(`${id}는 대화상자(${opened})를 여는 명령이라 실행하지 않고 닫았습니다. 필요한 값은 명령 인자로 넘기거나(command_list의 args), 사용자에게 창에서 직접 해 달라고 하세요.`);
  }
}

export function createAgentHandler(s: Services): AgentHandler {
  return async (method, params) => {
    switch (method) {
      case 'core':
        return coreRequest(s, (params.request ?? {}) as Params);
      case 'doc.new': {
        if (!(await s.files.newFile())) throw new Error('사용자가 취소했습니다(저장하지 않은 변경이 있음)');
        return { imageWarnings: [] };
      }
      case 'doc.open': {
        const kind = params.kind === 'spineJson' ? 'spineJson' : 'kgbone';
        const warnings = await s.files.openForAgent(localPath(s, String(params.path)), kind);
        return { warnings, imageWarnings: [] };
      }
      case 'doc.save': {
        const p = typeof params.path === 'string' ? localPath(s, params.path) : null;
        return { path: remotePath(s, await s.files.saveForAgent(p)) };
      }
      case 'doc.info': {
        const d = s.doc.get();
        return {
          filePath: remotePath(s, d.filePath),
          imagesDir: remotePath(s, s.files.imagesDir()),
          images: s.images.get().entries.map((e) => e.name),
          nameStem: s.doc.displayName().replace(/\.[^.]+$/, ''),
        };
      }
      case 'images.reload':
        await s.images.open(s.files.imagesDir());
        return { warnings: [] };
      case 'ui.run':
        await runUiCommand(s, String(params.id), (params.args ?? {}) as Params);
        return null;
      case 'ui.screenshot':
        // 바로 전 편집이 그려지도록 한 프레임 기다린다
        await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
        return screenshot();
      default:
        throw new Error(`창에서 모르는 요청입니다: ${method}`);
    }
  };
}

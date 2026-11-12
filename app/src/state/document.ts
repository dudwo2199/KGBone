// 열린 문서 저장소: Core를 감싸 명령 실행 후 project()·state()를 다시 읽어 구독자에게 알린다.
import type { Core } from '../core/bridge';
import type { DocState, Project } from '../core/types';
import { Store } from './store';

export interface DocSnapshot {
  project: Project;
  docState: DocState;
  /** 저장된 파일 경로. 새 문서면 null */
  filePath: string | null;
  /** 이미지 경로 계산 기준 폴더(파일 폴더 또는 가져온 파일 폴더) */
  baseDir: string;
  /** 표시용 문서 이름(경로가 없을 때) */
  untitledName: string;
  /** 명령을 실행할 때마다 1씩 는다(포즈 캐시 등에서 쓴다) */
  revision: number;
}

export type Notify = (level: 'info' | 'warn' | 'error', text: string) => void;

export interface RunResult<T> {
  ok: boolean;
  value?: T;
  error?: string;
}

export class DocStore extends Store<DocSnapshot> {
  constructor(
    readonly core: Core,
    private notify: Notify,
    readonly defaultBaseDir: string,
  ) {
    super({
      project: safeProject(core),
      docState: safeState(core),
      filePath: null,
      baseDir: defaultBaseDir,
      untitledName: '제목 없음',
      revision: 0,
    });
  }

  /** core에서 project/state를 다시 읽는다. */
  refresh(): void {
    this.set((s) => ({ project: safeProject(this.core), docState: safeState(this.core), revision: s.revision + 1 }));
  }

  /** 명령 실행. 실패하면 알림을 띄우고 ok=false를 돌려준다(앱은 계속 동작). */
  run<T = unknown>(id: string, args: Record<string, unknown> = {}, opts: { quiet?: boolean } = {}): RunResult<T> {
    try {
      const value = this.core.execute<T>(id, args);
      this.refresh();
      return { ok: true, value };
    } catch (e) {
      const msg = errorText(e);
      if (!opts.quiet) this.notify('error', msg);
      this.refresh();
      return { ok: false, error: msg };
    }
  }

  /** 여러 명령을 되돌리기 한 번으로 묶는다. */
  group<T>(label: string, fn: () => T): T {
    this.beginGroup(label);
    try {
      return fn();
    } finally {
      this.endGroup();
    }
  }
  beginGroup(label: string): void {
    try {
      this.core.beginGroup(label);
    } catch (e) {
      this.notify('error', errorText(e));
    }
  }
  endGroup(): void {
    try {
      this.core.endGroup();
    } catch (e) {
      this.notify('error', errorText(e));
    }
    this.refresh();
  }

  /** 새 문서·열기 후 파일 정보 바꾸기 */
  setFile(filePath: string | null, baseDir: string, untitledName = '제목 없음'): void {
    this.set({ filePath, baseDir, untitledName });
    this.refresh();
  }

  displayName(): string {
    const s = this.get();
    if (s.filePath) {
      return s.filePath.split(/[\\/]/).pop() ?? s.filePath;
    }
    return s.untitledName;
  }
}

export function errorText(e: unknown): string {
  if (e instanceof Error) return e.message;
  return String(e);
}

function safeProject(core: Core): Project {
  try {
    return core.project();
  } catch {
    return emptyProject();
  }
}
function safeState(core: Core): DocState {
  try {
    return core.state();
  } catch {
    return { dirty: false, canUndo: false, canRedo: false, undoLabel: null, redoLabel: null };
  }
}

export function emptyProject(): Project {
  return {
    formatVersion: 1,
    generator: 'KGBone',
    imagesPath: './images',
    skeleton: { x: 0, y: 0, width: 0, height: 0, fps: 30, referenceScale: 100, bones: [], slots: [], constraints: [], skins: [], events: [], animations: [] },
    editor: {
      mode: 'setup',
      view: { x: 0, y: 0, zoom: 1 },
      activeAnimation: null,
      activeSkin: null,
      time: 0,
      treeExpanded: [],
      onion: { enabled: false, before: 2, after: 2, step: 1 },
      playback: { speed: 1, loop: true },
    },
  };
}

// 화면 전용 상태: 선택, 도구·축·옵션, 재생, 뷰(확대·이동), 표시 옵션, 알림.
import type { Sel } from './selection';
import { Store } from './store';

export type ToolId = 'pose' | 'create' | 'weights' | 'rotate' | 'translate' | 'scale' | 'shear';
export type Axis = 'local' | 'parent' | 'world';
export type BottomTab = 'dopesheet' | 'graph';

export interface KeyRef {
  target: { kind: 'bone' | 'slot' | 'constraint' | 'deform' | 'sequence' | 'drawOrder' | 'event'; name?: string; skin?: string; slot?: string; attachment?: string };
  timeline: string;
  time: number;
}

export interface Message {
  id: number;
  level: 'info' | 'warn' | 'error';
  text: string;
}

export interface UiState {
  selection: Sel[];
  tool: ToolId;
  prevTool: ToolId;
  axis: Axis;
  compensate: boolean;
  /** 부모 지정(P) 고르기 중 */
  pickParent: boolean;
  playing: boolean;
  reverse: boolean;
  loop: boolean;
  speed: number;
  /** 재생 중 시점(초). 멈춰 있으면 문서의 editor.time을 쓴다 */
  playTime: number;
  view: { x: number; y: number; zoom: number };
  viewSize: { w: number; h: number };
  showBones: boolean;
  showImages: boolean;
  onion: boolean;
  bottomTab: BottomTab;
  selectedKeys: KeyRef[];
  /** 도프시트에서 고른 줄(그래프 대상) */
  graphRow: string | null;
  cursor: { x: number; y: number } | null;
  messages: Message[];
  status: Message | null;
  /** 이름 바꾸기 중인 트리 노드 키 */
  renaming: string | null;
  treeFilter: string;
  /** 메시 정점 편집 모드 */
  meshEdit: boolean;
  selectedVertices: number[];
  ready: boolean;
}

export function initialUi(): UiState {
  return {
    selection: [],
    tool: 'pose',
    prevTool: 'rotate',
    axis: 'local',
    compensate: false,
    pickParent: false,
    playing: false,
    reverse: false,
    loop: true,
    speed: 1,
    playTime: 0,
    view: { x: 0, y: 0, zoom: 1 },
    viewSize: { w: 800, h: 600 },
    showBones: true,
    showImages: true,
    onion: false,
    bottomTab: 'dopesheet',
    selectedKeys: [],
    graphRow: null,
    cursor: null,
    messages: [],
    status: null,
    renaming: null,
    treeFilter: '',
    meshEdit: false,
    selectedVertices: [],
    ready: false,
  };
}

export class UiStore extends Store<UiState> {
  private nextId = 1;
  constructor() {
    super(initialUi());
  }

  notify = (level: Message['level'], text: string): void => {
    const m: Message = { id: this.nextId++, level, text };
    this.set((s) => ({ status: m, messages: [...s.messages.slice(-49), m] }));
  };

  select(sel: Sel[]): void {
    this.set({ selection: sel });
  }

  setTool(tool: ToolId): void {
    const s = this.get();
    if (s.tool === tool) return;
    this.set({ tool, prevTool: s.tool });
  }

  /** 오른쪽 클릭: 직전 도구로 */
  swapTool(): void {
    const s = this.get();
    this.set({ tool: s.prevTool, prevTool: s.tool });
  }
}

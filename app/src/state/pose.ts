// 현재 포즈 계산(문서·시점·모드에 따라). 같은 입력이면 캐시를 돌려준다.
import type { Core } from '../core/bridge';
import type { PoseOutput, PoseRequest } from '../core/types';
import type { DocStore } from './document';
import type { UiStore } from './ui';

export const EMPTY_POSE: PoseOutput = { bones: [], locals: [], active: [], draw: [], overlays: [], events: [] };

export class PoseService {
  private cacheKey = '';
  private cached: PoseOutput = EMPTY_POSE;
  private lastError = '';

  constructor(
    private core: Core,
    private doc: DocStore,
    private ui: UiStore,
    private notify: (level: 'warn' | 'error', text: string) => void,
  ) {}

  /** 지금 보이는 시점(재생 중이면 재생 시점) */
  currentTime(): number {
    const u = this.ui.get();
    return u.playing ? u.playTime : this.doc.get().project.editor.time;
  }

  /** 지금 화면 상태의 포즈 요청 */
  request(time = this.currentTime()): PoseRequest {
    const p = this.doc.get().project;
    const ed = p.editor;
    if (ed.mode !== 'animate' || !ed.activeAnimation) return { animation: null, time: 0, skin: ed.activeSkin, physics: 'none', looping: false };
    return { animation: ed.activeAnimation, time, skin: ed.activeSkin, physics: 'none', looping: this.ui.get().loop };
  }

  /** 물리 없이 계산한 포즈(캐시) */
  pose(req: PoseRequest = this.request()): PoseOutput {
    const key = `${this.doc.get().revision}|${JSON.stringify(req)}`;
    if (key === this.cacheKey) return this.cached;
    this.cached = this.compute(req);
    this.cacheKey = key;
    return this.cached;
  }

  /** 캐시 없이 계산(재생 중 물리 진행) */
  compute(req: PoseRequest): PoseOutput {
    try {
      const out = this.core.pose(req);
      this.lastError = '';
      return out;
    } catch (e) {
      const msg = `포즈 계산 실패: ${e instanceof Error ? e.message : String(e)}`;
      if (msg !== this.lastError) this.notify('error', msg);
      this.lastError = msg;
      return EMPTY_POSE;
    }
  }
}

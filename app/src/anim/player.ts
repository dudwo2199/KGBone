// 재생기: requestAnimationFrame으로 시점을 진행한다. 멈추면 time.set으로 문서 시점을 저장한다.
import type { DocStore } from '../state/document';
import type { UiStore } from '../state/ui';
import { advancePlayback, snapToFrame } from './playback';
import { animationDuration } from './timeline';

export class Player {
  private raf = 0;
  private last = 0;
  /** 매 프레임 진행 후(뷰포트가 물리 dt와 함께 다시 그린다) */
  onFrame: ((dt: number, lastTime: number) => void) | null = null;

  constructor(
    private doc: DocStore,
    private ui: UiStore,
  ) {}

  duration(): number {
    const p = this.doc.get().project;
    return animationDuration(p.skeleton.animations.find((a) => a.name === p.editor.activeAnimation));
  }

  start(reverse: boolean): void {
    const p = this.doc.get().project;
    if (p.editor.mode !== 'animate' || !p.editor.activeAnimation) {
      this.ui.notify('warn', '애니메이트 모드에서 애니메이션을 고른 뒤 재생할 수 있습니다');
      return;
    }
    const dur = this.duration();
    let t = p.editor.time;
    if (!this.ui.get().loop && !reverse && t >= dur) t = 0;
    if (!this.ui.get().loop && reverse && t <= 0) t = dur;
    this.ui.set({ playing: true, reverse, playTime: t });
    this.last = performance.now();
    cancelAnimationFrame(this.raf);
    this.raf = requestAnimationFrame(this.tick);
  }

  stop(): void {
    if (!this.ui.get().playing) return;
    cancelAnimationFrame(this.raf);
    const fps = this.doc.get().project.skeleton.fps || 30;
    const t = snapToFrame(this.ui.get().playTime, fps);
    this.ui.set({ playing: false });
    this.doc.run('time.set', { time: t });
  }

  toggle(reverse = false): void {
    const u = this.ui.get();
    if (u.playing && u.reverse === reverse) this.stop();
    else if (u.playing) this.ui.set({ reverse });
    else this.start(reverse);
  }

  private tick = (now: number): void => {
    const dt = Math.min(0.1, Math.max(0, (now - this.last) / 1000));
    this.last = now;
    const u = this.ui.get();
    if (!u.playing) return;
    const lastTime = u.playTime;
    const next = advancePlayback({ time: u.playTime, playing: true, reverse: u.reverse, loop: u.loop, speed: u.speed }, dt, this.duration());
    this.ui.set({ playTime: next.time });
    this.onFrame?.(dt, lastTime);
    if (!next.playing) {
      this.stop();
      return;
    }
    this.raf = requestAnimationFrame(this.tick);
  };
}

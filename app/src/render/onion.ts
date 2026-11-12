// 오니언 스킨 시점 목록(G12).
export interface OnionOptions {
  before: number;
  after: number;
  /** 프레임 간격 */
  step: number;
  fps: number;
  duration: number;
  loop: boolean;
}

/** 현재 시점 앞뒤로 그릴 시점들(각각 가까운 것부터). 범위 밖은 반복이면 감고, 아니면 뺀다. */
export function onionTimes(current: number, o: OnionOptions): { before: number[]; after: number[] } {
  const dt = Math.max(1, o.step) / (o.fps > 0 ? o.fps : 30);
  const wrap = (t: number): number | null => {
    if (o.duration <= 0) return null;
    if (t >= -1e-9 && t <= o.duration + 1e-9) return Math.min(o.duration, Math.max(0, t));
    if (!o.loop) return null;
    return ((t % o.duration) + o.duration) % o.duration;
  };
  const before: number[] = [];
  const after: number[] = [];
  for (let i = 1; i <= o.before; i++) {
    const t = wrap(current - i * dt);
    if (t !== null) before.push(round(t));
  }
  for (let i = 1; i <= o.after; i++) {
    const t = wrap(current + i * dt);
    if (t !== null) after.push(round(t));
  }
  return { before, after };
}

function round(t: number): number {
  return Math.round(t * 1e6) / 1e6;
}

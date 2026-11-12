// 재생 진행 계산(G11): 속도·반복·거꾸로.
export interface PlaybackState {
  time: number;
  playing: boolean;
  reverse: boolean;
  loop: boolean;
  speed: number;
}

/** dt(실제 초)만큼 진행한 다음 상태. 반복이 꺼져 있으면 끝(거꾸로면 0)에서 멈춘다. */
export function advancePlayback(s: PlaybackState, dt: number, duration: number): PlaybackState {
  if (!s.playing) return s;
  const dir = s.reverse ? -1 : 1;
  let t = s.time + dt * s.speed * dir;
  if (duration <= 0) return { ...s, time: 0 };
  if (s.loop) {
    t = ((t % duration) + duration) % duration;
    return { ...s, time: t };
  }
  if (!s.reverse && t >= duration) return { ...s, time: duration, playing: false };
  if (s.reverse && t <= 0) return { ...s, time: 0, playing: false };
  return { ...s, time: t };
}

/** 프레임 단위로 맞춘 시점 */
export function snapToFrame(t: number, fps: number): number {
  if (fps <= 0) return t;
  return Math.round(t * fps) / fps;
}

/** 속도 범위 0.1~4배 */
export function clampSpeed(v: number): number {
  return Math.min(4, Math.max(0.1, v));
}

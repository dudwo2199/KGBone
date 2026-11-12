// 그래프(곡선) 편집기 좌표 변환(G10)과 곡선 계산.
import type { Curve, Key } from '../core/types';

export interface GraphView {
  /** 보이는 시간 범위 */
  t0: number;
  t1: number;
  /** 보이는 값 범위 */
  v0: number;
  v1: number;
  /** 그림 영역 픽셀 크기와 여백 */
  width: number;
  height: number;
  pad: number;
}

/** (시간, 값) → 화면 픽셀(y는 아래로) */
export function graphToScreen(g: GraphView, t: number, v: number): { x: number; y: number } {
  const w = g.width - 2 * g.pad;
  const h = g.height - 2 * g.pad;
  const dt = g.t1 - g.t0 || 1;
  const dv = g.v1 - g.v0 || 1;
  return { x: g.pad + ((t - g.t0) / dt) * w, y: g.pad + (1 - (v - g.v0) / dv) * h };
}

/** 화면 픽셀 → (시간, 값) */
export function screenToGraph(g: GraphView, x: number, y: number): { t: number; v: number } {
  const w = g.width - 2 * g.pad;
  const h = g.height - 2 * g.pad;
  const dt = g.t1 - g.t0 || 1;
  const dv = g.v1 - g.v0 || 1;
  return { t: g.t0 + ((x - g.pad) / w) * dt, v: g.v0 + (1 - (y - g.pad) / h) * dv };
}

/** 키 목록이 다 보이게 범위를 잡는다(값 범위 위아래 10% 여유). */
export function fitGraph(keys: Key[], channel: number, width: number, height: number, pad = 16): GraphView {
  let t0 = Infinity;
  let t1 = -Infinity;
  let v0 = Infinity;
  let v1 = -Infinity;
  for (const k of keys) {
    const v = k.v[channel] ?? 0;
    t0 = Math.min(t0, k.time);
    t1 = Math.max(t1, k.time);
    v0 = Math.min(v0, v);
    v1 = Math.max(v1, v);
    if (Array.isArray(k.curve)) {
      const c = k.curve.slice(channel * 4, channel * 4 + 4);
      if (c.length === 4) {
        v0 = Math.min(v0, c[1], c[3]);
        v1 = Math.max(v1, c[1], c[3]);
      }
    }
  }
  if (!isFinite(t0)) {
    t0 = 0;
    t1 = 1;
    v0 = 0;
    v1 = 1;
  }
  if (t1 - t0 < 1e-6) t1 = t0 + 1;
  if (v1 - v0 < 1e-6) {
    v0 -= 1;
    v1 += 1;
  }
  const m = (v1 - v0) * 0.1;
  return { t0, t1, v0: v0 - m, v1: v1 + m, width, height, pad };
}

/** 두 키 사이 기본 베지어 손잡이(1/3, 2/3 지점) — 채널마다 cx1, cy1, cx2, cy2 */
export function defaultBezier(a: Key, b: Key): number[] {
  const out: number[] = [];
  const n = Math.max(1, a.v.length);
  const dt = b.time - a.time;
  for (let i = 0; i < n; i++) {
    const va = a.v[i] ?? 0;
    const vb = b.v[i] ?? va;
    out.push(a.time + dt / 3, va, a.time + (dt * 2) / 3, vb);
  }
  return out;
}

/** 곡선 한 구간의 그림 점(시간·값). 선형·계단·베지어 */
export function segmentPoints(a: Key, b: Key, channel: number, curve: Curve, steps = 24): { t: number; v: number }[] {
  const va = a.v[channel] ?? 0;
  const vb = b.v[channel] ?? va;
  if (curve === 'stepped') return [{ t: a.time, v: va }, { t: b.time, v: va }, { t: b.time, v: vb }];
  if (Array.isArray(curve) && curve.length >= channel * 4 + 4) {
    const [cx1, cy1, cx2, cy2] = curve.slice(channel * 4, channel * 4 + 4);
    const pts: { t: number; v: number }[] = [];
    for (let i = 0; i <= steps; i++) {
      const s = i / steps;
      const u = 1 - s;
      const t = u * u * u * a.time + 3 * u * u * s * cx1 + 3 * u * s * s * cx2 + s * s * s * b.time;
      const v = u * u * u * va + 3 * u * u * s * cy1 + 3 * u * s * s * cy2 + s * s * s * vb;
      pts.push({ t, v });
    }
    return pts;
  }
  return [{ t: a.time, v: va }, { t: b.time, v: vb }];
}

/** 손잡이 하나를 옮긴 새 곡선 배열(시간은 구간 [t0, t1] 안으로 제한) */
export function moveHandle(curve: number[], channel: number, which: 1 | 2, t: number, v: number, t0: number, t1: number): number[] {
  const out = curve.slice();
  const i = channel * 4 + (which === 1 ? 0 : 2);
  out[i] = Math.min(t1, Math.max(t0, t));
  out[i + 1] = v;
  return out;
}

// 2D 아핀 변환 도우미(본 월드 행렬 a b c d x y: wx = a·lx + b·ly + x, wy = c·lx + d·ly + y).
import type { WorldBone } from '../core/types';

export const IDENTITY: WorldBone = { a: 1, b: 0, c: 0, d: 1, x: 0, y: 0 };

export function apply(m: WorldBone, x: number, y: number): { x: number; y: number } {
  return { x: m.a * x + m.b * y + m.x, y: m.c * x + m.d * y + m.y };
}

export function det(m: WorldBone): number {
  return m.a * m.d - m.b * m.c;
}

export function invert(m: WorldBone): WorldBone {
  const dt = det(m) || 1e-12;
  const a = m.d / dt;
  const b = -m.b / dt;
  const c = -m.c / dt;
  const d = m.a / dt;
  return { a, b, c, d, x: -(a * m.x + b * m.y), y: -(c * m.x + d * m.y) };
}

/** 선형 부분만 적용(방향 벡터) */
export function applyLinear(m: WorldBone, x: number, y: number): { x: number; y: number } {
  return { x: m.a * x + m.b * y, y: m.c * x + m.d * y };
}

export function multiply(p: WorldBone, q: WorldBone): WorldBone {
  return {
    a: p.a * q.a + p.b * q.c,
    b: p.a * q.b + p.b * q.d,
    c: p.c * q.a + p.d * q.c,
    d: p.c * q.b + p.d * q.d,
    x: p.a * q.x + p.b * q.y + p.x,
    y: p.c * q.x + p.d * q.y + p.y,
  };
}

const DEG = 180 / Math.PI;

/** 로컬 행렬 → 위치·회전·크기·기울기(shearX = 0으로 본다) */
export function decompose(m: WorldBone): { x: number; y: number; rotation: number; scaleX: number; scaleY: number; shearX: number; shearY: number } {
  const rotation = Math.atan2(m.c, m.a) * DEG;
  const scaleX = Math.hypot(m.a, m.c);
  const scaleY = Math.hypot(m.b, m.d);
  let shearY = Math.atan2(m.d, m.b) * DEG - 90 - rotation;
  shearY = normalizeDeg(shearY);
  return { x: m.x, y: m.y, rotation, scaleX, scaleY, shearX: 0, shearY };
}

export function normalizeDeg(a: number): number {
  let r = a % 360;
  if (r > 180) r -= 360;
  if (r <= -180) r += 360;
  return r;
}

export function angleDeg(cx: number, cy: number, x: number, y: number): number {
  return Math.atan2(y - cy, x - cx) * DEG;
}

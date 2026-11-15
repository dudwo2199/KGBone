// 변환 축(로컬/부모/월드): 드래그 계산에 쓰는 좌표 틀.
import type { WorldBone } from '../core/types';
import type { Axis } from '../state/ui';

export interface Frame {
  /** 틀의 x축·y축 단위 벡터(월드) */
  ux: number;
  uy: number;
  vx: number;
  vy: number;
}

function unit(x: number, y: number): [number, number] {
  const l = Math.hypot(x, y) || 1;
  return [x / l, y / l];
}

/** 축 종류에 맞는 틀: 월드 = 화면 축, 부모 = 부모 본 축, 로컬 = 본 자신의 축 */
export function axisFrame(axis: Axis, bone: WorldBone, parent: WorldBone): Frame {
  const m = axis === 'world' ? null : axis === 'parent' ? parent : bone;
  if (!m) return { ux: 1, uy: 0, vx: 0, vy: 1 };
  const [ux, uy] = unit(m.a, m.c);
  const [vx, vy] = unit(m.b, m.d);
  return { ux, uy, vx, vy };
}

/** Shift 이동: 틀의 두 축 중 끌기 방향에 가까운 축으로만 */
export function lockToFrame(f: Frame, dx: number, dy: number): { dx: number; dy: number } {
  const a = dx * f.ux + dy * f.uy;
  const b = dx * f.vx + dy * f.vy;
  return Math.abs(a) >= Math.abs(b) ? { dx: f.ux * a, dy: f.uy * a } : { dx: f.vx * b, dy: f.vy * b };
}

/** 틀 좌표로 바꾼 점 */
export function toFrame(f: Frame, ox: number, oy: number, x: number, y: number): { x: number; y: number } {
  const dx = x - ox;
  const dy = y - oy;
  return { x: dx * f.ux + dy * f.uy, y: dx * f.vx + dy * f.vy };
}

/**
 * 크기 끌기: 틀 축 방향의 비율(rx, ry)을 본의 크기 X/Y에 나눠 준다.
 * 본 x축이 틀 x축에 더 가까우면 X←rx, Y←ry, 아니면 서로 바꾼다.
 */
export function scaleRatios(f: Frame, bone: WorldBone, start: { x: number; y: number }, cur: { x: number; y: number }): { sx: number; sy: number } {
  const rx = Math.abs(start.x) > 1e-3 ? cur.x / start.x : 1;
  const ry = Math.abs(start.y) > 1e-3 ? cur.y / start.y : 1;
  const [bx, by] = unit(bone.a, bone.c);
  const alongX = Math.abs(bx * f.ux + by * f.uy);
  const alongY = Math.abs(bx * f.vx + by * f.vy);
  return alongX >= alongY ? { sx: rx, sy: ry } : { sx: ry, sy: rx };
}

/** Shift 회전 15° 맞춤: 월드면 결과 월드 각도, 부모면 로컬 값, 로컬이면 바뀐 양을 맞춘다 */
export function snapRotation(axis: Axis, startLocal: number, delta: number, parentWorldAngle: number, flip: number): number {
  const snap = (v: number) => Math.round(v / 15) * 15;
  if (axis === 'local') return startLocal + snap(delta);
  if (axis === 'parent') return snap(startLocal + delta);
  // 월드 각도 = 부모 월드 각도 + 로컬×flip
  const world = parentWorldAngle + (startLocal + delta) * flip;
  return (snap(world) - parentWorldAngle) * flip;
}

// 메시 정점 편집 계산: 로컬 좌표 얻기, 정점 끌기·추가(가까운 외곽선 변에 끼우기)·삭제.
import type { WorldBone } from '../core/types';
import { apply, invert } from './affine';

export interface MeshShape {
  /** 슬롯 본 기준 로컬 x, y */
  vertices: number[];
  uvs: number[];
  /** 앞쪽 외곽선 정점 수 */
  hull: number;
}

/** 월드 정점 위치 → 슬롯 본 로컬 좌표 */
export function worldToLocalVertices(world: readonly number[], slotBone: WorldBone): number[] {
  const inv = invert(slotBone);
  const out: number[] = [];
  for (let i = 0; i + 1 < world.length; i += 2) {
    const p = apply(inv, world[i], world[i + 1]);
    out.push(p.x, p.y);
  }
  return out;
}

/** (x, y)에서 tol 안의 가장 가까운 정점 번호(없으면 -1) */
export function hitVertex(world: readonly number[], x: number, y: number, tol: number): number {
  let best = -1;
  let bestD = tol;
  for (let i = 0; i + 1 < world.length; i += 2) {
    const d = Math.hypot(world[i] - x, world[i + 1] - y);
    if (d <= bestD) {
      bestD = d;
      best = i / 2;
    }
  }
  return best;
}

/** 고른 정점들을 로컬 (dx, dy)만큼 옮긴 정점 배열 */
export function moveVertices(vertices: readonly number[], indices: readonly number[], dx: number, dy: number): number[] {
  const out = vertices.slice();
  for (const i of indices) {
    out[i * 2] += dx;
    out[i * 2 + 1] += dy;
  }
  return out;
}

function hullCount(m: MeshShape): number {
  const n = m.vertices.length / 2;
  return m.hull >= 3 && m.hull <= n ? m.hull : n;
}

/**
 * 로컬 점 (x, y)를 외곽선의 가장 가까운 변 사이에 새 정점으로 끼운다.
 * uv는 그 변 위 같은 비율로 보간한다. 새 정점 번호도 돌려준다.
 */
export function insertVertex(m: MeshShape, x: number, y: number): { shape: MeshShape; index: number } {
  const h = hullCount(m);
  const v = m.vertices;
  let bestEdge = h - 1;
  let bestD = Infinity;
  let bestT = 0;
  for (let i = 0; i < h; i++) {
    const j = (i + 1) % h;
    const ax = v[i * 2];
    const ay = v[i * 2 + 1];
    const bx = v[j * 2];
    const by = v[j * 2 + 1];
    const ex = bx - ax;
    const ey = by - ay;
    const l2 = ex * ex + ey * ey;
    const t = l2 > 0 ? Math.max(0, Math.min(1, ((x - ax) * ex + (y - ay) * ey) / l2)) : 0;
    const d = Math.hypot(x - (ax + ex * t), y - (ay + ey * t));
    if (d < bestD) {
      bestD = d;
      bestEdge = i;
      bestT = t;
    }
  }
  const i = bestEdge;
  const j = (i + 1) % h;
  const u = m.uvs;
  const nu = u[i * 2] + (u[j * 2] - u[i * 2]) * bestT;
  const nv = u[i * 2 + 1] + (u[j * 2 + 1] - u[i * 2 + 1]) * bestT;
  // 변 (i, i+1) 사이 = 위치 i+1. 마지막 변(h-1 → 0)이면 외곽선 끝(h)
  const at = j === 0 ? h : i + 1;
  const vertices = [...v.slice(0, at * 2), x, y, ...v.slice(at * 2)];
  const uvs = [...u.slice(0, at * 2), nu, nv, ...u.slice(at * 2)];
  return { shape: { vertices, uvs, hull: h + 1 }, index: at };
}

/** 정점 삭제. 외곽선 정점이 3개보다 적어지면 null */
export function removeVertices(m: MeshShape, indices: readonly number[]): MeshShape | null {
  const del = new Set(indices);
  const h = hullCount(m);
  let hull = h;
  for (const i of del) if (i < h) hull--;
  if (hull < 3) return null;
  const vertices: number[] = [];
  const uvs: number[] = [];
  for (let i = 0; i < m.vertices.length / 2; i++) {
    if (del.has(i)) continue;
    vertices.push(m.vertices[i * 2], m.vertices[i * 2 + 1]);
    uvs.push(m.uvs[i * 2], m.uvs[i * 2 + 1]);
  }
  return { vertices, uvs, hull };
}

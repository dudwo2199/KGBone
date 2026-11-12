// 뷰포트 클릭 선택: 본(월드 변환으로 선분·원 거리), 그다음 첨부물 삼각형.
import type { Bone, PoseOutput } from '../core/types';

function segDist(px: number, py: number, x0: number, y0: number, x1: number, y1: number): number {
  const dx = x1 - x0;
  const dy = y1 - y0;
  const l2 = dx * dx + dy * dy;
  let t = l2 > 0 ? ((px - x0) * dx + (py - y0) * dy) / l2 : 0;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(px - (x0 + dx * t), py - (y0 + dy * t));
}

/** 월드 점 (x, y)에 가장 가까운 본 이름. tol은 월드 단위 허용 거리 */
export function hitBone(pose: PoseOutput, bones: Bone[], x: number, y: number, tol: number): string | null {
  let best: string | null = null;
  let bestD = tol;
  pose.bones.forEach((m, i) => {
    const b = bones[i];
    if (!b || b.visible === false || pose.active[i] === false) return;
    const d = b.length > 0 ? segDist(x, y, m.x, m.y, m.x + m.a * b.length, m.y + m.c * b.length) : Math.hypot(x - m.x, y - m.y) * 0.8;
    if (d <= bestD) {
      bestD = d;
      best = b.name;
    }
  });
  return best;
}

function inTri(px: number, py: number, ax: number, ay: number, bx: number, by: number, cx: number, cy: number): boolean {
  const s1 = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
  const s2 = (cx - bx) * (py - by) - (cy - by) * (px - bx);
  const s3 = (ax - cx) * (py - cy) - (ay - cy) * (px - cx);
  return (s1 >= 0 && s2 >= 0 && s3 >= 0) || (s1 <= 0 && s2 <= 0 && s3 <= 0);
}

/** 위에 그려진 것부터 찾은 슬롯 이름 */
export function hitSlot(pose: PoseOutput, x: number, y: number): { slot: string; attachment: string } | null {
  for (let k = pose.draw.length - 1; k >= 0; k--) {
    const it = pose.draw[k];
    const p = it.positions;
    for (let i = 0; i + 2 < it.indices.length; i += 3) {
      const a = it.indices[i] * 2;
      const b = it.indices[i + 1] * 2;
      const c = it.indices[i + 2] * 2;
      if (inTri(x, y, p[a], p[a + 1], p[b], p[b + 1], p[c], p[c + 1])) return { slot: it.slot, attachment: it.attachment };
    }
  }
  return null;
}

/** 포즈 전체 경계 상자 */
export function poseBounds(pose: PoseOutput, bones: Bone[]): [number, number, number, number] {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const add = (x: number, y: number) => {
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  };
  pose.bones.forEach((m, i) => {
    add(m.x, m.y);
    const len = bones[i]?.length ?? 0;
    if (len > 0) add(m.x + m.a * len, m.y + m.c * len);
  });
  for (const d of pose.draw) for (let i = 0; i + 1 < d.positions.length; i += 2) add(d.positions[i], d.positions[i + 1]);
  return [minX, minY, maxX, maxY];
}

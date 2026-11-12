// 뷰포트 카메라: 월드(y 위로) ↔ 화면(CSS 픽셀, y 아래로).
export interface View {
  /** 화면 가운데의 월드 좌표 */
  x: number;
  y: number;
  zoom: number;
}
export interface Size {
  w: number;
  h: number;
}

export function worldToScreen(v: View, s: Size, wx: number, wy: number): { x: number; y: number } {
  return { x: (wx - v.x) * v.zoom + s.w / 2, y: s.h / 2 - (wy - v.y) * v.zoom };
}

export function screenToWorld(v: View, s: Size, sx: number, sy: number): { x: number; y: number } {
  return { x: (sx - s.w / 2) / v.zoom + v.x, y: (s.h / 2 - sy) / v.zoom + v.y };
}

/** 월드 → 클립 공간 변환 [sx, sy, tx, ty]: clip = world × s + t */
export function clipTransform(v: View, s: Size): [number, number, number, number] {
  const sx = (2 * v.zoom) / Math.max(1, s.w);
  const sy = (2 * v.zoom) / Math.max(1, s.h);
  return [sx, sy, -v.x * sx, -v.y * sy];
}

/** 커서 위치를 고정한 채 확대 비율을 바꾼다 */
export function zoomAt(v: View, s: Size, sx: number, sy: number, zoom: number): View {
  const z = Math.min(64, Math.max(0.02, zoom));
  const before = screenToWorld(v, s, sx, sy);
  const nv = { ...v, zoom: z };
  const after = screenToWorld(nv, s, sx, sy);
  return { x: nv.x + before.x - after.x, y: nv.y + before.y - after.y, zoom: z };
}

/** 경계 상자가 화면에 들어오게 맞춘다 */
export function fitBounds(s: Size, minX: number, minY: number, maxX: number, maxY: number, margin = 40): View {
  if (!isFinite(minX) || maxX - minX < 1e-6 || maxY - minY < 1e-6) {
    const cx = isFinite(minX) ? (minX + maxX) / 2 : 0;
    const cy = isFinite(minY) ? (minY + maxY) / 2 : 0;
    return { x: cx, y: cy, zoom: 1 };
  }
  const zx = (s.w - 2 * margin) / (maxX - minX);
  const zy = (s.h - 2 * margin) / (maxY - minY);
  return { x: (minX + maxX) / 2, y: (minY + maxY) / 2, zoom: Math.min(64, Math.max(0.02, Math.min(zx, zy))) };
}

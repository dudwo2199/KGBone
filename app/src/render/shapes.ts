// 단색 도형 모으기(삼각형 목록). 선은 화면 픽셀 두께의 사각형으로 만든다.
export type Rgba = readonly [number, number, number, number];

export class ShapeBatch {
  private data = new Float32Array(6 * 4096);
  private n = 0;
  /** 화면 1픽셀 = 월드 몇 단위 */
  px = 1;

  reset(px: number): void {
    this.n = 0;
    this.px = px;
  }

  get count(): number {
    return this.n;
  }
  get buffer(): Float32Array {
    return this.data;
  }

  private grow(extra: number): void {
    if ((this.n + extra) * 6 <= this.data.length) return;
    const next = new Float32Array(Math.max(this.data.length * 2, (this.n + extra) * 6));
    next.set(this.data);
    this.data = next;
  }

  private v(x: number, y: number, c: Rgba): void {
    const i = this.n * 6;
    const d = this.data;
    d[i] = x;
    d[i + 1] = y;
    d[i + 2] = c[0];
    d[i + 3] = c[1];
    d[i + 4] = c[2];
    d[i + 5] = c[3];
    this.n++;
  }

  tri(x0: number, y0: number, x1: number, y1: number, x2: number, y2: number, c: Rgba): void {
    this.grow(3);
    this.v(x0, y0, c);
    this.v(x1, y1, c);
    this.v(x2, y2, c);
  }

  quad(x0: number, y0: number, x1: number, y1: number, x2: number, y2: number, x3: number, y3: number, c: Rgba): void {
    this.tri(x0, y0, x1, y1, x2, y2, c);
    this.tri(x0, y0, x2, y2, x3, y3, c);
  }

  /** 두께 widthPx(화면 픽셀)의 선 */
  line(x0: number, y0: number, x1: number, y1: number, c: Rgba, widthPx = 1): void {
    const dx = x1 - x0;
    const dy = y1 - y0;
    const len = Math.hypot(dx, dy);
    if (len < 1e-9) return;
    const h = (widthPx * this.px) / 2;
    const nx = (-dy / len) * h;
    const ny = (dx / len) * h;
    this.quad(x0 + nx, y0 + ny, x1 + nx, y1 + ny, x1 - nx, y1 - ny, x0 - nx, y0 - ny, c);
  }

  polyline(pts: readonly number[], closed: boolean, c: Rgba, widthPx = 1): void {
    const n = pts.length / 2;
    for (let i = 0; i + 1 < n; i++) this.line(pts[i * 2], pts[i * 2 + 1], pts[i * 2 + 2], pts[i * 2 + 3], c, widthPx);
    if (closed && n > 2) this.line(pts[(n - 1) * 2], pts[(n - 1) * 2 + 1], pts[0], pts[1], c, widthPx);
  }

  /** 반지름 rPx(화면 픽셀) 원 채우기 */
  disc(cx: number, cy: number, rPx: number, c: Rgba, seg = 16): void {
    const r = rPx * this.px;
    for (let i = 0; i < seg; i++) {
      const a0 = (i / seg) * Math.PI * 2;
      const a1 = ((i + 1) / seg) * Math.PI * 2;
      this.tri(cx, cy, cx + Math.cos(a0) * r, cy + Math.sin(a0) * r, cx + Math.cos(a1) * r, cy + Math.sin(a1) * r, c);
    }
  }

  /** 반지름 r(월드 단위, worldUnits=true) 또는 화면 픽셀 원 테두리 */
  circle(cx: number, cy: number, r: number, c: Rgba, widthPx = 1, worldUnits = false, seg = 32): void {
    const rr = worldUnits ? r : r * this.px;
    const pts: number[] = [];
    for (let i = 0; i < seg; i++) {
      const a = (i / seg) * Math.PI * 2;
      pts.push(cx + Math.cos(a) * rr, cy + Math.sin(a) * rr);
    }
    this.polyline(pts, true, c, widthPx);
  }

  /** 삼각형 인덱스로 채우기 */
  fillIndexed(pts: readonly number[], indices: readonly number[], c: Rgba): void {
    for (let i = 0; i + 2 < indices.length; i += 3) {
      const a = indices[i] * 2;
      const b = indices[i + 1] * 2;
      const d = indices[i + 2] * 2;
      this.tri(pts[a], pts[a + 1], pts[b], pts[b + 1], pts[d], pts[d + 1], c);
    }
  }

  /** 삼각형 인덱스의 모서리 선 */
  wireIndexed(pts: readonly number[], indices: readonly number[], c: Rgba, widthPx = 1): void {
    const seen = new Set<string>();
    for (let i = 0; i + 2 < indices.length; i += 3) {
      const t = [indices[i], indices[i + 1], indices[i + 2]];
      for (let e = 0; e < 3; e++) {
        const p = t[e];
        const q = t[(e + 1) % 3];
        const key = p < q ? `${p},${q}` : `${q},${p}`;
        if (seen.has(key)) continue;
        seen.add(key);
        this.line(pts[p * 2], pts[p * 2 + 1], pts[q * 2], pts[q * 2 + 1], c, widthPx);
      }
    }
  }
}

/** "RRGGBBAA" / "RRGGBB" → 0~1 */
export function hexToRgba(hex: string | null | undefined, fallback: Rgba = [1, 1, 1, 1]): [number, number, number, number] {
  if (!hex || !/^[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(hex)) return [...fallback] as [number, number, number, number];
  const n = (i: number) => parseInt(hex.slice(i, i + 2), 16) / 255;
  return [n(0), n(2), n(4), hex.length >= 8 ? n(6) : 1];
}

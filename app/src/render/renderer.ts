// WebGL2 뷰포트 렌더러: 격자·원점 축 → 오니언 스킨 → 첨부물 → 선 첨부물 → 본 → 선택 강조·도구 표시.
import type { Bone, PoseOutput } from '../core/types';
import { clipTransform, type Size, type View } from './camera';
import { SolidProgram, TexturedProgram } from './gl';
import { attachmentPainter, overlayPainter, type PaintContext } from './painters';
import { hexToRgba, ShapeBatch, type Rgba } from './shapes';
import { TextureCache } from './textures';

export interface Gizmo {
  /** 생성 도구 드래그 선(월드) */
  line?: [number, number, number, number];
  /** 가중치 붓(월드 중심, 화면 반지름) */
  brush?: { x: number; y: number; r: number };
  /** 회전·크기·기울기 도구 표시 원(본 원점) */
  ring?: { x: number; y: number };
  /** 변환 축 표시(본 원점, 축 틀 단위 벡터) */
  axes?: { x: number; y: number; ux: number; uy: number; vx: number; vy: number };
  /** 메시 정점 편집: 정점 월드 좌표와 고른 정점 번호 */
  vertices?: { points: number[]; selected: number[]; hull: number };
}

export interface FrameInput {
  view: View;
  size: Size;
  dpr: number;
  pose: PoseOutput;
  bones: Bone[];
  onion: { pose: PoseOutput; tint: Rgba }[];
  showBones: boolean;
  showImages: boolean;
  selectedBones: Set<string>;
  selectedSlots: Set<string>;
  hoverBone: string | null;
  imageUrl(name: string): string | null;
  gizmo: Gizmo;
}

const BG: Rgba = [0.16, 0.16, 0.17, 1];
const GRID: Rgba = [1, 1, 1, 0.05];
const GRID_MAJOR: Rgba = [1, 1, 1, 0.1];
const AXIS_X: Rgba = [0.9, 0.3, 0.3, 0.8];
const AXIS_Y: Rgba = [0.3, 0.85, 0.35, 0.8];
const SELECT: Rgba = [0.25, 0.85, 1, 1];
const HOVER: Rgba = [1, 1, 1, 0.9];
const GIZMO: Rgba = [1, 0.75, 0.2, 0.9];

export class Renderer {
  private textured: TexturedProgram;
  private solid: SolidProgram;
  private shapes = new ShapeBatch();
  readonly textures: TextureCache;

  private constructor(
    private gl: WebGL2RenderingContext,
    onTexture: () => void,
  ) {
    this.textured = new TexturedProgram(gl);
    this.solid = new SolidProgram(gl);
    this.textures = new TextureCache(gl, onTexture);
  }

  /** WebGL2가 없으면 null */
  static create(canvas: HTMLCanvasElement, onTexture: () => void): Renderer | null {
    const gl = canvas.getContext('webgl2', { alpha: false, antialias: true, premultipliedAlpha: true, preserveDrawingBuffer: true }); // AI 연결 화면 찍기(view_screenshot)
    if (!gl) return null;
    try {
      return new Renderer(gl, onTexture);
    } catch {
      return null;
    }
  }

  draw(f: FrameInput): void {
    const gl = this.gl;
    const cw = Math.max(1, Math.round(f.size.w * f.dpr));
    const ch = Math.max(1, Math.round(f.size.h * f.dpr));
    if (gl.canvas.width !== cw || gl.canvas.height !== ch) {
      gl.canvas.width = cw;
      gl.canvas.height = ch;
    }
    gl.viewport(0, 0, cw, ch);
    gl.clearColor(BG[0], BG[1], BG[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    const xf = clipTransform(f.view, f.size);
    const px = 1 / f.view.zoom;

    // 1) 격자·원점 축
    this.shapes.reset(px);
    this.grid(f);
    this.flush(xf);

    // 2) 오니언 스킨 + 3) 첨부물
    if (f.showImages) {
      const ctx: PaintContext = { gl, textured: this.textured, textures: this.textures, shapes: this.shapes, imageUrl: f.imageUrl, fade: 1 };
      this.textured.use(xf);
      for (const o of f.onion) {
        ctx.fade = o.tint[3];
        for (const item of o.pose.draw) attachmentPainter.paint(ctx, item);
      }
      ctx.fade = 1;
      for (const item of f.pose.draw) attachmentPainter.paint(ctx, item);
      gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    }

    // 4) 선 첨부물·선택 슬롯 강조
    this.shapes.reset(px);
    for (const o of f.pose.overlays) overlayPainter(o.kind).paint(this.shapes, o, f.selectedSlots.has(o.slot));
    for (const item of f.pose.draw) if (f.selectedSlots.has(item.slot)) this.shapes.wireIndexed(item.positions, item.indices, SELECT, 1);

    // 5) 본
    if (f.showBones) this.bonesLayer(f);

    // 6) 도구 표시
    const g = f.gizmo;
    if (g.line) {
      this.shapes.line(g.line[0], g.line[1], g.line[2], g.line[3], GIZMO, 2);
      this.shapes.disc(g.line[0], g.line[1], 3, GIZMO);
    }
    if (g.brush) this.shapes.circle(g.brush.x, g.brush.y, g.brush.r, GIZMO, 1.5);
    if (g.ring) this.shapes.circle(g.ring.x, g.ring.y, 40, [GIZMO[0], GIZMO[1], GIZMO[2], 0.5], 1);
    if (g.axes) {
      const L = 36 * px;
      const a = g.axes;
      this.shapes.line(a.x, a.y, a.x + a.ux * L, a.y + a.uy * L, AXIS_X, 2);
      this.shapes.line(a.x, a.y, a.x + a.vx * L, a.y + a.vy * L, AXIS_Y, 2);
    }
    if (g.vertices) {
      const v = g.vertices;
      const n = v.points.length / 2;
      const h = v.hull >= 3 && v.hull <= n ? v.hull : n;
      const hullPts = v.points.slice(0, h * 2);
      this.shapes.polyline(hullPts, true, [1, 1, 1, 0.8], 1.5);
      const sel = new Set(v.selected);
      for (let i = 0; i < n; i++) {
        const x = v.points[i * 2];
        const y = v.points[i * 2 + 1];
        this.shapes.disc(x, y, sel.has(i) ? 5 : 4, [0, 0, 0, 0.8], 10);
        this.shapes.disc(x, y, sel.has(i) ? 3.5 : 2.5, sel.has(i) ? GIZMO : [1, 1, 1, 1], 10);
      }
    }
    this.flush(xf);
  }

  private flush(xf: [number, number, number, number]): void {
    this.gl.blendFunc(this.gl.ONE, this.gl.ONE_MINUS_SRC_ALPHA);
    this.solid.draw(xf, this.shapes.buffer, this.shapes.count);
    this.shapes.reset(this.shapes.px);
  }

  private grid(f: FrameInput): void {
    const z = f.view.zoom;
    let step = 10;
    while (step * z < 16) step *= 5;
    while (step * z > 120) step /= 5;
    const hw = f.size.w / 2 / z;
    const hh = f.size.h / 2 / z;
    const x0 = f.view.x - hw;
    const x1 = f.view.x + hw;
    const y0 = f.view.y - hh;
    const y1 = f.view.y + hh;
    const s = this.shapes;
    for (let i = Math.ceil(x0 / step); i * step <= x1; i++) s.line(i * step, y0, i * step, y1, i % 5 === 0 ? GRID_MAJOR : GRID, 1);
    for (let j = Math.ceil(y0 / step); j * step <= y1; j++) s.line(x0, j * step, x1, j * step, j % 5 === 0 ? GRID_MAJOR : GRID, 1);
    s.line(x0, 0, x1, 0, AXIS_X, 1);
    s.line(0, y0, 0, y1, AXIS_Y, 1);
  }

  private bonesLayer(f: FrameInput): void {
    const s = this.shapes;
    f.pose.bones.forEach((m, i) => {
      const bone = f.bones[i];
      if (!bone || bone.visible === false) return;
      if (f.pose.active[i] === false) return;
      const selected = f.selectedBones.has(bone.name);
      const hover = f.hoverBone === bone.name;
      const base = hexToRgba(bone.color, [0.6, 0.6, 0.6, 1]);
      const fill: Rgba = selected ? [SELECT[0], SELECT[1], SELECT[2], 0.55] : [base[0], base[1], base[2], 0.5];
      const edge: Rgba = selected ? SELECT : hover ? HOVER : [base[0] * 0.6, base[1] * 0.6, base[2] * 0.6, 0.95];
      const len = bone.length;
      if (len > 0) {
        const tx = m.x + m.a * len;
        const ty = m.y + m.c * len;
        const dx = tx - m.x;
        const dy = ty - m.y;
        const L = Math.hypot(dx, dy) || 1;
        const ux = dx / L;
        const uy = dy / L;
        const w = Math.min(L * 0.12, 9 * s.px);
        const bx = m.x + ux * L * 0.18;
        const by = m.y + uy * L * 0.18;
        const p1x = bx - uy * w;
        const p1y = by + ux * w;
        const p2x = bx + uy * w;
        const p2y = by - ux * w;
        s.tri(m.x, m.y, p1x, p1y, tx, ty, fill);
        s.tri(m.x, m.y, tx, ty, p2x, p2y, fill);
        s.polyline([m.x, m.y, p1x, p1y, tx, ty, p2x, p2y], true, edge, selected || hover ? 2 : 1);
        s.disc(m.x, m.y, 2.5, edge, 10);
      } else {
        s.disc(m.x, m.y, 5, fill, 16);
        s.circle(m.x, m.y, 5, edge, selected || hover ? 2 : 1.25);
        // 방향 표시
        s.line(m.x, m.y, m.x + m.a * 10 * s.px, m.y + m.c * 10 * s.px, edge, 1);
      }
    });
  }
}

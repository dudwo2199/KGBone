// 첨부물 종류별 그리기. 이미지·메시는 삼각형(DrawItem), 나머지는 겹쳐 그리는 선(Overlay).
import type { DrawItem, Overlay } from '../core/types';
import { applyBlend } from './blend';
import type { TexturedProgram } from './gl';
import type { Rgba, ShapeBatch } from './shapes';
import type { TextureCache } from './textures';

export interface PaintContext {
  gl: WebGL2RenderingContext;
  textured: TexturedProgram;
  textures: TextureCache;
  shapes: ShapeBatch;
  /** 이미지 이름 → URL */
  imageUrl(name: string): string | null;
  /** 오니언 스킨 등 반투명 배율 */
  fade: number;
}

/** 삼각형 첨부물(이미지/메시) 그리기 */
export interface AttachmentPainter {
  paint(ctx: PaintContext, item: DrawItem): void;
}

/** 선 첨부물(바운딩 박스·클리핑·패스·포인트·메시 외곽선) 그리기 */
export interface OverlayPainter {
  paint(shapes: ShapeBatch, o: Overlay, highlight: boolean): void;
}

class TexturedPainter implements AttachmentPainter {
  paint(ctx: PaintContext, item: DrawItem): void {
    let tex = ctx.textures.get(item.image, ctx.imageUrl(item.image));
    let light: readonly number[] = item.color;
    if (!tex) {
      // 이미지를 못 찾았거나 아직 불러오는 중: 흐린 자리 표시
      tex = ctx.textures.whiteTexture();
      light = [0.6, 0.6, 0.6, 0.25 * item.color[3]];
    }
    applyBlend(ctx.gl, item.blend);
    ctx.textured.draw(tex, item.positions, item.uvs, item.indices, light, item.dark, ctx.fade);
  }
}

const col = (o: Overlay, a = 1): Rgba => [o.color[0], o.color[1], o.color[2], o.color[3] * a];

class PolygonOverlay implements OverlayPainter {
  paint(s: ShapeBatch, o: Overlay, hi: boolean): void {
    s.polyline(o.points, true, col(o), hi ? 2.5 : 1.25);
  }
}

class PathOverlay implements OverlayPainter {
  paint(s: ShapeBatch, o: Overlay, hi: boolean): void {
    s.polyline(o.points, o.closed, col(o), hi ? 2.5 : 1.5);
  }
}

class PointOverlay implements OverlayPainter {
  paint(s: ShapeBatch, o: Overlay, hi: boolean): void {
    const [x, y] = o.points;
    if (x === undefined) return;
    s.disc(x, y, hi ? 5 : 4, col(o));
    if (o.points.length >= 4) s.line(x, y, o.points[2], o.points[3], col(o), 1.5);
  }
}

class MeshOverlay implements OverlayPainter {
  paint(s: ShapeBatch, o: Overlay, hi: boolean): void {
    const c = col(o, hi ? 0.9 : 0.45);
    if (o.indices.length) s.wireIndexed(o.points, o.indices, c, 1);
    else s.polyline(o.points, true, c, 1);
    if (hi) for (let i = 0; i + 1 < o.points.length; i += 2) s.disc(o.points[i], o.points[i + 1], 2.5, c, 8);
  }
}

const overlayPainters: Record<Overlay['kind'], OverlayPainter> = {
  boundingbox: new PolygonOverlay(),
  clipping: new PolygonOverlay(),
  path: new PathOverlay(),
  point: new PointOverlay(),
  mesh: new MeshOverlay(),
};

export const attachmentPainter: AttachmentPainter = new TexturedPainter();

export function overlayPainter(kind: Overlay['kind']): OverlayPainter {
  return overlayPainters[kind] ?? overlayPainters.boundingbox;
}

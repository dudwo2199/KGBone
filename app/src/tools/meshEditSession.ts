// 메시 정점 편집 모드: 고른 메시 첨부물의 정점 끌기·추가·삭제를 핵심 명령(mesh.setVertices / mesh.setTriangles)으로 보낸다.
import type { MeshAttachment, WorldBone } from '../core/types';
import type { Services } from '../services';
import type { Sel } from '../state/selection';
import { IDENTITY } from './affine';
import type { DragTool, Point } from './dragTools';
import { insertVertex, moveVertices, removeVertices, worldToLocalVertices, type MeshShape } from './meshEdit';
import { applyLinear, invert } from './affine';

export interface MeshEditTarget {
  sel: Sel;
  mesh: MeshAttachment;
  /** 지금 보이는 정점 월드 좌표 */
  world: number[];
  slotBone: WorldBone;
  /** 명령 공통 인자 */
  base: Record<string, unknown>;
}

/** 정점 편집 대상: 선택의 마지막이 메시 첨부물이고 지금 슬롯에 보이는 경우 */
export function meshEditTarget(s: Services): MeshEditTarget | null {
  const sel = s.ui.get().selection.at(-1);
  if (!sel || sel.kind !== 'attachment') return null;
  const sk = s.doc.get().project.skeleton;
  const att = sk.skins.find((k) => k.name === sel.skin)?.attachments[sel.slot ?? '']?.[sel.name];
  if (!att || att.type !== 'mesh') return null;
  const pose = s.pose.pose();
  const ov = pose.overlays.find((o) => o.kind === 'mesh' && o.slot === sel.slot && o.attachment === sel.name);
  if (!ov) return null;
  const slot = sk.slots.find((x) => x.name === sel.slot);
  const bi = sk.bones.findIndex((b) => b.name === slot?.bone);
  const slotBone = pose.bones[bi] ?? IDENTITY;
  const base: Record<string, unknown> = { slot: sel.slot, name: sel.name, ...(sel.skin && sel.skin !== 'default' ? { skin: sel.skin } : {}) };
  return { sel, mesh: att, world: ov.points, slotBone, base };
}

function shapeOf(t: MeshEditTarget): MeshShape {
  return { vertices: worldToLocalVertices(t.world, t.slotBone), uvs: t.mesh.uvs, hull: t.mesh.hull };
}

function canChangeCount(s: Services, t: MeshEditTarget): boolean {
  if (t.mesh.weighted) {
    s.ui.notify('warn', '가중치 메시는 정점 수를 바꿀 수 없습니다. 먼저 바인딩을 해제하세요');
    return false;
  }
  if (s.doc.get().project.editor.mode === 'animate') {
    s.ui.notify('warn', '애니메이트 모드에서는 정점을 추가·삭제할 수 없습니다(끌기만 됩니다)');
    return false;
  }
  return true;
}

/** 빈 곳 클릭: 가장 가까운 외곽선 변에 정점을 끼우고 삼각형을 다시 나눈다 */
export function addVertexAt(s: Services, t: MeshEditTarget, p: Point): void {
  if (!canChangeCount(s, t)) return;
  const local = applyLinear(invert(t.slotBone), p.x - t.slotBone.x, p.y - t.slotBone.y);
  const { shape, index } = insertVertex(shapeOf(t), local.x, local.y);
  s.doc.group('정점 추가', () => {
    if (s.doc.run('mesh.setVertices', { ...t.base, vertices: shape.vertices, uvs: shape.uvs, hull: shape.hull }).ok) s.doc.run('mesh.setTriangles', t.base);
  });
  s.ui.set({ selectedVertices: [index] });
}

/** 고른 정점 삭제 */
export function deleteSelectedVertices(s: Services): boolean {
  const t = meshEditTarget(s);
  const idx = s.ui.get().selectedVertices;
  if (!t || !idx.length) return false;
  if (!canChangeCount(s, t)) return true;
  const shape = removeVertices(shapeOf(t), idx);
  if (!shape) {
    s.ui.notify('warn', '외곽선 정점은 3개 이상 남아야 합니다');
    return true;
  }
  s.doc.group('정점 삭제', () => {
    if (s.doc.run('mesh.setVertices', { ...t.base, vertices: shape.vertices, uvs: shape.uvs, hull: shape.hull }).ok) s.doc.run('mesh.setTriangles', t.base);
  });
  s.ui.set({ selectedVertices: [] });
  return true;
}

/** 고른 정점 끌기: 끌기 한 번 = 되돌리기 한 번 */
export class VertexDrag implements DragTool {
  private start: Point = { x: 0, y: 0 };
  private startLocal: number[];
  private started = false;

  constructor(
    private s: Services,
    private t: MeshEditTarget,
    private indices: number[],
  ) {
    this.startLocal = worldToLocalVertices(t.world, t.slotBone);
  }

  begin(p: Point): void {
    this.start = p;
  }
  move(p: Point): void {
    if (!this.started) {
      this.started = true;
      this.s.doc.beginGroup('메시 정점 편집');
    }
    const d = applyLinear(invert(this.t.slotBone), p.x - this.start.x, p.y - this.start.y);
    const vertices = moveVertices(this.startLocal, this.indices, d.x, d.y).map((v) => Math.round(v * 1000) / 1000);
    this.s.doc.run('mesh.setVertices', { ...this.t.base, vertices });
  }
  end(): void {
    if (this.started) this.s.doc.endGroup();
  }
  gizmo() {
    return null;
  }
}

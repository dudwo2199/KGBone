// 뷰포트 드래그 도구: 본 회전·이동·크기·기울기·포즈, 본 생성, 가중치 붓. 드래그 한 번 = 되돌리기 한 번.
import type { Bone, PoseOutput, WorldBone } from '../core/types';
import type { Services } from '../services';
import { namesOf } from '../state/selection';
import type { ToolId } from '../state/ui';
import { angleDeg, apply, applyLinear, decompose, det, IDENTITY, invert, multiply, normalizeDeg } from './affine';
import { axisFrame, lockToFrame, scaleRatios, snapRotation, toFrame } from './axis';

export interface Point {
  x: number;
  y: number;
}

export interface DragTool {
  begin(p: Point, e: { shiftKey: boolean; altKey: boolean }): void;
  move(p: Point, e: { shiftKey: boolean; altKey: boolean }): void;
  end(): void;
  /** 그릴 도구 표시 */
  gizmo(): { line?: [number, number, number, number]; brush?: { x: number; y: number; r: number } } | null;
}

interface Local {
  x: number;
  y: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
  shearX: number;
  shearY: number;
}

function boneIndex(bones: Bone[], name: string): number {
  return bones.findIndex((b) => b.name === name);
}

/** 화면에 보이는 로컬 값(셋업 모드: 셋업 값, 애니메이트: 포즈 결과) */
export function currentLocal(s: Services, pose: PoseOutput, i: number): Local {
  const p = s.doc.get().project;
  const b = p.skeleton.bones[i];
  if (p.editor.mode === 'animate' && pose.locals[i]) return { ...pose.locals[i] };
  return { x: b.x, y: b.y, rotation: b.rotation, scaleX: b.scaleX, scaleY: b.scaleY, shearX: b.shearX, shearY: b.shearY };
}

function parentWorld(bones: Bone[], pose: PoseOutput, i: number): WorldBone {
  const parent = bones[i]?.parent;
  if (!parent) return IDENTITY;
  const pi = boneIndex(bones, parent);
  return pose.bones[pi] ?? IDENTITY;
}

const round = (v: number) => Math.round(v * 1000) / 1000;

class BoneDrag implements DragTool {
  private start: Point = { x: 0, y: 0 };
  private targets: { name: string; i: number; local: Local; world: WorldBone; parent: WorldBone }[] = [];
  private pose: PoseOutput;
  private started = false;
  private children: { name: string; world: WorldBone; i: number }[] = [];

  constructor(
    private s: Services,
    private tool: ToolId,
  ) {
    this.pose = s.pose.pose();
  }

  begin(p: Point): void {
    this.start = p;
    const bones = this.s.doc.get().project.skeleton.bones;
    const names = namesOf(this.s.ui.get().selection, 'bone');
    this.targets = names
      .map((name) => boneIndex(bones, name))
      .filter((i) => i >= 0 && this.pose.bones[i])
      .map((i) => ({ name: bones[i].name, i, local: currentLocal(this.s, this.pose, i), world: this.pose.bones[i], parent: parentWorld(bones, this.pose, i) }));
    // 자식 보정: 셋업 모드에서만, 직계 자식의 월드 변환을 기억
    if (this.s.ui.get().compensate && this.s.doc.get().project.editor.mode === 'setup') {
      const set = new Set(names);
      this.children = bones
        .map((b, i) => ({ b, i }))
        .filter(({ b }) => b.parent && set.has(b.parent) && !set.has(b.name) && b.inherit === 'normal')
        .map(({ b, i }) => ({ name: b.name, world: this.pose.bones[i], i }));
    }
  }

  move(p: Point, e: { shiftKey: boolean }): void {
    if (!this.targets.length) return;
    if (!this.started) {
      this.started = true;
      this.s.doc.beginGroup(toolLabel(this.tool));
    }
    for (const t of this.targets) this.s.doc.run('bone.setTransform', { name: t.name, ...this.compute(t, p, e.shiftKey) }, { quiet: false });
    if (this.children.length) this.compensate();
  }

  private compute(t: BoneDrag['targets'][number], p: Point, shift: boolean): Partial<Local> {
    const o = { x: t.world.x, y: t.world.y };
    const flip = det(t.parent) < 0 ? -1 : 1;
    const axis = this.s.ui.get().axis;
    const frame = axisFrame(axis, t.world, t.parent);
    switch (this.tool) {
      case 'rotate':
      case 'shear': {
        const d = normalizeDeg(angleDeg(o.x, o.y, p.x, p.y) - angleDeg(o.x, o.y, this.start.x, this.start.y)) * flip;
        if (this.tool === 'shear') return { shearY: round(t.local.shearY + (shift ? Math.round(d / 15) * 15 : d)) };
        if (!shift) return { rotation: round(t.local.rotation + d) };
        const parentAngle = Math.atan2(t.parent.c, t.parent.a) * (180 / Math.PI);
        return { rotation: round(snapRotation(axis, t.local.rotation, d, parentAngle, flip)) };
      }
      case 'pose': {
        // 본이 커서를 향하게: 부모 공간에서 커서 방향 각도
        const lp = apply(invert(t.parent), p.x, p.y);
        const ang = Math.atan2(lp.y - t.local.y, lp.x - t.local.x) * (180 / Math.PI);
        return { rotation: round(ang - t.local.shearX) };
      }
      case 'translate': {
        let dx = p.x - this.start.x;
        let dy = p.y - this.start.y;
        // Shift: 고른 축(로컬/부모/월드)의 한 방향으로만
        if (shift) ({ dx, dy } = lockToFrame(frame, dx, dy));
        const d = applyLinear(invert(t.parent), dx, dy);
        return { x: round(t.local.x + d.x), y: round(t.local.y + d.y) };
      }
      case 'scale': {
        // 고른 축 틀에서 본 원점 기준 비율
        const a = toFrame(frame, o.x, o.y, this.start.x, this.start.y);
        const b = toFrame(frame, o.x, o.y, p.x, p.y);
        if (shift) {
          const r = Math.hypot(b.x, b.y) / (Math.hypot(a.x, a.y) || 1);
          return { scaleX: round(t.local.scaleX * r), scaleY: round(t.local.scaleY * r) };
        }
        const { sx, sy } = scaleRatios(frame, t.world, a, b);
        return { scaleX: round(t.local.scaleX * sx), scaleY: round(t.local.scaleY * sy) };
      }
      default:
        return {};
    }
  }

  private compensate(): void {
    const bones = this.s.doc.get().project.skeleton.bones;
    const now = this.s.pose.pose();
    for (const c of this.children) {
      const pw = parentWorld(bones, now, c.i);
      const local = decompose(multiply(invert(pw), c.world));
      this.s.doc.run('bone.setTransform', {
        name: c.name,
        x: round(local.x),
        y: round(local.y),
        rotation: round(local.rotation),
        scaleX: round(local.scaleX),
        scaleY: round(local.scaleY),
        shearX: 0,
        shearY: round(local.shearY),
      });
    }
  }

  end(): void {
    if (this.started) this.s.doc.endGroup();
  }
  gizmo() {
    return null;
  }
}

function toolLabel(t: ToolId): string {
  return { pose: '포즈', create: '본 생성', weights: '가중치 페인팅', rotate: '회전', translate: '이동', scale: '크기', shear: '기울기' }[t];
}

class CreateDrag implements DragTool {
  private a: Point = { x: 0, y: 0 };
  private b: Point = { x: 0, y: 0 };
  private parent: string | null = null;
  private pose: PoseOutput;

  constructor(private s: Services) {
    this.pose = s.pose.pose();
  }

  begin(p: Point): void {
    this.a = p;
    this.b = p;
    const bones = this.s.doc.get().project.skeleton.bones;
    this.parent = namesOf(this.s.ui.get().selection, 'bone').pop() ?? bones[0]?.name ?? null;
  }
  move(p: Point): void {
    this.b = p;
  }
  end(): void {
    const bones = this.s.doc.get().project.skeleton.bones;
    const pi = this.parent ? boneIndex(bones, this.parent) : -1;
    const pw = pi >= 0 ? (this.pose.bones[pi] ?? IDENTITY) : IDENTITY;
    const inv = invert(pw);
    const lp = apply(inv, this.a.x, this.a.y);
    const ld = applyLinear(inv, this.b.x - this.a.x, this.b.y - this.a.y);
    const length = Math.hypot(ld.x, ld.y);
    const args: Record<string, unknown> = { x: round(lp.x), y: round(lp.y) };
    if (this.parent) args.parent = this.parent;
    if (length * (this.s.ui.get().view.zoom || 1) > 4) {
      args.length = round(length);
      args.rotation = round((Math.atan2(ld.y, ld.x) * 180) / Math.PI);
    }
    const before = new Set(bones.map((b) => b.name));
    const r = this.s.doc.run('bone.create', args);
    if (r.ok) {
      const created = this.s.doc.get().project.skeleton.bones.find((b) => !before.has(b.name));
      if (created) this.s.ui.select([{ kind: 'bone', name: created.name }]);
    }
  }
  gizmo() {
    return { line: [this.a.x, this.a.y, this.b.x, this.b.y] as [number, number, number, number] };
  }
}

class WeightsDrag implements DragTool {
  private p: Point = { x: 0, y: 0 };
  private started = false;
  private target: { slot: string; name: string; skin?: string; bone: string } | null = null;

  constructor(private s: Services) {}

  private radius(): number {
    return 30 / (this.s.ui.get().view.zoom || 1);
  }

  begin(p: Point, e: { altKey: boolean }): void {
    this.p = p;
    const sel = this.s.ui.get().selection;
    const bone = namesOf(sel, 'bone').pop();
    const att = [...sel].reverse().find((x) => x.kind === 'attachment');
    if (!bone || !att) {
      this.s.ui.notify('warn', '가중치 붓: 메시 첨부물과 본을 함께 고르세요(Ctrl+클릭)');
      return;
    }
    this.target = { slot: att.slot ?? '', name: att.name, skin: att.skin && att.skin !== 'default' ? att.skin : undefined, bone };
    this.paint(e.altKey);
  }
  move(p: Point, e: { altKey: boolean }): void {
    this.p = p;
    this.paint(e.altKey);
  }
  private paint(remove: boolean): void {
    if (!this.target) return;
    if (!this.started) {
      this.started = true;
      this.s.doc.beginGroup('가중치 페인팅');
    }
    this.s.doc.run('weights.paint', { ...this.target, x: this.p.x, y: this.p.y, radius: this.radius(), strength: 0.1, mode: remove ? 'remove' : 'add' });
  }
  end(): void {
    if (this.started) this.s.doc.endGroup();
  }
  gizmo() {
    return { brush: { x: this.p.x, y: this.p.y, r: 30 } };
  }
}

export function createDragTool(s: Services, tool: ToolId): DragTool {
  if (tool === 'create') return new CreateDrag(s);
  if (tool === 'weights') return new WeightsDrag(s);
  return new BoneDrag(s, tool);
}

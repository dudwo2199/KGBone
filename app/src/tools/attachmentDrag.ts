// 뷰포트에서 이미지 첨부물 끌어 옮기기(D14): 이동 도구로 그림을 잡고 끌면 첨부물의 x, y(본 로컬)가 바뀐다.
// 셋업 모드의 이미지(region)만 옮긴다. 끌기 한 번 = 되돌리기 한 번.
import type { Project, WorldBone } from '../core/types';
import type { Services } from '../services';
import { applyLinear, IDENTITY, invert } from './affine';
import { axisFrame, lockToFrame, type Frame } from './axis';
import type { DragTool, Point } from './dragTools';

export interface AttachmentTarget {
  slot: string;
  name: string;
  /** 기본 스킨이면 undefined(명령 인자에서 생략) */
  skin?: string;
  /** 끌기 시작할 때의 첨부물 위치(본 로컬) */
  x: number;
  y: number;
  /** 슬롯 본의 월드 변환 */
  bone: WorldBone;
}

/** 첨부물을 실제로 가진 스킨: 편집 중인 스킨 → 기본 스킨 → 처음 찾은 스킨 */
export function attachmentSkin(p: Project, slot: string, name: string): string | null {
  const skins = p.skeleton.skins;
  const has = (skin: string) => !!skins.find((s) => s.name === skin)?.attachments[slot]?.[name];
  const active = p.editor.activeSkin;
  if (active && has(active)) return active;
  if (has('default')) return 'default';
  return skins.find((s) => s.attachments[slot]?.[name])?.name ?? null;
}

/** 옮길 대상. 옮길 수 없으면 이유(한국어 메시지) */
export function attachmentTarget(p: Project, bones: WorldBone[], slot: string, name: string): AttachmentTarget | string {
  if (p.editor.mode !== 'setup') return '그림 옮기기는 셋업 모드에서 할 수 있습니다(Tab으로 전환)';
  const skin = attachmentSkin(p, slot, name);
  const att = skin ? p.skeleton.skins.find((s) => s.name === skin)?.attachments[slot]?.[name] : undefined;
  if (!skin || !att) return `첨부물을 찾을 수 없습니다: ${slot}/${name}`;
  if (att.type !== 'region') return '뷰포트에서 끌어 옮길 수 있는 것은 이미지 첨부물뿐입니다(메시는 정점 편집을 쓰세요)';
  const s = p.skeleton.slots.find((x) => x.name === slot);
  const bi = s ? p.skeleton.bones.findIndex((b) => b.name === s.bone) : -1;
  return { slot, name, skin: skin === 'default' ? undefined : skin, x: att.x ?? 0, y: att.y ?? 0, bone: bones[bi] ?? IDENTITY };
}

const round = (v: number) => Math.round(v * 1000) / 1000;

/** 월드 이동량(dx, dy) → 새 첨부물 위치. lock이 있으면 그 틀의 한 축으로만 */
export function movedPosition(t: AttachmentTarget, dx: number, dy: number, lock?: Frame): { x: number; y: number } {
  const d0 = lock ? lockToFrame(lock, dx, dy) : { dx, dy };
  const d = applyLinear(invert(t.bone), d0.dx, d0.dy);
  return { x: round(t.x + d.x), y: round(t.y + d.y) };
}

export class AttachmentDrag implements DragTool {
  private start: Point = { x: 0, y: 0 };
  private target: AttachmentTarget | string = '';
  private started = false;
  private warned = false;

  constructor(
    private s: Services,
    private slot: string,
    private name: string,
  ) {}

  begin(p: Point): void {
    this.start = p;
    this.target = attachmentTarget(this.s.doc.get().project, this.s.pose.pose().bones, this.slot, this.name);
  }

  move(p: Point, e: { shiftKey: boolean }): void {
    const t = this.target;
    // 클릭만 할 때는 조용히, 실제로 끌기 시작하면 옮길 수 없는 이유를 한 번 알린다
    if (typeof t === 'string') {
      if (!this.warned && t && Math.hypot(p.x - this.start.x, p.y - this.start.y) > 0) {
        this.warned = true;
        this.s.ui.notify('warn', t);
      }
      return;
    }
    if (!this.started) {
      this.started = true;
      this.s.doc.beginGroup('그림 이동');
    }
    // 첨부물의 로컬·부모 틀은 모두 슬롯 본의 틀이다
    const lock = e.shiftKey ? axisFrame(this.s.ui.get().axis, t.bone, t.bone) : undefined;
    const pos = movedPosition(t, p.x - this.start.x, p.y - this.start.y, lock);
    this.s.doc.run('attachment.set', { slot: t.slot, name: t.name, ...(t.skin ? { skin: t.skin } : {}), props: pos });
  }

  end(): void {
    if (this.started) this.s.doc.endGroup();
  }

  gizmo() {
    return null;
  }
}

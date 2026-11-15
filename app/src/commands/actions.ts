// 선택 상태를 인자로 채워 핵심 명령을 부르는 동작들(메뉴·트리 오른쪽 클릭·속성 버튼 공용).
import type { AttachmentType, ConstraintType, Project } from '../core/types';
import type { Services } from '../services';
import { namesOf, type Sel } from '../state/selection';

type NameLists = (p: Project) => string[];
const lists: Record<'bone' | 'slot' | 'constraint' | 'skin' | 'event' | 'animation', NameLists> = {
  bone: (p) => p.skeleton.bones.map((b) => b.name),
  slot: (p) => p.skeleton.slots.map((b) => b.name),
  constraint: (p) => p.skeleton.constraints.map((b) => b.name),
  skin: (p) => p.skeleton.skins.map((b) => b.name),
  event: (p) => p.skeleton.events.map((b) => b.name),
  animation: (p) => p.skeleton.animations.map((b) => b.name),
};

/** 명령을 실행하고 새로 생긴 항목 이름을 찾아 선택한다 */
function createAndSelect(s: Services, kind: keyof typeof lists, id: string, args: Record<string, unknown>): string | null {
  const before = new Set(lists[kind](s.doc.get().project));
  const r = s.doc.run<unknown>(id, args);
  if (!r.ok) return null;
  const created = lists[kind](s.doc.get().project).find((n) => !before.has(n)) ?? (r.value && typeof r.value === 'object' && typeof (r.value as { name?: unknown }).name === 'string' ? (r.value as { name: string }).name : null);
  if (created) s.ui.select([{ kind, name: created }]);
  return created;
}

export function primary(s: Services): Sel | undefined {
  const sel = s.ui.get().selection;
  return sel[sel.length - 1];
}

/** 선택에서 본 하나: 본이면 그 본, 슬롯이면 그 슬롯의 본, 없으면 root(첫 본) */
export function contextBone(s: Services): string | null {
  const p = s.doc.get().project;
  const sel = primary(s);
  if (sel?.kind === 'bone') return sel.name;
  if (sel?.kind === 'slot') return p.skeleton.slots.find((x) => x.name === sel.name)?.bone ?? null;
  if (sel?.kind === 'attachment') return p.skeleton.slots.find((x) => x.name === sel.slot)?.bone ?? null;
  return p.skeleton.bones[0]?.name ?? null;
}

export function contextSlot(s: Services): string | null {
  const sel = primary(s);
  if (sel?.kind === 'slot') return sel.name;
  if (sel?.kind === 'attachment') return sel.slot ?? null;
  return null;
}

export function createBone(s: Services, extra: Record<string, unknown> = {}): string | null {
  const parent = contextBone(s);
  return createAndSelect(s, 'bone', 'bone.create', { ...(parent ? { parent } : {}), ...extra });
}

export function createSlot(s: Services): string | null {
  const bone = contextBone(s);
  if (!bone) {
    s.ui.notify('warn', '슬롯을 붙일 본이 없습니다');
    return null;
  }
  return createAndSelect(s, 'slot', 'slot.create', { bone });
}

const ATTACHMENT_NAMES: Record<AttachmentType, string> = {
  region: '이미지',
  mesh: '메시',
  linkedmesh: '링크드 메시',
  boundingbox: '바운딩 박스',
  point: '포인트',
  clipping: '클리핑',
  path: '패스',
};
export function attachmentTypeLabel(t: AttachmentType): string {
  return ATTACHMENT_NAMES[t];
}

function uniqueName(base: string, taken: Set<string>): string {
  if (!taken.has(base)) return base;
  for (let i = 2; ; i++) if (!taken.has(`${base}${i}`)) return `${base}${i}`;
}

/** 첨부물 만들기. 이미지·메시는 이미지를 고른다 */
export async function createAttachment(s: Services, type: AttachmentType, imageName?: string): Promise<void> {
  const slot = contextSlot(s);
  if (!slot) {
    s.ui.notify('warn', '첨부물을 만들 슬롯을 먼저 고르세요');
    return;
  }
  const p = s.doc.get().project;
  const skin = p.editor.activeSkin ?? 'default';
  const skinObj = p.skeleton.skins.find((k) => k.name === skin);
  const taken = new Set(Object.keys(skinObj?.attachments[slot] ?? {}));
  let name: string;
  const props: Record<string, unknown> = {};
  if (type === 'region' || type === 'mesh' || type === 'linkedmesh') {
    const img = imageName ?? (await s.dialogs.pickImage(s.images.get().entries.map((e) => e.name)));
    if (!img) return;
    name = uniqueName(img, taken);
    if (name !== img) props.path = img;
    const e = s.images.byName(img);
    if (e && e.width > 0) {
      props.width = e.width;
      props.height = e.height;
    }
  } else {
    name = uniqueName(attachmentTypeLabel(type), taken);
  }
  const args: Record<string, unknown> = { slot, name, type, props };
  if (skin !== 'default') args.skin = skin;
  const r = s.doc.run('attachment.create', args);
  if (r.ok) s.ui.select([{ kind: 'attachment', name, slot, skin }]);
}

const CONSTRAINT_NAMES: Record<ConstraintType, string> = { ik: 'IK', transform: '트랜스폼', path: '패스', physics: '물리', slider: '슬라이더' };
export function constraintTypeLabel(t: ConstraintType): string {
  return CONSTRAINT_NAMES[t];
}

/** 제약 만들기: 선택한 본들 중 마지막을 목표(원본)로 쓴다 */
export function createConstraint(s: Services, type: ConstraintType): void {
  const sel = s.ui.get().selection;
  const bones = namesOf(sel, 'bone');
  const slots = namesOf(sel, 'slot');
  const p = s.doc.get().project;
  const args: Record<string, unknown> = { type };
  switch (type) {
    case 'ik':
      if (bones.length >= 2) {
        args.bones = bones.slice(0, -1).slice(0, 2);
        args.target = bones[bones.length - 1];
      } else if (bones.length === 1) args.bones = bones;
      break;
    case 'transform':
      if (bones.length >= 2) {
        args.bones = bones.slice(0, -1);
        args.source = bones[bones.length - 1];
      } else if (bones.length === 1) args.bones = bones;
      break;
    case 'path':
      if (bones.length) args.bones = bones;
      if (slots.length) args.target = slots[0];
      break;
    case 'physics':
      if (bones.length) args.bone = bones[0];
      break;
    case 'slider':
      if (p.editor.activeAnimation) args.animation = p.editor.activeAnimation;
      if (bones.length) args.bone = bones[0];
      break;
  }
  createAndSelect(s, 'constraint', 'constraint.create', args);
}

export function createSimple(s: Services, kind: 'skin' | 'event' | 'animation'): void {
  const name = createAndSelect(s, kind, `${kind}.create`, {});
  if (name && kind === 'animation') s.doc.run('animation.setActive', { name }, { quiet: true });
}

/** 선택 항목 삭제 */
export function deleteSelection(s: Services): void {
  const sel = s.ui.get().selection;
  if (!sel.length) return;
  s.doc.group('삭제', () => {
    for (const x of sel) {
      switch (x.kind) {
        case 'bone':
          s.doc.run('bone.delete', { name: x.name });
          break;
        case 'slot':
          s.doc.run('slot.delete', { name: x.name });
          break;
        case 'attachment':
          s.doc.run('attachment.delete', { slot: x.slot, name: x.name, ...(x.skin && x.skin !== 'default' ? { skin: x.skin } : {}) });
          break;
        case 'constraint':
        case 'skin':
        case 'event':
        case 'animation':
          s.doc.run(`${x.kind}.delete`, { name: x.name });
          break;
      }
    }
  });
  s.ui.select([]);
}

/** 이름 바꾸기 명령 인자 */
export function renameArgs(sel: Sel, newName: string): Record<string, unknown> | null {
  if (sel.kind === 'skeleton' || sel.kind === 'image') return null;
  const args: Record<string, unknown> = { kind: sel.kind, name: sel.name, newName };
  if (sel.kind === 'attachment') {
    args.slot = sel.slot;
    if (sel.skin) args.skin = sel.skin;
  }
  return args;
}

export function renameItem(s: Services, sel: Sel, newName: string): boolean {
  const name = newName.trim();
  if (!name || name === sel.name) return false;
  const args = renameArgs(sel, name);
  if (!args) return false;
  const r = s.doc.run('item.rename', args);
  if (r.ok) s.ui.select([{ ...sel, name }]);
  return r.ok;
}

/** 시점 이동(재생 중이면 재생 시점만) */
export function setTime(s: Services, t: number): void {
  const time = Math.max(0, t);
  if (s.ui.get().playing) s.ui.set({ playTime: time });
  else s.doc.run('time.set', { time });
}

/** 재생 속도·반복을 화면에 반영하고 문서(editor.playback)에도 저장한다 */
export function setPlayback(s: Services, patch: { speed?: number; loop?: boolean }): void {
  s.ui.set(patch);
  const u = s.ui.get();
  s.doc.run('editor.set', { patch: { playback: { speed: u.speed, loop: u.loop } } }, { quiet: true });
}

/** 오니언 스킨 켜기/끄기(editor.onion.enabled에도 저장) */
export function setOnion(s: Services, enabled: boolean): void {
  s.ui.set({ onion: enabled });
  s.doc.run('editor.set', { patch: { onion: { enabled } } }, { quiet: true });
}

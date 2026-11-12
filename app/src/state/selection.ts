// 선택 상태: 종류 + 이름(첨부물은 스킨·슬롯도).
import type { Project } from '../core/types';

export type SelKind = 'skeleton' | 'bone' | 'slot' | 'attachment' | 'constraint' | 'skin' | 'event' | 'animation' | 'image';

export interface Sel {
  kind: SelKind;
  name: string;
  /** 첨부물: 슬롯 이름 */
  slot?: string;
  /** 첨부물: 스킨 이름 */
  skin?: string;
}

export function sameSel(a: Sel, b: Sel): boolean {
  return a.kind === b.kind && a.name === b.name && (a.slot ?? '') === (b.slot ?? '') && (a.skin ?? '') === (b.skin ?? '');
}

export function selKey(s: Sel): string {
  return s.kind === 'attachment' ? `attachment:${s.skin ?? ''}/${s.slot ?? ''}/${s.name}` : `${s.kind}:${s.name}`;
}

/** 아직 문서에 있는 선택만 남긴다. */
export function pruneSelection(sel: Sel[], p: Project): Sel[] {
  const sk = p.skeleton;
  return sel.filter((s) => {
    switch (s.kind) {
      case 'skeleton':
      case 'image':
        return true;
      case 'bone':
        return sk.bones.some((b) => b.name === s.name);
      case 'slot':
        return sk.slots.some((x) => x.name === s.name);
      case 'constraint':
        return sk.constraints.some((x) => x.name === s.name);
      case 'skin':
        return sk.skins.some((x) => x.name === s.name);
      case 'event':
        return sk.events.some((x) => x.name === s.name);
      case 'animation':
        return sk.animations.some((x) => x.name === s.name);
      case 'attachment': {
        const skin = sk.skins.find((x) => x.name === s.skin);
        return !!skin && !!skin.attachments[s.slot ?? '']?.[s.name];
      }
    }
  });
}

export function namesOf(sel: Sel[], kind: SelKind): string[] {
  return sel.filter((s) => s.kind === kind).map((s) => s.name);
}

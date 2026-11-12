// 선택한 키(KeyRef) 찾기·복사 도우미.
import type { Animation, Key } from '../core/types';
import type { KeyRef } from '../state/ui';

/** KeyRef가 가리키는 키 목록(타임라인 전체) */
export function timelineKeys(a: Animation, ref: Pick<KeyRef, 'target' | 'timeline'>): Key[] | null {
  const t = ref.target;
  switch (t.kind) {
    case 'bone':
      return a.bones?.[t.name ?? '']?.[ref.timeline] ?? null;
    case 'slot':
      return a.slots?.[t.name ?? '']?.[ref.timeline] ?? null;
    case 'constraint':
      return a.constraints?.[t.name ?? '']?.[ref.timeline] ?? null;
    case 'deform':
      return a.deform?.[t.skin ?? '']?.[t.slot ?? '']?.[t.attachment ?? ''] ?? null;
    case 'sequence':
      return a.sequence?.[t.skin ?? '']?.[t.slot ?? '']?.[t.attachment ?? ''] ?? null;
    default:
      return null;
  }
}

/** 키와 다음 키 */
export function keyPair(a: Animation, ref: KeyRef): { key: Key; next: Key | null } | null {
  const keys = timelineKeys(a, ref);
  if (!keys) return null;
  const i = keys.findIndex((k) => Math.abs(k.time - ref.time) < 1e-6);
  if (i < 0) return null;
  return { key: keys[i], next: keys[i + 1] ?? null };
}

export function sameKey(a: KeyRef, b: KeyRef): boolean {
  return (
    a.timeline === b.timeline &&
    Math.abs(a.time - b.time) < 1e-6 &&
    a.target.kind === b.target.kind &&
    (a.target.name ?? '') === (b.target.name ?? '') &&
    (a.target.skin ?? '') === (b.target.skin ?? '') &&
    (a.target.slot ?? '') === (b.target.slot ?? '') &&
    (a.target.attachment ?? '') === (b.target.attachment ?? '')
  );
}

/** 복사: 가장 이른 키 기준 상대 시간(KeyCopy) */
export function copyKeys(a: Animation, refs: KeyRef[]): Record<string, unknown>[] {
  if (!refs.length) return [];
  const t0 = Math.min(...refs.map((r) => r.time));
  return refs.map((r) => {
    const p = keyPair(a, r);
    const out: Record<string, unknown> = { target: r.target, timeline: r.timeline, time: r.time - t0 };
    if (p) {
      out.v = p.key.v;
      if (p.key.s !== null) out.s = p.key.s;
      if (p.key.offset) out.offset = p.key.offset;
      if (p.key.curve !== null) out.curve = p.key.curve;
    }
    return out;
  });
}

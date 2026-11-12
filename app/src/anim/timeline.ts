// 애니메이션 타임라인 정리: 길이, 키 시점 목록, 도프시트 줄.
import type { Animation, Key } from '../core/types';
import type { KeyRef } from '../state/ui';

export interface TimelineRow {
  /** 줄 고유 키 */
  id: string;
  /** 묶음(대상) 이름 표시 */
  group: string;
  label: string;
  target: KeyRef['target'];
  timeline: string;
  keys: { time: number; key?: Key }[];
}

/** 애니메이션 길이 = 가장 늦은 키 시점 */
export function animationDuration(a: Animation | null | undefined): number {
  if (!a) return 0;
  let max = 0;
  for (const r of timelineRows(a)) for (const k of r.keys) if (k.time > max) max = k.time;
  return max;
}

/** 모든 키 시점(정렬, 중복 없음) */
export function keyTimes(a: Animation | null | undefined): number[] {
  if (!a) return [];
  const set = new Set<number>();
  for (const r of timelineRows(a)) for (const k of r.keys) set.add(k.time);
  return [...set].sort((x, y) => x - y);
}

const EPS = 1e-6;
export function prevKeyTime(times: number[], t: number): number | null {
  let best: number | null = null;
  for (const x of times) if (x < t - EPS) best = x;
  return best;
}
export function nextKeyTime(times: number[], t: number): number | null {
  for (const x of times) if (x > t + EPS) return x;
  return null;
}

const TIMELINE_LABELS: Record<string, string> = {
  rotate: '회전',
  translate: '이동',
  translatex: '이동 X',
  translatey: '이동 Y',
  scale: '크기',
  scalex: '크기 X',
  scaley: '크기 Y',
  shear: '기울기',
  shearx: '기울기 X',
  sheary: '기울기 Y',
  inherit: '상속',
  attachment: '첨부물',
  rgba: '색상',
  rgb: '색상',
  alpha: '투명도',
  rgba2: '색상·어두운 색',
  rgb2: '색상·어두운 색',
  ik: 'IK',
  transform: '트랜스폼',
  position: '위치',
  spacing: '간격',
  mix: '혼합',
  time: '시점',
  inertia: '관성',
  strength: '강도',
  damping: '감쇠',
  mass: '질량',
  wind: '바람',
  gravity: '중력',
  reset: '초기화',
};
export function timelineLabel(name: string): string {
  return TIMELINE_LABELS[name] ?? name;
}

function rowId(target: KeyRef['target'], tl: string): string {
  return `${target.kind}:${target.skin ?? ''}/${target.slot ?? ''}/${target.attachment ?? ''}/${target.name ?? ''}:${tl}`;
}

/** 도프시트 줄 목록(본 → 슬롯 → 제약 → 변형 → 시퀀스 → 그리기 순서 → 이벤트) */
export function timelineRows(a: Animation): TimelineRow[] {
  const rows: TimelineRow[] = [];
  const add = (group: string, target: KeyRef['target'], tls: Record<string, Key[]>) => {
    for (const [tl, keys] of Object.entries(tls)) {
      rows.push({ id: rowId(target, tl), group, label: timelineLabel(tl), target, timeline: tl, keys: keys.map((k) => ({ time: k.time, key: k })) });
    }
  };
  for (const [name, tls] of Object.entries(a.bones ?? {})) add(name, { kind: 'bone', name }, tls);
  for (const [name, tls] of Object.entries(a.slots ?? {})) add(name, { kind: 'slot', name }, tls);
  for (const [name, tls] of Object.entries(a.constraints ?? {})) add(name || '전체 물리', { kind: 'constraint', name }, tls);
  const nested = (kind: 'deform' | 'sequence', label: string, data: Animation['deform']) => {
    for (const [skin, slots] of Object.entries(data ?? {}))
      for (const [slot, atts] of Object.entries(slots))
        for (const [att, keys] of Object.entries(atts)) {
          const target: KeyRef['target'] = { kind, skin, slot, attachment: att };
          rows.push({ id: rowId(target, ''), group: `${slot}/${att}`, label, target, timeline: '', keys: keys.map((k) => ({ time: k.time, key: k })) });
        }
  };
  nested('deform', '변형', a.deform);
  nested('sequence', '시퀀스', a.sequence);
  if (a.drawOrder?.length)
    rows.push({ id: 'drawOrder', group: '그리기 순서', label: '그리기 순서', target: { kind: 'drawOrder' }, timeline: 'drawOrder', keys: a.drawOrder.map((k) => ({ time: k.time })) });
  if (a.events?.length)
    rows.push({ id: 'events', group: '이벤트', label: '이벤트', target: { kind: 'event' }, timeline: 'events', keys: a.events.map((k) => ({ time: k.time })) });
  return rows;
}

/** 대상(본·슬롯·제약)에 현재 시점 키가 있는가(트리 키 표시) */
export function hasKeyAt(a: Animation | null | undefined, kind: 'bone' | 'slot' | 'constraint', name: string, t: number): boolean {
  if (!a) return false;
  const map = kind === 'bone' ? a.bones : kind === 'slot' ? a.slots : a.constraints;
  const tls = map?.[name];
  if (!tls) return false;
  return Object.values(tls).some((keys) => keys.some((k) => Math.abs(k.time - t) < 1e-4));
}

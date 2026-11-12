// 스킨·이벤트·애니메이션·뼈대·이미지 속성.
import { createAttachment, renameItem } from '../../commands/actions';
import { animationDuration } from '../../anim/timeline';
import type { Animation, EventDef, Skin } from '../../core/types';
import { useServices } from '../../services';
import { namesOf } from '../../state/selection';
import { useStore } from '../../state/store';
import { NameField, Row } from '../fields/Fields';
import { NumberField } from '../fields/NumberField';
import { formatNumber } from '../fields/numberInput';
import { TextField } from '../fields/TextField';
import { FieldList, type FieldSpec } from './FieldList';

export function SkinProps({ skin }: { skin: Skin }) {
  const s = useServices();
  const active = useStore(s.doc, (d) => d.project.editor.activeSkin);
  const selection = useStore(s.ui, (u) => u.selection);
  const count = Object.values(skin.attachments).reduce((n, m) => n + Object.keys(m).length, 0);
  const add = (kind: 'bone' | 'constraint') => {
    const names = namesOf(selection, kind);
    if (!names.length) {
      s.ui.notify('warn', kind === 'bone' ? '추가할 본을 Ctrl+클릭으로 함께 고르세요' : '추가할 제약을 Ctrl+클릭으로 함께 고르세요');
      return;
    }
    s.doc.group('스킨에 추가', () => names.forEach((name) => s.doc.run('skin.addItem', { skin: skin.name, kind, name })));
  };
  return (
    <div className="props-group">
      <NameField value={skin.name} onCommit={(v) => renameItem(s, { kind: 'skin', name: skin.name }, v)} />
      <div className="props-note">첨부물 {count}개</div>
      <div className="props-buttons">
        <button disabled={active === skin.name} onClick={() => s.doc.run('skin.setActive', { name: skin.name })}>
          이 스킨 사용
        </button>
        <button disabled={!active} onClick={() => s.doc.run('skin.setActive', { name: null })}>
          스킨 끄기
        </button>
      </div>
      <div className="props-subtitle">스킨 전용 본</div>
      <ItemList items={skin.bones} onRemove={(name) => s.doc.run('skin.removeItem', { skin: skin.name, kind: 'bone', name })} />
      <button onClick={() => add('bone')}>선택한 본 추가</button>
      <div className="props-subtitle">스킨 전용 제약</div>
      <ItemList items={skin.constraints} onRemove={(name) => s.doc.run('skin.removeItem', { skin: skin.name, kind: 'constraint', name })} />
      <button onClick={() => add('constraint')}>선택한 제약 추가</button>
    </div>
  );
}

function ItemList({ items, onRemove }: { items: string[]; onRemove(n: string): void }) {
  if (!items.length) return <div className="props-note">(없음)</div>;
  return (
    <ul className="item-list">
      {items.map((n) => (
        <li key={n}>
          {n} <button onClick={() => onRemove(n)}>빼기</button>
        </li>
      ))}
    </ul>
  );
}

const EVENT_FIELDS: FieldSpec[] = [
  { key: 'int', label: '정수', kind: 'number' },
  { key: 'float', label: '실수', kind: 'number', step: 0.1 },
  { key: 'audio', label: '오디오', kind: 'text' },
  { key: 'volume', label: '볼륨', kind: 'number', step: 0.01 },
  { key: 'balance', label: '균형', kind: 'number', step: 0.01 },
];

export function EventProps({ ev }: { ev: EventDef }) {
  const s = useServices();
  const animate = useStore(s.doc, (d) => d.project.editor.mode === 'animate' && !!d.project.editor.activeAnimation);
  const set = (key: string, v: unknown) => s.doc.run('event.set', { name: ev.name, props: { [key]: v } });
  return (
    <div className="props-group">
      <NameField value={ev.name} onCommit={(v) => renameItem(s, { kind: 'event', name: ev.name }, v)} />
      <Row label="문자열">
        <TextField aria-label="문자열" value={ev.string} onCommit={(v) => set('string', v)} />
      </Row>
      <FieldList label="이벤트" fields={EVENT_FIELDS} value={ev as unknown as Record<string, unknown>} onChange={set} />
      <button disabled={!animate} onClick={() => s.doc.run('key.set', { target: { kind: 'event', name: ev.name }, timeline: 'events', time: s.pose.currentTime() })}>
        현재 시점에 이벤트 키
      </button>
    </div>
  );
}

export function AnimationProps({ anim }: { anim: Animation }) {
  const s = useServices();
  const active = useStore(s.doc, (d) => d.project.editor.activeAnimation);
  const fps = useStore(s.doc, (d) => d.project.skeleton.fps);
  const dur = animationDuration(anim);
  return (
    <div className="props-group">
      <NameField value={anim.name} onCommit={(v) => renameItem(s, { kind: 'animation', name: anim.name }, v)} />
      <div className="props-note">
        길이 {formatNumber(dur)}초 ({Math.round(dur * fps)}프레임)
      </div>
      <div className="props-buttons">
        <button disabled={active === anim.name} onClick={() => s.doc.run('animation.setActive', { name: anim.name })}>
          이 애니메이션 고르기
        </button>
        <button onClick={() => s.doc.run('animation.duplicate', { name: anim.name })}>복제</button>
      </div>
    </div>
  );
}

export function SkeletonProps() {
  const s = useServices();
  const sk = useStore(s.doc, (d) => d.project.skeleton);
  const imagesPath = useStore(s.doc, (d) => d.project.imagesPath);
  const onion = useStore(s.doc, (d) => d.project.editor.onion);
  return (
    <div className="props-group">
      <div className="props-kind">뼈대</div>
      <NumberField label="FPS" value={sk.fps} onCommit={(v) => s.doc.run('skeleton.set', { fps: v })} />
      <NumberField label="기준 배율" value={sk.referenceScale} onCommit={(v) => s.doc.run('skeleton.set', { referenceScale: v })} />
      <Row label="이미지 폴더">
        <TextField aria-label="이미지 폴더" value={imagesPath} onCommit={(v) => s.doc.run('project.setImagesPath', { path: v }).ok && void s.files.reloadImages()} />
      </Row>
      <div className="props-subtitle">오니언 스킨</div>
      <NumberField label="앞 프레임 수" value={onion.before} onCommit={(v) => s.doc.run('editor.set', { patch: { onion: { ...onion, before: Math.max(0, Math.round(v)) } } })} />
      <NumberField label="뒤 프레임 수" value={onion.after} onCommit={(v) => s.doc.run('editor.set', { patch: { onion: { ...onion, after: Math.max(0, Math.round(v)) } } })} />
      <NumberField label="간격" value={onion.step} onCommit={(v) => s.doc.run('editor.set', { patch: { onion: { ...onion, step: Math.max(1, Math.round(v)) } } })} />
      <div className="props-note">
        본 {sk.bones.length} · 슬롯 {sk.slots.length} · 제약 {sk.constraints.length} · 애니메이션 {sk.animations.length}
      </div>
    </div>
  );
}

export function ImageProps({ name }: { name: string }) {
  const s = useServices();
  const e = useStore(s.images, (i) => i.entries.find((x) => x.name === name));
  if (!e) return null;
  return (
    <div className="props-group">
      <div className="props-kind">이미지</div>
      <div className="props-note">{e.name}</div>
      <div className="props-note">
        {e.width} × {e.height}
      </div>
      <img className="image-preview" src={e.url} alt={e.name} />
      <button onClick={() => void createAttachment(s, 'region', e.name)}>선택한 슬롯에 붙이기</button>
    </div>
  );
}

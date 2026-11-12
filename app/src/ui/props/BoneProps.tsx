// 본 속성: 이름, 부모, 위치·회전·크기·기울기, 길이, 상속, 색상, 스킨 필요.
import { renameItem } from '../../commands/actions';
import type { Bone, Inherit } from '../../core/types';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { CheckField, ColorField, NameField, SelectField } from '../fields/Fields';
import { NumberField } from '../fields/NumberField';
import { currentLocal } from '../../tools/dragTools';

const INHERIT: { value: Inherit; label: string }[] = [
  { value: 'normal', label: '보통' },
  { value: 'onlyTranslation', label: '이동만' },
  { value: 'noRotationOrReflection', label: '회전·반사 없음' },
  { value: 'noScale', label: '크기 없음' },
  { value: 'noScaleOrReflection', label: '크기·반사 없음' },
];

export function BoneProps({ bone }: { bone: Bone }) {
  const s = useServices();
  const bones = useStore(s.doc, (d) => d.project.skeleton.bones);
  const mode = useStore(s.doc, (d) => d.project.editor.mode);
  useStore(s.doc, (d) => d.revision);
  useStore(s.ui, (u) => (u.playing ? Math.round(u.playTime * 30) : -1));
  const i = bones.findIndex((b) => b.name === bone.name);
  const local = currentLocal(s, s.pose.pose(), i);
  const set = (k: string, v: number) => s.doc.run('bone.setTransform', { name: bone.name, [k]: v });
  const descendants = new Set<string>([bone.name]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const b of bones) if (b.parent && descendants.has(b.parent) && !descendants.has(b.name)) {
      descendants.add(b.name);
      grew = true;
    }
  }
  const num = (key: keyof typeof local, label: string, step = 1) => (
    <NumberField key={key} label={label} value={local[key]} step={step} testId={`bone-${key}`} dragLabel={`본 ${label}`} onCommit={(v) => set(key, v)} />
  );
  return (
    <div className="props-group">
      <NameField value={bone.name} onCommit={(v) => renameItem(s, { kind: 'bone', name: bone.name }, v)} />
      {bone.parent !== null && (
        <SelectField
          label="부모"
          value={bone.parent}
          options={bones.filter((b) => !descendants.has(b.name)).map((b) => ({ value: b.name, label: b.name }))}
          onChange={(p) => p && s.doc.run('bone.setParent', { name: bone.name, parent: p })}
        />
      )}
      {mode === 'animate' && <div className="props-note">애니메이트 모드: 값을 바꾸면 현재 시점에 키가 생깁니다</div>}
      <div className="props-pair">
        {num('x', '위치 X')}
        {num('y', '위치 Y')}
      </div>
      {num('rotation', '회전')}
      <div className="props-pair">
        {num('scaleX', '크기 X', 0.01)}
        {num('scaleY', '크기 Y', 0.01)}
      </div>
      <div className="props-pair">
        {num('shearX', '기울기 X')}
        {num('shearY', '기울기 Y')}
      </div>
      <NumberField label="길이" value={bone.length} onCommit={(v) => s.doc.run('bone.setLength', { name: bone.name, length: v })} />
      <SelectField label="상속" value={bone.inherit} options={INHERIT} onChange={(v) => v && s.doc.run('bone.setInherit', { name: bone.name, inherit: v })} />
      <ColorField label="색상" value={bone.color} onChange={(c) => s.doc.run('bone.setColor', { name: bone.name, color: c })} />
      <CheckField label="스킨 필요" value={bone.skinRequired} onChange={(v) => s.doc.run('bone.setProps', { name: bone.name, skinRequired: v })} />
    </div>
  );
}

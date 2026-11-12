// 슬롯 속성: 이름, 본, 색상, 어두운 색, 블렌드, 셋업 첨부물.
import { renameItem } from '../../commands/actions';
import type { BlendMode, Slot } from '../../core/types';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { CheckField, ColorField, NameField, SelectField } from '../fields/Fields';

const BLEND: { value: BlendMode; label: string }[] = [
  { value: 'normal', label: '보통' },
  { value: 'additive', label: '더하기' },
  { value: 'multiply', label: '곱하기' },
  { value: 'screen', label: '스크린' },
];

export function SlotProps({ slot }: { slot: Slot }) {
  const s = useServices();
  const sk = useStore(s.doc, (d) => d.project.skeleton);
  const activeSkin = useStore(s.doc, (d) => d.project.editor.activeSkin);
  const attNames = new Set<string>();
  for (const skin of sk.skins) if (skin.name === 'default' || skin.name === activeSkin) Object.keys(skin.attachments[slot.name] ?? {}).forEach((n) => attNames.add(n));
  return (
    <div className="props-group">
      <NameField value={slot.name} onCommit={(v) => renameItem(s, { kind: 'slot', name: slot.name }, v)} />
      <SelectField label="본" value={slot.bone} options={sk.bones.map((b) => ({ value: b.name, label: b.name }))} onChange={(b) => b && s.doc.run('slot.setProps', { name: slot.name, bone: b })} />
      <ColorField label="색상" value={slot.color} onChange={(c) => s.doc.run('slot.setColor', { name: slot.name, color: c })} />
      <CheckField label="어두운 색 사용" value={slot.dark !== null} onChange={(on) => s.doc.run('slot.setDarkColor', { name: slot.name, dark: on ? '000000' : null })} />
      {slot.dark !== null && <ColorField label="어두운 색" alpha={false} value={slot.dark} onChange={(c) => s.doc.run('slot.setDarkColor', { name: slot.name, dark: c.slice(0, 6) })} />}
      <SelectField label="블렌드" value={slot.blend} options={BLEND} onChange={(b) => b && s.doc.run('slot.setBlend', { name: slot.name, blend: b })} />
      <SelectField
        label="셋업 첨부물"
        value={slot.attachment}
        options={[{ value: '', label: '(없음)' }, ...[...attNames].map((n) => ({ value: n, label: n }))]}
        onChange={(a) => s.doc.run('slot.setAttachment', { name: slot.name, attachment: a })}
      />
    </div>
  );
}

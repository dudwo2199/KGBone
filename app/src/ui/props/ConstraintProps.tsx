// 제약 속성: 종류별 필드, 순서 바꾸기, 트랜스폼 속성 매핑 표.
import { renameItem } from '../../commands/actions';
import type { BoneProperty, Constraint, FromProperty, TransformConstraint } from '../../core/types';
import { useServices } from '../../services';
import { useStore } from '../../state/store';
import { NameField } from '../fields/Fields';
import { formatNumber } from '../fields/numberInput';
import { FieldList, opts, type FieldSpec } from './FieldList';

const PROPS: BoneProperty[] = ['rotate', 'x', 'y', 'scaleX', 'scaleY', 'shearY'];
const PROP_LABEL: Record<string, string> = { rotate: '회전', x: '이동 X', y: '이동 Y', scaleX: '크기 X', scaleY: '크기 Y', shearY: '기울기 Y' };
const TYPE_LABEL = { ik: 'IK', transform: '트랜스폼', path: '패스', physics: '물리', slider: '슬라이더' };

function fieldsFor(c: Constraint, bones: string[], slots: string[], anims: string[]): FieldSpec[] {
  const mix = (k: string, l: string): FieldSpec => ({ key: k, label: l, kind: 'number', step: 0.01 });
  const skinReq: FieldSpec = { key: 'skinRequired', label: '스킨 필요', kind: 'bool' };
  switch (c.type) {
    case 'ik':
      return [
        { key: 'target', label: '목표', kind: 'select', options: opts(bones) },
        mix('mix', '혼합'),
        { key: 'softness', label: '부드러움', kind: 'number' },
        { key: 'bendPositive', label: '굽힘 방향(+)', kind: 'bool' },
        { key: 'compress', label: '압축', kind: 'bool' },
        { key: 'stretch', label: '늘이기', kind: 'bool' },
        { key: 'uniform', label: '균등', kind: 'bool' },
        { key: 'volume', label: '부피 보존', kind: 'bool' },
        skinReq,
      ];
    case 'transform':
      return [
        { key: 'source', label: '원본', kind: 'select', options: opts(bones) },
        { key: 'localSource', label: '로컬 원본', kind: 'bool' },
        { key: 'localTarget', label: '로컬 대상', kind: 'bool' },
        { key: 'additive', label: '더하기', kind: 'bool' },
        { key: 'clamp', label: '범위 제한', kind: 'bool' },
        mix('mixRotate', '혼합 회전'),
        mix('mixX', '혼합 X'),
        mix('mixY', '혼합 Y'),
        mix('mixScaleX', '혼합 크기 X'),
        mix('mixScaleY', '혼합 크기 Y'),
        mix('mixShearY', '혼합 기울기 Y'),
        skinReq,
      ];
    case 'path':
      return [
        { key: 'target', label: '패스 슬롯', kind: 'select', options: opts(slots) },
        { key: 'positionMode', label: '위치 방식', kind: 'select', options: opts(['fixed', 'percent'], { fixed: '고정', percent: '비율' }) },
        { key: 'spacingMode', label: '간격 방식', kind: 'select', options: opts(['length', 'fixed', 'percent', 'proportional'], { length: '길이', fixed: '고정', percent: '비율', proportional: '비례' }) },
        { key: 'rotateMode', label: '회전 방식', kind: 'select', options: opts(['tangent', 'chain', 'chainScale'], { tangent: '접선', chain: '사슬', chainScale: '사슬+크기' }) },
        { key: 'offsetRotation', label: '회전 보정', kind: 'number' },
        { key: 'position', label: '위치', kind: 'number' },
        { key: 'spacing', label: '간격', kind: 'number' },
        mix('mixRotate', '혼합 회전'),
        mix('mixX', '혼합 X'),
        mix('mixY', '혼합 Y'),
        skinReq,
      ];
    case 'physics':
      return [
        { key: 'bone', label: '본', kind: 'select', options: opts(bones) },
        mix('x', '이동 X'),
        mix('y', '이동 Y'),
        mix('rotate', '회전'),
        mix('scaleX', '크기 X'),
        mix('shearX', '기울기 X'),
        { key: 'limit', label: '속도 제한', kind: 'number' },
        { key: 'fps', label: '초당 단계', kind: 'number' },
        mix('inertia', '관성'),
        { key: 'strength', label: '강도', kind: 'number' },
        mix('damping', '감쇠'),
        mix('mass', '질량'),
        { key: 'wind', label: '바람', kind: 'number' },
        { key: 'gravity', label: '중력', kind: 'number' },
        mix('mix', '혼합'),
        { key: 'volume', label: '부피 보존', kind: 'bool' },
        skinReq,
      ];
    case 'slider':
      return [
        { key: 'animation', label: '애니메이션', kind: 'select', options: opts(anims) },
        { key: 'time', label: '시점', kind: 'number', step: 0.01 },
        { key: 'loop', label: '반복', kind: 'bool' },
        { key: 'additive', label: '더하기', kind: 'bool' },
        mix('mix', '혼합'),
        { key: 'bone', label: '본', kind: 'select', options: opts(bones), allowNone: true },
        { key: 'property', label: '속성', kind: 'select', options: opts(PROPS, PROP_LABEL) },
        { key: 'propertyFrom', label: '범위 시작', kind: 'number' },
        { key: 'propertyTo', label: '범위 끝', kind: 'number' },
        { key: 'local', label: '로컬 값', kind: 'bool' },
        skinReq,
      ];
  }
}

function MappingTable({ c, onChange }: { c: TransformConstraint; onChange(props: FromProperty[]): void }) {
  const rows = c.properties ?? [];
  const num = (v: number, set: (n: number) => void) => (
    <input
      type="text"
      defaultValue={formatNumber(v)}
      key={v}
      onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
      onBlur={(e) => {
        const n = parseFloat(e.target.value);
        if (isFinite(n) && n !== v) set(n);
      }}
    />
  );
  const update = (i: number, f: (r: FromProperty) => FromProperty) => onChange(rows.map((r, j) => (j === i ? f(structuredClone(r)) : r)));
  return (
    <div className="mapping">
      <div className="props-subtitle">속성 매핑</div>
      <table>
        <thead>
          <tr>
            <th>원본</th>
            <th>오프셋</th>
            <th>대상</th>
            <th>오프셋</th>
            <th>최대</th>
            <th>배율</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {rows.flatMap((r, i) =>
            r.to.map((t, k) => (
              <tr key={`${i}-${k}`}>
                <td>
                  {k === 0 && (
                    <select value={r.from} onChange={(e) => update(i, (x) => ({ ...x, from: e.target.value as BoneProperty }))}>
                      {PROPS.map((p) => (
                        <option key={p} value={p}>
                          {PROP_LABEL[p]}
                        </option>
                      ))}
                    </select>
                  )}
                </td>
                <td>{k === 0 && num(r.offset, (n) => update(i, (x) => ({ ...x, offset: n })))}</td>
                <td>
                  <select value={t.property} onChange={(e) => update(i, (x) => ((x.to[k].property = e.target.value as BoneProperty), x))}>
                    {PROPS.map((p) => (
                      <option key={p} value={p}>
                        {PROP_LABEL[p]}
                      </option>
                    ))}
                  </select>
                </td>
                <td>{num(t.offset, (n) => update(i, (x) => ((x.to[k].offset = n), x)))}</td>
                <td>{num(t.max, (n) => update(i, (x) => ((x.to[k].max = n), x)))}</td>
                <td>{num(t.scale, (n) => update(i, (x) => ((x.to[k].scale = n), x)))}</td>
                <td>
                  <button title="이 줄 빼기" onClick={() => onChange(rows.map((x, j) => (j === i ? { ...x, to: x.to.filter((_, m) => m !== k) } : x)).filter((x) => x.to.length))}>
                    ×
                  </button>
                </td>
              </tr>
            )),
          )}
        </tbody>
      </table>
      <button onClick={() => onChange([...rows, { from: 'rotate', offset: 0, to: [{ property: 'rotate', offset: 0, max: 0, scale: 1 }] }])}>매핑 추가</button>
    </div>
  );
}

export function ConstraintProps({ c }: { c: Constraint }) {
  const s = useServices();
  const sk = useStore(s.doc, (d) => d.project.skeleton);
  const index = sk.constraints.findIndex((x) => x.name === c.name);
  const set = (key: string, v: unknown) => s.doc.run('constraint.set', { name: c.name, props: { [key]: v } });
  const boneList = 'bones' in c ? c.bones : [];
  return (
    <div className="props-group">
      <div className="props-kind">{TYPE_LABEL[c.type]} 제약 · 적용 순서 {index + 1}</div>
      <NameField value={c.name} onCommit={(v) => renameItem(s, { kind: 'constraint', name: c.name }, v)} />
      {boneList.length > 0 && <div className="props-note">대상 본: {boneList.join(', ')}</div>}
      <FieldList label="제약" fields={fieldsFor(c, sk.bones.map((b) => b.name), sk.slots.map((x) => x.name), sk.animations.map((a) => a.name))} value={c as unknown as Record<string, unknown>} onChange={set} />
      {c.type === 'transform' && <MappingTable c={c} onChange={(p) => set('properties', p)} />}
      <div className="props-buttons">
        <button disabled={index <= 0} onClick={() => s.doc.run('constraint.reorder', { name: c.name, index: index - 1 })}>
          순서 위로
        </button>
        <button disabled={index >= sk.constraints.length - 1} onClick={() => s.doc.run('constraint.reorder', { name: c.name, index: index + 1 })}>
          순서 아래로
        </button>
      </div>
    </div>
  );
}

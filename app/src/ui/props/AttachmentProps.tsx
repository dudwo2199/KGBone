// 첨부물 속성: 종류별 필드와 메시 작업 버튼.
import { renameItem } from '../../commands/actions';
import type { Attachment } from '../../core/types';
import { useServices } from '../../services';
import { namesOf, type Sel } from '../../state/selection';
import { useStore } from '../../state/store';
import { NameField } from '../fields/Fields';
import { FieldList, opts, type FieldSpec } from './FieldList';

const TYPE_LABEL: Record<Attachment['type'], string> = {
  region: '이미지',
  mesh: '메시',
  linkedmesh: '링크드 메시',
  boundingbox: '바운딩 박스',
  point: '포인트',
  clipping: '클리핑',
  path: '패스',
};

function fieldsFor(a: Attachment, slots: string[], skins: string[]): FieldSpec[] {
  switch (a.type) {
    case 'region':
      return [
        { key: 'path', label: '경로', kind: 'text' },
        { key: 'x', label: '위치 X', kind: 'number' },
        { key: 'y', label: '위치 Y', kind: 'number' },
        { key: 'rotation', label: '회전', kind: 'number' },
        { key: 'scaleX', label: '크기 X', kind: 'number', step: 0.01 },
        { key: 'scaleY', label: '크기 Y', kind: 'number', step: 0.01 },
        { key: 'width', label: '폭', kind: 'number' },
        { key: 'height', label: '높이', kind: 'number' },
        { key: 'color', label: '색상', kind: 'color' },
      ];
    case 'mesh':
      return [
        { key: 'path', label: '경로', kind: 'text' },
        { key: 'width', label: '폭', kind: 'number' },
        { key: 'height', label: '높이', kind: 'number' },
        { key: 'color', label: '색상', kind: 'color' },
      ];
    case 'linkedmesh':
      return [
        { key: 'path', label: '경로', kind: 'text' },
        { key: 'parent', label: '부모 메시', kind: 'text' },
        { key: 'skin', label: '부모 스킨', kind: 'select', options: opts(skins), allowNone: true },
        { key: 'timelines', label: '변형 상속', kind: 'bool' },
        { key: 'color', label: '색상', kind: 'color' },
      ];
    case 'boundingbox':
      return [{ key: 'color', label: '색상', kind: 'color' }];
    case 'point':
      return [
        { key: 'x', label: '위치 X', kind: 'number' },
        { key: 'y', label: '위치 Y', kind: 'number' },
        { key: 'rotation', label: '회전', kind: 'number' },
        { key: 'color', label: '색상', kind: 'color' },
      ];
    case 'clipping':
      return [
        { key: 'end', label: '끝 슬롯', kind: 'select', options: opts(slots), allowNone: true },
        { key: 'inverse', label: '반전', kind: 'bool' },
        { key: 'convex', label: '볼록', kind: 'bool' },
        { key: 'color', label: '색상', kind: 'color' },
      ];
    case 'path':
      return [
        { key: 'closed', label: '닫힘', kind: 'bool' },
        { key: 'constantSpeed', label: '일정 속도', kind: 'bool' },
        { key: 'color', label: '색상', kind: 'color' },
      ];
  }
}

export function AttachmentProps({ sel, att }: { sel: Sel; att: Attachment }) {
  const s = useServices();
  const sk = useStore(s.doc, (d) => d.project.skeleton);
  const selection = useStore(s.ui, (u) => u.selection);
  const meshEdit = useStore(s.ui, (u) => u.meshEdit);
  const base = { slot: sel.slot, name: sel.name, ...(sel.skin && sel.skin !== 'default' ? { skin: sel.skin } : {}) };
  const set = (key: string, v: unknown) => s.doc.run('attachment.set', { ...base, props: { [key]: v } });
  const value: Record<string, unknown> = { ...att };
  if ('path' in att && att.path === null) value.path = sel.name;
  const bones = namesOf(selection, 'bone');
  return (
    <div className="props-group">
      <div className="props-kind">{TYPE_LABEL[att.type]} 첨부물 · 슬롯 {sel.slot}{sel.skin && sel.skin !== 'default' ? ` · 스킨 ${sel.skin}` : ''}</div>
      <NameField value={sel.name} onCommit={(v) => renameItem(s, sel, v)} />
      <FieldList label="첨부물" fields={fieldsFor(att, sk.slots.map((x) => x.name), sk.skins.map((x) => x.name))} value={value} onChange={set} />
      {att.type === 'mesh' && (
        <div className="props-buttons">
          <button
            className={meshEdit ? 'on' : ''}
            aria-pressed={meshEdit}
            data-testid="mesh-edit-toggle"
            title="뷰포트에서 정점 끌기·추가·삭제"
            onClick={() => s.ui.set({ meshEdit: !meshEdit, selectedVertices: [] })}
          >
            정점 편집
          </button>
          <button onClick={() => s.doc.run('mesh.generate', { ...base, detail: 0.5 })}>자동 생성</button>
          <button onClick={() => s.doc.run('mesh.setTriangles', base)}>삼각형 다시 나누기</button>
          <button
            title="Ctrl+클릭으로 함께 고른 본에 바인딩"
            onClick={() => {
              if (!bones.length) {
                s.ui.notify('warn', '바인딩할 본을 Ctrl+클릭으로 함께 고르세요');
                return;
              }
              s.doc.run('mesh.bind', { ...base, bones });
            }}
          >
            바인딩
          </button>
          <button disabled={!att.weighted} onClick={() => s.doc.run('mesh.unbind', base)}>
            바인딩 해제
          </button>
          <button onClick={() => s.doc.run('weights.auto', { ...base, ...(bones.length ? { bones } : {}) })}>자동 가중치</button>
          <div className="props-note">정점 {att.uvs.length / 2}개 · 삼각형 {att.triangles.length / 3}개{att.weighted ? ' · 가중치 있음' : ''}</div>
        </div>
      )}
      {(att.type === 'boundingbox' || att.type === 'clipping' || att.type === 'path') && <div className="props-note">정점 {att.vertexCount}개</div>}
    </div>
  );
}

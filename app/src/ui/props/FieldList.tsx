// 필드 설명 목록으로 속성 칸을 그린다(제약·이벤트·첨부물 공용).
import { CheckField, ColorField, NameField, SelectField } from '../fields/Fields';
import { NumberField } from '../fields/NumberField';

export type FieldSpec =
  | { key: string; label: string; kind: 'number'; step?: number }
  | { key: string; label: string; kind: 'bool' }
  | { key: string; label: string; kind: 'text' }
  | { key: string; label: string; kind: 'color' }
  | { key: string; label: string; kind: 'select'; options: { value: string; label: string }[]; allowNone?: boolean };

export interface FieldListProps {
  fields: FieldSpec[];
  value: Record<string, unknown>;
  onChange(key: string, v: unknown): void;
  /** 끌기 묶음 이름 앞부분 */
  label: string;
}

export function FieldList({ fields, value, onChange, label }: FieldListProps) {
  return (
    <>
      {fields.map((f) => {
        const v = value[f.key];
        switch (f.kind) {
          case 'number':
            return <NumberField key={f.key} label={f.label} value={typeof v === 'number' ? v : 0} step={f.step ?? 1} dragLabel={`${label} ${f.label}`} onCommit={(n) => onChange(f.key, n)} />;
          case 'bool':
            return <CheckField key={f.key} label={f.label} value={!!v} onChange={(b) => onChange(f.key, b)} />;
          case 'text':
            return <NameField key={f.key} label={f.label} value={typeof v === 'string' ? v : ''} onCommit={(t) => onChange(f.key, t)} />;
          case 'color':
            return <ColorField key={f.key} label={f.label} value={typeof v === 'string' ? v : 'FFFFFFFF'} onChange={(c) => onChange(f.key, c)} />;
          case 'select':
            return (
              <SelectField
                key={f.key}
                label={f.label}
                value={(v as string | null) ?? null}
                options={[...(f.allowNone ? [{ value: '' as const, label: '(없음)' }] : []), ...f.options]}
                onChange={(x) => onChange(f.key, x)}
              />
            );
        }
      })}
    </>
  );
}

export const opts = (values: string[], labels?: Record<string, string>) => values.map((v) => ({ value: v, label: labels?.[v] ?? v }));

// 작은 입력칸들: 켜기/끄기, 드롭다운, 색 선택, 이름(글자).
import type { ReactNode } from 'react';
import { TextField } from './TextField';

export function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="field">
      <span className="field-label">{label}</span>
      {children}
    </div>
  );
}

export function CheckField({ label, value, onChange }: { label: string; value: boolean; onChange(v: boolean): void }) {
  return (
    <label className="field check">
      <input type="checkbox" checked={value} onChange={(e) => onChange(e.target.checked)} />
      <span>{label}</span>
    </label>
  );
}

export function SelectField<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T | null;
  options: { value: T | ''; label: string }[];
  onChange(v: T | null): void;
}) {
  return (
    <Row label={label}>
      <select aria-label={label} value={value ?? ''} onChange={(e) => onChange((e.target.value || null) as T | null)}>
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </Row>
  );
}

export function NameField({ label = '이름', value, onCommit }: { label?: string; value: string; onCommit(v: string): void }) {
  return (
    <Row label={label}>
      <TextField aria-label={label} value={value} onCommit={onCommit} />
    </Row>
  );
}

/** "RRGGBBAA" 색: 색 상자(RGB) + 투명도(0~255) */
export function ColorField({ label, value, onChange, alpha = true }: { label: string; value: string; onChange(v: string): void; alpha?: boolean }) {
  const rgb = (value || 'FFFFFFFF').slice(0, 6).toLowerCase();
  const a = value && value.length >= 8 ? parseInt(value.slice(6, 8), 16) : 255;
  const hex2 = (n: number) => Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, '0');
  return (
    <Row label={label}>
      <input type="color" aria-label={label} value={`#${rgb}`} onChange={(e) => onChange(e.target.value.slice(1).toUpperCase() + (alpha ? hex2(a).toUpperCase() : ''))} />
      {alpha && (
        <input
          type="range"
          min={0}
          max={255}
          value={a}
          title="투명도"
          aria-label={`${label} 투명도`}
          onChange={(e) => onChange(rgb.toUpperCase() + hex2(Number(e.target.value)).toUpperCase())}
        />
      )}
    </Row>
  );
}

// 숫자 입력칸: 수식 입력(Enter 확정·Esc 취소) + 라벨 좌우 끌기(끌기 한 번 = 되돌리기 한 번).
import { useRef, useState } from 'react';
import { useServices } from '../../services';
import { dragValue, evaluateNumberInput, formatNumber } from './numberInput';

export interface NumberFieldProps {
  label: string;
  value: number;
  onCommit(v: number): void;
  step?: number;
  /** 끌기 묶음 이름(되돌리기 목록) */
  dragLabel?: string;
  testId?: string;
  disabled?: boolean;
}

export function NumberField({ label, value, onCommit, step = 1, dragLabel, testId, disabled }: NumberFieldProps) {
  const s = useServices();
  const [editing, setEditing] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const skipBlur = useRef(false);
  const drag = useRef<{ x: number; start: number; moved: boolean } | null>(null);

  const confirm = (text: string): boolean => {
    const r = evaluateNumberInput(s.core, text, value);
    if (!r.ok) {
      setError(true);
      s.ui.notify('error', `수식 오류: ${r.error}`);
      return false;
    }
    setError(false);
    if (r.value !== value) onCommit(r.value);
    return true;
  };

  return (
    <div className={'field number' + (error ? ' error' : '')}>
      <span
        className="field-label drag"
        title="좌우로 끌어 값 바꾸기"
        onPointerDown={(e) => {
          if (disabled || e.button !== 0) return;
          (e.target as HTMLElement).setPointerCapture(e.pointerId);
          drag.current = { x: e.clientX, start: value, moved: false };
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (!d) return;
          const dx = e.clientX - d.x;
          if (!d.moved && Math.abs(dx) < 2) return;
          if (!d.moved) {
            d.moved = true;
            s.doc.beginGroup(dragLabel ?? label);
          }
          onCommit(dragValue(d.start, dx, e.shiftKey ? step * 0.1 : step));
        }}
        onPointerUp={() => {
          const d = drag.current;
          drag.current = null;
          if (d?.moved) s.doc.endGroup();
        }}
      >
        {label}
      </span>
      <input
        type="text"
        data-testid={testId}
        aria-label={label}
        disabled={disabled}
        value={editing ?? formatNumber(value)}
        onFocus={(e) => {
          setEditing(formatNumber(value));
          // 다음 프레임에 전체 선택. 그 사이에 확정(Enter)·취소로 초점이 떠났으면 select()가 초점을 도로 가져가므로 하지 않는다
          // (그러면 이후 글자 단축키가 입력칸으로 가서 먹히지 않는다)
          const input = e.target;
          requestAnimationFrame(() => {
            if (document.activeElement === input) input.select();
          });
        }}
        onChange={(e) => setEditing(e.target.value)}
        onBlur={() => {
          if (skipBlur.current) skipBlur.current = false;
          else if (editing !== null && editing !== formatNumber(value)) confirm(editing);
          setEditing(null);
          setError(false);
        }}
        onKeyDown={(e) => {
          if (e.nativeEvent.isComposing) return;
          if (e.key === 'Enter') {
            e.preventDefault();
            e.stopPropagation();
            if (editing !== null && confirm(editing)) {
              skipBlur.current = true;
              setEditing(null);
              (e.target as HTMLInputElement).blur();
            }
          } else if (e.key === 'Escape') {
            e.preventDefault();
            e.stopPropagation();
            skipBlur.current = true;
            setEditing(null);
            setError(false);
            (e.target as HTMLInputElement).blur();
          }
        }}
      />
    </div>
  );
}

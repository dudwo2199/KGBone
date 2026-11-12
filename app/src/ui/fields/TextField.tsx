// 한국어 조합이 깨지지 않는 글자 입력칸: 조합 중에는 내부 상태만 바꾸고, Enter·초점 잃기에서 확정한다.
import { useEffect, useRef, useState, type InputHTMLAttributes } from 'react';

export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'value' | 'onChange' | 'onSubmit'> {
  value: string;
  /** 확정(Enter·초점 잃기) */
  onCommit(v: string): void;
  /** 입력할 때마다(검색칸처럼 바로 반영할 때). 조합 중에는 부르지 않는다 */
  onLive?(v: string): void;
  /** Esc */
  onCancel?(): void;
  autoSelect?: boolean;
}

export function TextField({ value, onCommit, onLive, onCancel, autoSelect, ...rest }: TextFieldProps) {
  const [draft, setDraft] = useState(value);
  const composing = useRef(false);
  const focused = useRef(false);
  const ref = useRef<HTMLInputElement>(null);
  const done = useRef(false);

  // 바깥 값이 바뀌면, 편집 중이 아닐 때만 따라간다(조합 중 값 덮어쓰기 금지)
  useEffect(() => {
    if (!focused.current && !composing.current) setDraft(value);
  }, [value]);

  useEffect(() => {
    if (autoSelect && ref.current) {
      ref.current.focus();
      ref.current.select();
    }
  }, [autoSelect]);

  const commit = () => {
    if (done.current) return;
    if (draft !== value) onCommit(draft);
  };

  return (
    <input
      {...rest}
      ref={ref}
      type="text"
      value={draft}
      spellCheck={false}
      onFocus={(e) => {
        focused.current = true;
        done.current = false;
        rest.onFocus?.(e);
      }}
      onBlur={(e) => {
        focused.current = false;
        commit();
        rest.onBlur?.(e);
      }}
      onCompositionStart={() => {
        composing.current = true;
      }}
      onCompositionEnd={(e) => {
        composing.current = false;
        const v = (e.target as HTMLInputElement).value;
        setDraft(v);
        onLive?.(v);
      }}
      onChange={(e) => {
        setDraft(e.target.value);
        const native = e.nativeEvent as InputEvent;
        if (!composing.current && !native.isComposing) onLive?.(e.target.value);
      }}
      onKeyDown={(e) => {
        if (e.nativeEvent.isComposing || composing.current || e.keyCode === 229) return;
        if (e.key === 'Enter') {
          e.preventDefault();
          e.stopPropagation();
          commit();
          done.current = true;
          (e.target as HTMLInputElement).blur();
        } else if (e.key === 'Escape') {
          e.preventDefault();
          e.stopPropagation();
          done.current = true;
          setDraft(value);
          onCancel?.();
          (e.target as HTMLInputElement).blur();
        }
        rest.onKeyDown?.(e);
      }}
    />
  );
}

// 숫자 입력칸 수식 연결: 핵심의 evalExpr로 계산한다(v·value, +=, -=, *=, /=, 사칙연산).
export interface ExprEvaluator {
  evalExpr(input: string, current: number): number;
}

export type NumberParse = { ok: true; value: number } | { ok: false; error: string };

/** 입력 글자 → 숫자. 핵심 계산이 실패하면, 그냥 숫자일 때만 그대로 받는다 */
export function evaluateNumberInput(core: ExprEvaluator, text: string, current: number): NumberParse {
  const t = text.trim();
  if (!t) return { ok: false, error: '값을 입력하세요' };
  try {
    const v = core.evalExpr(t, current);
    if (typeof v !== 'number' || !isFinite(v)) return { ok: false, error: '계산 결과가 숫자가 아닙니다' };
    return { ok: true, value: v };
  } catch (e) {
    if (/^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$/i.test(t)) return { ok: true, value: parseFloat(t) };
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** 화면에 보일 숫자(소수 셋째 자리까지, 뒤 0 없앰) */
export function formatNumber(v: number, digits = 3): string {
  if (!isFinite(v)) return '0';
  const s = v.toFixed(digits);
  return s.includes('.') ? s.replace(/\.?0+$/, '') || '0' : s;
}

/** 라벨 끌기: 가로 이동 픽셀 → 새 값 */
export function dragValue(start: number, dxPixels: number, step: number): number {
  return start + Math.round(dxPixels) * step;
}

import { describe, expect, it } from 'vitest';
import { FakeCore } from '../../testing/fakes';
import { dragValue, evaluateNumberInput, formatNumber } from './numberInput';

describe('I01 숫자칸 수식 연결', () => {
  it('u_i01_number_field_uses_core_eval_expr', () => {
    const core = new FakeCore();
    expect(evaluateNumberInput(core, 'v*2', 7)).toEqual({ ok: true, value: 14 });
    expect(evaluateNumberInput(core, '+=5', 7)).toEqual({ ok: true, value: 12 });
    // 핵심에 현재 값이 그대로 넘어간다
    expect(core.exprCalls).toEqual([
      ['v*2', 7],
      ['+=5', 7],
    ]);
  });

  it('u_i01_invalid_expression_is_error', () => {
    const core = new FakeCore();
    const r = evaluateNumberInput(core, '((', 1);
    expect(r.ok).toBe(false);
    expect(evaluateNumberInput(core, '  ', 1).ok).toBe(false);
  });

  it('u_i01_format_and_drag', () => {
    expect(formatNumber(1.5)).toBe('1.5');
    expect(formatNumber(2)).toBe('2');
    expect(formatNumber(-0.12345)).toBe('-0.123');
    expect(dragValue(10, 5, 0.1)).toBeCloseTo(10.5);
  });
});

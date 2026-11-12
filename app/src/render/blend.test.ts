import { describe, expect, it } from 'vitest';
import { blendFunc } from './blend';

describe('C04 블렌드 모드', () => {
  it('u_c04_blend_mode_selects_blend_function', () => {
    expect(blendFunc('normal')).toEqual({ src: 'ONE', dst: 'ONE_MINUS_SRC_ALPHA' });
    expect(blendFunc('additive')).toEqual({ src: 'ONE', dst: 'ONE' });
    expect(blendFunc('multiply')).toEqual({ src: 'DST_COLOR', dst: 'ONE_MINUS_SRC_ALPHA' });
    expect(blendFunc('screen')).toEqual({ src: 'ONE', dst: 'ONE_MINUS_SRC_COLOR' });
  });

  it('u_c04_modes_are_distinct', () => {
    const all = (['normal', 'additive', 'multiply', 'screen'] as const).map((m) => JSON.stringify(blendFunc(m)));
    expect(new Set(all).size).toBe(4);
  });
});

// 블렌드 모드 → WebGL 블렌드 함수(C04). 텍스처는 미리 곱한 알파(premultiplied)로 올린다.
import type { BlendMode } from '../core/types';

export type GlBlendFactor = 'ONE' | 'ONE_MINUS_SRC_ALPHA' | 'DST_COLOR' | 'ONE_MINUS_SRC_COLOR';

export interface BlendFunc {
  src: GlBlendFactor;
  dst: GlBlendFactor;
}

/**
 * 미리 곱한 알파 기준 섞기 식:
 * - 보통: 결과 = 원본 + 바탕 × (1 − 원본 알파)
 * - 더하기: 결과 = 원본 + 바탕
 * - 곱하기: 결과 = 원본 × 바탕 + 바탕 × (1 − 원본 알파)
 * - 스크린: 결과 = 원본 + 바탕 × (1 − 원본)
 */
export function blendFunc(mode: BlendMode): BlendFunc {
  switch (mode) {
    case 'additive':
      return { src: 'ONE', dst: 'ONE' };
    case 'multiply':
      return { src: 'DST_COLOR', dst: 'ONE_MINUS_SRC_ALPHA' };
    case 'screen':
      return { src: 'ONE', dst: 'ONE_MINUS_SRC_COLOR' };
    case 'normal':
    default:
      return { src: 'ONE', dst: 'ONE_MINUS_SRC_ALPHA' };
  }
}

/** 블렌드 모드를 GL 상태에 적용 */
export function applyBlend(gl: WebGL2RenderingContext, mode: BlendMode): void {
  const f = blendFunc(mode);
  gl.blendFunc(gl[f.src], gl[f.dst]);
}

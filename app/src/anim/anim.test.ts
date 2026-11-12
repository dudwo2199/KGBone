import { describe, expect, it } from 'vitest';
import type { Key } from '../core/types';
import { onionTimes } from '../render/onion';
import { defaultBezier, fitGraph, graphToScreen, moveHandle, screenToGraph, segmentPoints, type GraphView } from './graph';
import { advancePlayback } from './playback';

const k = (time: number, v: number[], curve: Key['curve'] = null): Key => ({ time, v, s: null, offset: 0, curve });

describe('G11 재생', () => {
  const base = { time: 0, playing: true, reverse: false, loop: true, speed: 1 };

  it('u_g11_speed_two_advances_twice', () => {
    // 같은 실제 시간 0.25초: 1배 → 0.25, 2배 → 0.5
    expect(advancePlayback(base, 0.25, 2).time).toBeCloseTo(0.25);
    expect(advancePlayback({ ...base, speed: 2 }, 0.25, 2).time).toBeCloseTo(0.5);
  });

  it('u_g11_loop_off_stops_at_end', () => {
    const r = advancePlayback({ ...base, loop: false, time: 1.9 }, 0.5, 2);
    expect(r.time).toBe(2);
    expect(r.playing).toBe(false);
  });

  it('u_g11_loop_wraps', () => {
    // 1.9 + 0.5 = 2.4 → 길이 2로 감으면 0.4
    expect(advancePlayback({ ...base, time: 1.9 }, 0.5, 2).time).toBeCloseTo(0.4);
  });

  it('u_g11_reverse_stops_at_zero', () => {
    const r = advancePlayback({ ...base, loop: false, reverse: true, time: 0.1 }, 0.5, 2);
    expect(r).toMatchObject({ time: 0, playing: false });
  });
});

describe('G12 오니언 스킨', () => {
  it('u_g12_times_match_before_after_settings', () => {
    // fps 10, 간격 1프레임 = 0.1초. 현재 0.5: 앞 0.4, 0.3 / 뒤 0.6, 0.7, 0.8
    const r = onionTimes(0.5, { before: 2, after: 3, step: 1, fps: 10, duration: 2, loop: false });
    expect(r.before).toEqual([0.4, 0.3]);
    expect(r.after).toEqual([0.6, 0.7, 0.8]);
  });

  it('u_g12_step_and_range', () => {
    // 간격 2프레임(fps 10) = 0.2초. 현재 0.1, 반복 없음: 앞 -0.1은 빠짐, 뒤 0.3
    const r = onionTimes(0.1, { before: 1, after: 1, step: 2, fps: 10, duration: 1, loop: false });
    expect(r.before).toEqual([]);
    expect(r.after).toEqual([0.3]);
    // 반복이면 -0.1 → 0.9
    expect(onionTimes(0.1, { before: 1, after: 0, step: 2, fps: 10, duration: 1, loop: true }).before).toEqual([0.9]);
  });
});

describe('G10 그래프 좌표 변환', () => {
  const g: GraphView = { t0: 0, t1: 2, v0: -10, v1: 10, width: 220, height: 120, pad: 10 };

  it('u_g10_graph_to_screen_and_back', () => {
    // 그림 영역 200×100. t=1 → x = 10 + 0.5×200 = 110, v=0 → y = 10 + 0.5×100 = 60
    expect(graphToScreen(g, 1, 0)).toEqual({ x: 110, y: 60 });
    // v=10(위 끝) → y = 10
    expect(graphToScreen(g, 0, 10)).toEqual({ x: 10, y: 10 });
    const p = screenToGraph(g, 160, 35);
    expect(p.t).toBeCloseTo(1.5);
    expect(p.v).toBeCloseTo(5);
  });

  it('u_g10_fit_includes_all_keys', () => {
    const v = fitGraph([k(0, [0]), k(1, [100])], 0, 200, 100, 0);
    expect(v.t0).toBe(0);
    expect(v.t1).toBe(1);
    // 값 범위 0~100에 10% 여유 → -10 ~ 110
    expect(v.v0).toBeCloseTo(-10);
    expect(v.v1).toBeCloseTo(110);
  });

  it('u_g10_bezier_handles', () => {
    const a = k(0, [0]);
    const b = k(3, [30]);
    // 1/3, 2/3 지점: (1, 0), (2, 30)
    expect(defaultBezier(a, b)).toEqual([1, 0, 2, 30]);
    const pts = segmentPoints(a, b, 0, [1, 0, 2, 30], 2);
    // s=0.5: t = 3×0.125×1 + 3×0.125×2 + 0.125×3 = 1.5, v = 3×0.125×30 + 0.125×30 = 15
    expect(pts[1].t).toBeCloseTo(1.5);
    expect(pts[1].v).toBeCloseTo(15);
    // 손잡이 시간은 구간 안으로 제한
    expect(moveHandle([1, 0, 2, 30], 0, 1, -5, 7, 0, 3)).toEqual([0, 7, 2, 30]);
  });

  it('u_g10_stepped_segment', () => {
    expect(segmentPoints(k(0, [1]), k(1, [5]), 0, 'stepped').map((p) => p.v)).toEqual([1, 1, 5]);
  });
});

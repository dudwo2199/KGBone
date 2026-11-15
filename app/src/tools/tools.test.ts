import { describe, expect, it } from 'vitest';
import type { WorldBone } from '../core/types';
import { axisFrame, lockToFrame, scaleRatios, snapRotation, toFrame } from './axis';
import { hitVertex, insertVertex, moveVertices, removeVertices, worldToLocalVertices } from './meshEdit';

// 정사각형 메시(외곽선 4개): (0,0) (10,0) (10,10) (0,10), uv는 모서리
const square = { vertices: [0, 0, 10, 0, 10, 10, 0, 10], uvs: [0, 1, 1, 1, 1, 0, 0, 0], hull: 4 };

describe('D02 메시 정점 편집(화면 계산)', () => {
  it('u_d02_insert_vertex_on_nearest_hull_edge', () => {
    // (5, -1)은 변 0→1(아래 변)에 가장 가깝다 → 위치 1에 끼움, uv는 변 가운데 (0.5, 1)
    const r = insertVertex(square, 5, -1);
    expect(r.index).toBe(1);
    expect(r.shape.vertices).toEqual([0, 0, 5, -1, 10, 0, 10, 10, 0, 10]);
    expect(r.shape.uvs).toEqual([0, 1, 0.5, 1, 1, 1, 1, 0, 0, 0]);
    expect(r.shape.hull).toBe(5);
  });

  it('u_d02_insert_vertex_on_closing_edge_goes_to_hull_end', () => {
    // (-1, 5)는 마지막 변 3→0(왼쪽 변)에 가깝다 → 외곽선 끝(위치 4)
    const r = insertVertex(square, -1, 5);
    expect(r.index).toBe(4);
    expect(r.shape.vertices.slice(8)).toEqual([-1, 5]);
    expect(r.shape.uvs.slice(8)).toEqual([0, 0.5]);
  });

  it('u_d02_insert_keeps_interior_vertices_after_hull', () => {
    const m = { vertices: [...square.vertices, 5, 5], uvs: [...square.uvs, 0.5, 0.5], hull: 4 };
    const r = insertVertex(m, 11, 5);
    // 오른쪽 변 1→2 → 위치 2, 안쪽 정점(5,5)은 맨 뒤에 그대로
    expect(r.index).toBe(2);
    expect(r.shape.vertices).toEqual([0, 0, 10, 0, 11, 5, 10, 10, 0, 10, 5, 5]);
    expect(r.shape.hull).toBe(5);
  });

  it('u_d02_remove_vertices', () => {
    const five = insertVertex(square, 5, -1).shape;
    const r = removeVertices(five, [1])!;
    expect(r.vertices).toEqual(square.vertices);
    expect(r.uvs).toEqual(square.uvs);
    expect(r.hull).toBe(4);
    // 외곽선이 3개보다 적어지면 거부
    expect(removeVertices(square, [0, 1])).toBeNull();
  });

  it('u_d02_move_and_hit_and_local', () => {
    expect(moveVertices(square.vertices, [2], 1, -2)).toEqual([0, 0, 10, 0, 11, 8, 0, 10]);
    expect(hitVertex([0, 0, 10, 0], 9, 1, 2)).toBe(1);
    expect(hitVertex([0, 0, 10, 0], 5, 5, 2)).toBe(-1);
    // 슬롯 본: 90° 회전 + (100, 0) 이동. 월드 (100, 10) → 로컬 (10, 0)
    const bone: WorldBone = { a: 0, b: -1, c: 1, d: 0, x: 100, y: 0 };
    const local = worldToLocalVertices([100, 10], bone);
    expect(local[0]).toBeCloseTo(10);
    expect(local[1]).toBeCloseTo(0);
  });
});

describe('B04 변환 축(로컬/부모/월드)', () => {
  // 본: 90° 회전(x축이 월드 위쪽), 부모: 회전 없음
  const bone: WorldBone = { a: 0, b: -1, c: 1, d: 0, x: 0, y: 0 };
  const parent: WorldBone = { a: 1, b: 0, c: 0, d: 1, x: 0, y: 0 };

  it('u_b04_axis_frames', () => {
    expect(axisFrame('world', bone, parent)).toEqual({ ux: 1, uy: 0, vx: 0, vy: 1 });
    expect(axisFrame('parent', bone, parent)).toEqual({ ux: 1, uy: 0, vx: 0, vy: 1 });
    const f = axisFrame('local', bone, parent);
    expect([f.ux, f.uy, f.vx, f.vy].map((v) => Math.round(v))).toEqual([0, 1, -1, 0]);
  });

  it('u_b04_shift_translate_locks_to_axis', () => {
    // 끌기 (3, 4): 월드 축이면 y축(4가 더 큼)만 → (0, 4)
    expect(lockToFrame(axisFrame('world', bone, parent), 3, 4)).toEqual({ dx: 0, dy: 4 });
    // 45° 틀(로컬 축이 대각선)이면 (3, 4)는 x축 성분 7/√2 → (3.5, 3.5)
    const diag: WorldBone = { a: 1, b: -1, c: 1, d: 1, x: 0, y: 0 };
    const r = lockToFrame(axisFrame('local', diag, parent), 3, 4);
    expect(r.dx).toBeCloseTo(3.5);
    expect(r.dy).toBeCloseTo(3.5);
  });

  it('u_b04_scale_uses_axis_frame', () => {
    // 월드 틀에서 끌기: 시작 (10, 0) → (20, 0)이면 월드 x 비율 2.
    // 본 x축은 월드 y쪽이므로 본의 크기 Y에 2가 간다
    const f = axisFrame('world', bone, parent);
    const a = toFrame(f, 0, 0, 10, 0);
    const b = toFrame(f, 0, 0, 20, 0);
    expect(scaleRatios(f, bone, a, b)).toEqual({ sx: 1, sy: 2 });
    // 로컬 틀이면 본 x축 방향(월드 위) 끌기가 크기 X
    const fl = axisFrame('local', bone, parent);
    const r = scaleRatios(fl, bone, toFrame(fl, 0, 0, 0, 10), toFrame(fl, 0, 0, 0, 30));
    expect(r.sx).toBeCloseTo(3);
    expect(r.sy).toBe(1);
  });

  it('u_b04_rotation_snap_per_axis', () => {
    // 시작 로컬 10°, 바뀐 양 22°, 부모 월드 각 30°
    expect(snapRotation('local', 10, 22, 30, 1)).toBe(25); // 10 + round(22/15)×15 = 25
    expect(snapRotation('parent', 10, 22, 30, 1)).toBe(30); // round(32/15)×15 = 30
    expect(snapRotation('world', 10, 22, 30, 1)).toBe(30); // 월드 62 → 60, 로컬 = 60 − 30 = 30
    expect(snapRotation('world', 10, 35, 5, 1)).toBe(40); // 월드 5 + 45 = 50 → 45, 로컬 = 45 − 5 = 40
  });
});

//! 가중치: 바인딩, 자동 가중치, 페인팅.
//!
//! - 자동 가중치: 정점에서 각 본 선분(원점 → 끝, 월드)까지 거리 d로 w = 1 / (d + 0.01)³, 큰 것부터 최대 4개, 합 1.
//! - 바인딩: 본마다 정점의 월드 좌표를 그 본의 로컬로 옮겨 저장하므로 Σ wᵢ·Mᵢ(로컬ᵢ) = 원래 월드 위치가 된다.
//! - 페인팅: 중심에서 거리 r < radius인 정점에 f = 1 − r/radius로 감쇠한 양 strength·f를
//!   add(더함) / remove(뺌) / replace(그 본 가중치를 strength 쪽으로 f만큼 옮김) 하고, 나머지 본 가중치를 비율대로 늘이거나 줄여 합 1.
use crate::pose::WorldBone;

/// 가중치 계산에 쓰는 본 정보(월드).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneInfo {
    /// `skeleton.bones` 안의 번호.
    pub index: usize,
    pub world: WorldBone,
    pub length: f32,
}

/// 정점 하나의 (본 번호, 가중치) 목록. 합은 1.
pub type VertexWeights = Vec<(usize, f32)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintMode {
    Add,
    Remove,
    Replace,
}

const MAX_INFLUENCES: usize = 4;

fn seg_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 { (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
    ((px - ax - dx * t).powi(2) + (py - ay - dy * t).powi(2)).sqrt()
}

fn normalize(w: &mut VertexWeights) {
    w.retain(|(_, v)| *v > 1e-6);
    let sum: f32 = w.iter().map(|(_, v)| *v).sum();
    if sum > 0.0 {
        w.iter_mut().for_each(|(_, v)| *v /= sum);
    }
}

/// 정점 월드 좌표마다 가까운 본일수록 큰 가중치(거리 반비례, 합 = 1).
pub fn auto_weights(world_vertices: &[f32], bones: &[BoneInfo]) -> Vec<VertexWeights> {
    world_vertices
        .chunks_exact(2)
        .map(|p| {
            let mut w: VertexWeights = bones
                .iter()
                .map(|b| {
                    let (tx, ty) = b.world.local_to_world(b.length, 0.0);
                    let d = seg_dist(p[0], p[1], b.world.x, b.world.y, tx, ty);
                    (b.index, 1.0 / (d + 0.01).powi(3))
                })
                .collect();
            w.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
            w.truncate(MAX_INFLUENCES);
            normalize(&mut w);
            w
        })
        .collect()
}

/// 가중치 + 월드 좌표 → Spine식 가중치 정점 배열([본 수, (본 번호, 본 기준 x, y, 가중치)×n]…).
/// 셋업 포즈에서 바인딩 전후 정점 월드 위치가 같다.
pub fn bind_vertices(world_vertices: &[f32], weights: &[VertexWeights], world: &[WorldBone]) -> Vec<f32> {
    let mut out = vec![];
    for (p, w) in world_vertices.chunks_exact(2).zip(weights) {
        let valid: Vec<&(usize, f32)> = w.iter().filter(|(b, _)| *b < world.len()).collect();
        out.push(valid.len() as f32);
        for &&(b, wt) in &valid {
            let (lx, ly) = world[b].world_to_local(p[0], p[1]);
            out.extend_from_slice(&[b as f32, lx, ly, wt]);
        }
    }
    out
}

/// 가중치 정점 배열에서 정점마다 (본, 가중치) 목록을 꺼낸다.
pub(crate) fn read_weights(weighted: &[f32]) -> Vec<VertexWeights> {
    let mut out = vec![];
    let mut i = 0;
    while i < weighted.len() {
        let n = weighted[i].max(0.0) as usize;
        i += 1;
        let mut w = vec![];
        for _ in 0..n {
            if i + 3 >= weighted.len() {
                break;
            }
            w.push((weighted[i] as usize, weighted[i + 3]));
            i += 4;
        }
        out.push(w);
    }
    out
}

/// 가중치 정점 배열 → 슬롯 본 기준 가중치 없는 정점 [x, y, …] (셋업 포즈 위치 유지).
pub fn unbind_vertices(weighted: &[f32], world: &[WorldBone], slot_bone: &WorldBone) -> Vec<f32> {
    let wv = crate::pose::vertex_world_positions(true, weighted, slot_bone, world, None);
    wv.chunks_exact(2)
        .flat_map(|p| {
            let (x, y) = slot_bone.world_to_local(p[0], p[1]);
            [x, y]
        })
        .collect()
}

/// 붓질 한 번: 중심 (x, y)에서 `radius` 안 정점의 `bone` 가중치를 `strength`만큼 바꾸고 다시 합 = 1로 맞춘다.
#[allow(clippy::too_many_arguments)]
pub fn paint_weights(
    weights: &mut [VertexWeights],
    world_vertices: &[f32],
    bone: usize,
    x: f32,
    y: f32,
    radius: f32,
    strength: f32,
    mode: PaintMode,
) {
    if radius <= 0.0 {
        return;
    }
    for (w, p) in weights.iter_mut().zip(world_vertices.chunks_exact(2)) {
        let r = ((p[0] - x).powi(2) + (p[1] - y).powi(2)).sqrt();
        if r >= radius {
            continue;
        }
        let f = 1.0 - r / radius;
        let cur = w.iter().find(|(b, _)| *b == bone).map(|(_, v)| *v).unwrap_or(0.0);
        let others: f32 = w.iter().filter(|(b, _)| *b != bone).map(|(_, v)| *v).sum();
        let mut target = match mode {
            PaintMode::Add => cur + strength * f,
            PaintMode::Remove => cur - strength * f,
            PaintMode::Replace => cur + (strength - cur) * f,
        }
        .clamp(0.0, 1.0);
        // 다른 본이 없으면 이 본이 1을 가져야 한다.
        if others <= 1e-6 {
            target = 1.0;
        }
        let rest = 1.0 - target;
        for (b, v) in w.iter_mut() {
            if *b != bone && others > 0.0 {
                *v = *v / others * rest;
            }
        }
        if let Some(e) = w.iter_mut().find(|(b, _)| *b == bone) {
            e.1 = target;
        } else {
            w.push((bone, target));
        }
        normalize(w);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(index: usize, x: f32, y: f32, rot: f32, len: f32) -> BoneInfo {
        let (s, c) = rot.to_radians().sin_cos();
        BoneInfo { index, world: WorldBone { a: c, b: -s, c: s, d: c, x, y }, length: len }
    }

    #[test]
    fn f_d06_auto_weights() {
        // 본 0: (0,0)→(10,0), 본 1: (10,0)→(20,0). 정점 (2, 1)은 본 0에 가깝고 (18, −1)은 본 1에 가깝다.
        let bones = [bone(0, 0.0, 0.0, 0.0, 10.0), bone(1, 10.0, 0.0, 0.0, 10.0)];
        let w = auto_weights(&[2.0, 1.0, 18.0, -1.0, 10.0, 5.0], &bones);
        for v in &w {
            let s: f32 = v.iter().map(|x| x.1).sum();
            assert!((s - 1.0).abs() < 1e-5);
        }
        let get = |v: &VertexWeights, b: usize| v.iter().find(|x| x.0 == b).map(|x| x.1).unwrap_or(0.0);
        assert!(get(&w[0], 0) > get(&w[0], 1));
        assert!(get(&w[1], 1) > get(&w[1], 0));
        // (10, 5)는 두 본에서 거리 5로 같다 → 0.5씩
        assert!((get(&w[2], 0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn f_d04_bind_keeps_world_positions() {
        let bones = [bone(0, 0.0, 0.0, 30.0, 10.0), bone(1, 8.66, 5.0, 75.0, 10.0)];
        let world: Vec<WorldBone> = bones.iter().map(|b| b.world).collect();
        let verts = [1.0, 2.0, 9.0, 9.0, -3.0, 4.0, 12.0, 14.0];
        let w = auto_weights(&verts, &bones);
        let bound = bind_vertices(&verts, &w, &world);
        let back = crate::pose::vertex_world_positions(true, &bound, &WorldBone::default(), &world, None);
        for (a, b) in verts.iter().zip(&back) {
            assert!((a - b).abs() < 1e-3, "{verts:?} {back:?}");
        }
        // 해제도 위치 유지(슬롯 본 = 본 0)
        let un = unbind_vertices(&bound, &world, &world[0]);
        let again = crate::pose::vertex_world_positions(false, &un, &world[0], &world, None);
        for (a, b) in verts.iter().zip(&again) {
            assert!((a - b).abs() < 1e-3);
        }
    }

    #[test]
    fn f_d05_paint_sums_to_one() {
        let verts = [0.0, 0.0, 1.0, 0.0, 5.0, 0.0, 50.0, 0.0];
        let mut w: Vec<VertexWeights> = vec![vec![(0, 0.7), (1, 0.3)], vec![(0, 1.0)], vec![(1, 0.5), (2, 0.5)], vec![(0, 1.0)]];
        for mode in [PaintMode::Add, PaintMode::Remove, PaintMode::Replace, PaintMode::Remove] {
            paint_weights(&mut w, &verts, 1, 0.0, 0.0, 10.0, 0.6, mode);
            for v in &w {
                let s: f32 = v.iter().map(|x| x.1).sum();
                assert!((s - 1.0).abs() < 1e-4, "{mode:?} {w:?}");
            }
        }
        // 반경 밖 정점(50, 0)은 그대로
        assert_eq!(w[3], vec![(0, 1.0)]);
        // add: 중심 정점 본 1 = 0.3 + 0.6 = 0.9, 본 0 = 0.1
        let mut w2: Vec<VertexWeights> = vec![vec![(0, 0.7), (1, 0.3)]];
        paint_weights(&mut w2, &[0.0, 0.0], 1, 0.0, 0.0, 10.0, 0.6, PaintMode::Add);
        assert!((w2[0].iter().find(|x| x.0 == 1).unwrap().1 - 0.9).abs() < 1e-5);
    }
}

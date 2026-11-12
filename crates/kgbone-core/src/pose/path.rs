//! 패스 제약.
//!
//! 패스 첨부물(대상 슬롯의 현재 첨부물)의 월드 정점은 [앞 제어, 점, 뒤 제어]가 정점마다 이어진 3차 베지어 연속 구간이다.
//! 구간 i: 점 i → 뒤 제어 i → 앞 제어 i+1 → 점 i+1. closed면 마지막 점 → 첫 점 구간이 더 있다.
//! - 길이: 구간마다 64등분한 꺾은선 길이를 누적한 표. constantSpeed면 거리 → (구간, 매개변수)를 이 표로 찾고,
//!   아니면 거리 비율을 전체 매개변수(구간 수)에 그대로 대응시킨다.
//! - 위치: percent = position × 전체 길이, fixed = position.
//! - 간격(본 i 다음까지): length = 본 길이 × 월드 scaleX + spacing, fixed = spacing, percent = spacing × 전체 길이,
//!   proportional = spacing × 전체 길이 × (본 길이 / 본 길이 합).
//! - 열린 패스에서 범위를 벗어난 거리는 양 끝으로 자르고, 닫힌 패스는 한 바퀴로 감는다.
//! - 회전: tangent = 그 점의 접선 방향, chain = 다음 본 자리를 향함, chainScale = chain + 다음 자리까지 닿도록 scaleX.
//!   모두 offsetRotation을 더한다. 위치는 `현재 + (목표 − 현재) × mixX/mixY`, 회전은 감은 차이 × mixRotate.
use super::bone_transform::{set_world, update_world};
use super::math::{rotate_world, wrap_deg};
use super::mesh_deform::{find_attachment, vertex_world_positions};
use super::{ConstraintSolver, SkeletonPose, SolveContext};
use crate::model::{Attachment, Constraint, PathAttachment, PositionMode, RotateMode, Skeleton, SpacingMode};

pub(crate) struct PathSolver;

const SAMPLES: usize = 64;

/// 월드 좌표 베지어 연속 구간.
struct PathCurve {
    /// 구간마다 [p0, c1, c2, p1] (x, y).
    segs: Vec<[(f32, f32); 4]>,
    /// 구간마다 SAMPLES+1개의 누적 길이(전체 기준).
    table: Vec<Vec<f32>>,
    total: f32,
    closed: bool,
    constant_speed: bool,
}

fn bez(s: &[(f32, f32); 4], t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (a * s[0].0 + b * s[1].0 + c * s[2].0 + d * s[3].0, a * s[0].1 + b * s[1].1 + c * s[2].1 + d * s[3].1)
}

fn bez_tangent(s: &[(f32, f32); 4], t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (a, b, c) = (3.0 * u * u, 6.0 * u * t, 3.0 * t * t);
    let dx = a * (s[1].0 - s[0].0) + b * (s[2].0 - s[1].0) + c * (s[3].0 - s[2].0);
    let dy = a * (s[1].1 - s[0].1) + b * (s[2].1 - s[1].1) + c * (s[3].1 - s[2].1);
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        // 제어점이 끝점과 겹치면 현(弦) 방향
        return (s[3].0 - s[0].0, s[3].1 - s[0].1);
    }
    (dx, dy)
}

impl PathCurve {
    fn new(world: &[f32], closed: bool, constant_speed: bool) -> Option<PathCurve> {
        let pts: Vec<(f32, f32)> = world.chunks_exact(2).map(|p| (p[0], p[1])).collect();
        let k = pts.len() / 3;
        if k < 2 && !(closed && k >= 1) {
            return None;
        }
        let mut segs = Vec::new();
        for i in 0..k - 1 {
            segs.push([pts[3 * i + 1], pts[3 * i + 2], pts[3 * i + 3], pts[3 * i + 4]]);
        }
        if closed {
            segs.push([pts[3 * (k - 1) + 1], pts[3 * (k - 1) + 2], pts[0], pts[1]]);
        }
        let mut table = Vec::with_capacity(segs.len());
        let mut acc = 0.0f32;
        for s in &segs {
            let mut row = Vec::with_capacity(SAMPLES + 1);
            row.push(acc);
            let mut prev = s[0];
            for j in 1..=SAMPLES {
                let p = bez(s, j as f32 / SAMPLES as f32);
                acc += ((p.0 - prev.0).powi(2) + (p.1 - prev.1).powi(2)).sqrt();
                row.push(acc);
                prev = p;
            }
            table.push(row);
        }
        Some(PathCurve { segs, table, total: acc, closed, constant_speed })
    }

    /// 거리 → (구간, 매개변수).
    fn locate(&self, dist: f32) -> (usize, f32) {
        let mut d = dist;
        if self.closed && self.total > 0.0 {
            d = d.rem_euclid(self.total);
        } else {
            d = d.clamp(0.0, self.total);
        }
        let n = self.segs.len();
        if !self.constant_speed {
            let g = if self.total > 0.0 { d / self.total * n as f32 } else { 0.0 };
            let i = (g.floor() as usize).min(n - 1);
            return (i, (g - i as f32).clamp(0.0, 1.0));
        }
        let i = self.table.iter().position(|row| d <= *row.last().unwrap_or(&0.0)).unwrap_or(n - 1);
        let row = &self.table[i];
        let j = row.iter().position(|&v| v >= d).unwrap_or(SAMPLES).max(1);
        let (l0, l1) = (row[j - 1], row[j]);
        let f = if l1 > l0 { (d - l0) / (l1 - l0) } else { 0.0 };
        (i, ((j - 1) as f32 + f) / SAMPLES as f32)
    }

    /// 거리 → (위치, 접선 각도(도)).
    fn sample(&self, dist: f32) -> ((f32, f32), f32) {
        let (i, t) = self.locate(dist);
        let s = &self.segs[i];
        let (tx, ty) = bez_tangent(s, t);
        (bez(s, t), ty.atan2(tx).to_degrees())
    }
}

/// 패스 첨부물의 월드 정점.
fn path_world<'a>(skel: &'a Skeleton, pose: &SkeletonPose, skin: Option<&str>, slot: usize) -> Option<(Vec<f32>, &'a PathAttachment)> {
    let s = &skel.slots[slot];
    let name = pose.slot_attachments[slot].as_deref()?;
    let (_, att) = find_attachment(skel, skin, &s.name, name)?;
    let Attachment::Path(p) = att else { return None };
    let bone = skel.bone_index(&s.bone).map(|i| pose.world[i]).unwrap_or_default();
    let deform = pose.deform.get(&(s.name.clone(), name.to_string())).map(Vec::as_slice);
    Some((vertex_world_positions(p.weighted, &p.vertices, &bone, &pose.world, deform), p))
}

impl ConstraintSolver for PathSolver {
    fn apply(&mut self, skel: &Skeleton, index: usize, pose: &mut SkeletonPose, ctx: &mut SolveContext) {
        let Constraint::Path(c) = &skel.constraints[index] else { return };
        let Some(si) = skel.slot_index(&c.target) else { return };
        let Some((world, att)) = path_world(skel, pose, ctx.skin, si) else { return };
        let Some(curve) = PathCurve::new(&world, att.closed, att.constant_speed) else { return };
        let v = &pose.constraint_values[index];
        let (position, spacing, mix_r, mix_x, mix_y) = (v[0], v[1], v[2], v[3], v[4]);
        let bones: Vec<usize> = c.bones.iter().filter_map(|n| skel.bone_index(n)).collect();
        if bones.is_empty() {
            return;
        }
        let lens: Vec<f32> = bones.iter().map(|&b| skel.bones[b].length * pose.world[b].scale_x()).collect();
        let sum_len: f32 = lens.iter().sum();
        let gaps: Vec<f32> = lens
            .iter()
            .map(|&l| match c.spacing_mode {
                SpacingMode::Length => l + spacing,
                SpacingMode::Fixed => spacing,
                SpacingMode::Percent => spacing * curve.total,
                SpacingMode::Proportional => {
                    if sum_len > 0.0 {
                        spacing * curve.total * l / sum_len
                    } else {
                        0.0
                    }
                }
            })
            .collect();
        let start = match c.position_mode {
            PositionMode::Percent => position * curve.total,
            PositionMode::Fixed => position,
        };
        // 본 i의 거리, 그리고 마지막 본 다음 자리까지(n+1개)
        let mut dists = vec![start];
        for g in &gaps {
            let last = *dists.last().unwrap_or(&start);
            dists.push(last + g);
        }
        let samples: Vec<((f32, f32), f32)> = dists.iter().map(|&d| curve.sample(d)).collect();
        for (k, &b) in bones.iter().enumerate() {
            let ((px, py), tangent) = samples[k];
            let mut w = pose.world[b];
            let (nx, ny) = samples[k + 1].0;
            let chain_angle = (ny - py).atan2(nx - px).to_degrees();
            let target_rot = match c.rotate_mode {
                RotateMode::Tangent => tangent,
                RotateMode::Chain | RotateMode::ChainScale => chain_angle,
            } + c.offset_rotation;
            w.x += (px - w.x) * mix_x;
            w.y += (py - w.y) * mix_y;
            let turn = wrap_deg(target_rot - w.rotation_x()) * mix_r;
            rotate_world(&mut w, turn);
            let cur_len = skel.bones[b].length * w.scale_x();
            if c.rotate_mode == RotateMode::ChainScale && cur_len > 1e-5 {
                let reach = ((nx - px).powi(2) + (ny - py).powi(2)).sqrt();
                let f = 1.0 + (reach / cur_len - 1.0) * mix_r;
                w.a *= f;
                w.c *= f;
            }
            set_world(ctx.parents, pose, b, &w);
            update_world(skel, ctx.parents, pose, b);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Attachment, Bone, Constraint, PathAttachment, PathConstraint, PositionMode, RotateMode, Skeleton, Slot, SpacingMode};
    use crate::pose::{PoseEngine, PoseRequest};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-2
    }

    /// (0,0) → (100,0) 직선 패스(제어점도 직선 위 1/3, 2/3), 길이 20인 본 3개 사슬.
    fn skel(spacing_mode: SpacingMode, rotate: RotateMode, position: f32, spacing: f32, vertical: bool) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        let mut prev = "root".to_string();
        for i in 0..3 {
            let mut b = Bone::new(format!("b{i}"), Some(prev.clone()));
            b.length = 20.0;
            if i > 0 {
                b.x = 20.0;
            }
            prev = b.name.clone();
            s.bones.push(b);
        }
        let mut pb = Bone::new("pathbone", Some("root".into()));
        if vertical {
            pb.rotation = 90.0;
        }
        s.bones.push(pb);
        s.slots.push(Slot { attachment: Some("p".into()), ..Slot::new("path", "pathbone") });
        let v = vec![-10.0, 0.0, 0.0, 0.0, 100.0 / 3.0, 0.0, 200.0 / 3.0, 0.0, 100.0, 0.0, 110.0, 0.0];
        let att = PathAttachment { closed: false, constant_speed: true, vertex_count: 6, weighted: false, vertices: v, lengths: vec![100.0], color: crate::model::Color::PATH };
        s.skins[0].attachments.entry("path").insert("p", Attachment::Path(att));
        s.constraints.push(Constraint::Path(PathConstraint {
            name: "pc".into(),
            skin_required: false,
            bones: vec!["b0".into(), "b1".into(), "b2".into()],
            target: "path".into(),
            position_mode: PositionMode::Percent,
            spacing_mode,
            rotate_mode: rotate,
            offset_rotation: 0.0,
            position,
            spacing,
            mix_rotate: 1.0,
            mix_x: 1.0,
            mix_y: 1.0,
        }));
        s
    }

    #[test]
    fn f_f04_straight_path_positions() {
        // 위치 10% → 10, 간격 length(20 + 5) → 본 위치 10, 35, 60, 회전 0
        let out = PoseEngine::new().compute(&skel(SpacingMode::Length, RotateMode::Tangent, 0.1, 5.0, false), &PoseRequest::default());
        for (k, x) in [10.0, 35.0, 60.0].into_iter().enumerate() {
            let w = out.bones[1 + k];
            assert!(close(w.x, x) && close(w.y, 0.0) && close(w.rotation_x(), 0.0), "{k}: {w:?}");
        }
        // fixed 30: 10, 40, 70
        let out = PoseEngine::new().compute(&skel(SpacingMode::Fixed, RotateMode::Chain, 0.1, 30.0, false), &PoseRequest::default());
        assert!(close(out.bones[3].x, 70.0));
        // percent 0.2 → 20씩: 10, 30, 50
        let out = PoseEngine::new().compute(&skel(SpacingMode::Percent, RotateMode::Tangent, 0.1, 0.2, false), &PoseRequest::default());
        assert!(close(out.bones[2].x, 30.0) && close(out.bones[3].x, 50.0));
        // proportional 0.6: 전체 60을 길이 비율(1:1:1)로 → 20씩
        let out = PoseEngine::new().compute(&skel(SpacingMode::Proportional, RotateMode::Tangent, 0.1, 0.6, false), &PoseRequest::default());
        assert!(close(out.bones[3].x, 50.0));
        // chainScale, fixed 30: 다음 자리까지 30, 본 길이 20 → scaleX 1.5
        let out = PoseEngine::new().compute(&skel(SpacingMode::Fixed, RotateMode::ChainScale, 0.1, 30.0, false), &PoseRequest::default());
        assert!(close(out.bones[1].scale_x(), 1.5), "{}", out.bones[1].scale_x());
        // 패스 본을 90° 돌리면 패스가 y축 위: 본 위치 (0, 10), (0, 35), (0, 60), 회전 90
        let out = PoseEngine::new().compute(&skel(SpacingMode::Length, RotateMode::Tangent, 0.1, 5.0, true), &PoseRequest::default());
        for (k, y) in [10.0, 35.0, 60.0].into_iter().enumerate() {
            let w = out.bones[1 + k];
            assert!(close(w.x, 0.0) && close(w.y, y) && close(w.rotation_x(), 90.0), "{k}: {w:?}");
        }
    }
}

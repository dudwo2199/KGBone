//! IK 제약(1본·2본, 부피 보존).
//!
//! 계산은 모두 "첫 본의 부모 공간"(부모의 상속 행렬 + 부모 원점, `math::parent_space`)에서 한다.
//! - 1본: 본 원점 → 목표 방향 각에서 shearX를 뺀 값이 목표 회전. `회전 += 감은(목표 − 회전) × mix`.
//!   compress(짧아지기)/stretch(늘어나기): 목표 거리 d와 부모 공간 본 길이 L = length × scaleX로 s = d / L,
//!   `scaleX *= 1 + (s − 1) × mix`, uniform이면 scaleY도 같이, volume이면 scaleY를 그 배율로 나눈다.
//! - 2본: 부모 본 길이 l1 = |부모 원점 → 자식 원점|, 자식 길이 l2 = |자식 원점 → 자식 끝|, 목표 거리 d.
//!   코사인 법칙으로 부모 쪽 각 A = acos((l1² + d² − l2²) / (2·l1·d)), 팔꿈치 각 B = acos((l1² + l2² − d²) / (2·l1·l2)).
//!   bendPositive(굽힘 +1)면 자식이 반시계로 굽는다: 부모 방향 = 목표 방향 − bend·A, 자식은 목표 쪽 끝점을 향한다.
//!   softness s > 0이고 d > l1 + l2 − s 이면 d' = (l1 + l2 − s) + s·(1 − e^(−(d − (l1 + l2 − s))/s)) 로 부드럽게 줄인다.
//!   stretch이고 d > l1 + l2 이면 부모 scaleX를 k = d / (l1 + l2)배(자식이 normal 상속으로 같이 늘어난다고 본다).
use super::bone_transform::{parent_world, update_world};
use super::math::{parent_space, wrap_deg};
use super::{ConstraintSolver, SkeletonPose, SolveContext};
use crate::model::{Constraint, Skeleton};

pub(crate) struct IkSolver;

impl ConstraintSolver for IkSolver {
    fn apply(&mut self, skel: &Skeleton, index: usize, pose: &mut SkeletonPose, ctx: &mut SolveContext) {
        let Constraint::Ik(c) = &skel.constraints[index] else { return };
        let vals = pose.constraint_values[index].clone();
        let mix = vals.first().copied().unwrap_or(c.mix);
        let softness = vals.get(1).copied().unwrap_or(c.softness).max(0.0);
        let bend = if vals.get(2).copied().unwrap_or(1.0) < 0.0 { -1.0 } else { 1.0 };
        let compress = vals.get(3).copied().unwrap_or(0.0) > 0.5;
        let stretch = vals.get(4).copied().unwrap_or(0.0) > 0.5;
        let Some(ti) = skel.bone_index(&c.target) else { return };
        let (tx, ty) = (pose.world[ti].x, pose.world[ti].y);
        let bones: Vec<usize> = c.bones.iter().filter_map(|n| skel.bone_index(n)).collect();
        let opts = Opts { mix, compress, stretch, uniform: c.uniform, volume: c.volume };
        match bones.as_slice() {
            [b] => one_bone(skel, ctx.parents, pose, *b, tx, ty, opts),
            [p, ch] => two_bone(skel, ctx.parents, pose, *p, *ch, tx, ty, softness, bend, opts),
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
struct Opts {
    mix: f32,
    compress: bool,
    stretch: bool,
    uniform: bool,
    volume: bool,
}

/// 본 `i`가 월드 점 (tx, ty)를 향하는 로컬 회전과, 부모 공간에서의 거리.
fn aim(parents: &[Option<usize>], pose: &SkeletonPose, i: usize, tx: f32, ty: f32) -> (f32, f32) {
    let space = parent_space(parent_world(parents, &pose.world, i).as_ref(), pose.inherits[i]);
    let (px, py) = space.world_to_local(tx, ty);
    let l = pose.locals[i];
    let (dx, dy) = (px - l.x, py - l.y);
    (dy.atan2(dx).to_degrees() - l.shear_x, (dx * dx + dy * dy).sqrt())
}

/// 늘이기·줄이기 배율 `f`를 본 로컬 크기에 적용한다.
fn apply_scale(pose: &mut SkeletonPose, i: usize, f: f32, o: Opts) {
    let l = &mut pose.locals[i];
    l.scale_x *= f;
    if o.uniform {
        l.scale_y *= f;
    } else if o.volume && f.abs() > 1e-6 {
        l.scale_y /= f;
    }
}

fn one_bone(skel: &Skeleton, parents: &[Option<usize>], pose: &mut SkeletonPose, i: usize, tx: f32, ty: f32, o: Opts) {
    let (rot, d) = aim(parents, pose, i, tx, ty);
    let cur = pose.locals[i].rotation;
    pose.locals[i].rotation = cur + wrap_deg(rot - cur) * o.mix;
    let len = skel.bones[i].length * pose.locals[i].scale_x;
    if len.abs() > 1e-5 && ((o.compress && d < len) || (o.stretch && d > len)) {
        let s = d / len;
        apply_scale(pose, i, 1.0 + (s - 1.0) * o.mix, o);
    }
    update_world(skel, parents, pose, i);
}

#[allow(clippy::too_many_arguments)]
fn two_bone(skel: &Skeleton, parents: &[Option<usize>], pose: &mut SkeletonPose, p: usize, c: usize, tx: f32, ty: f32, softness: f32, bend: f32, o: Opts) {
    let space = parent_space(parent_world(parents, &pose.world, p).as_ref(), pose.inherits[p]);
    let lp = pose.locals[p];
    let (p0x, p0y) = (lp.x, lp.y);
    let cw = pose.world[c];
    let (c0x, c0y) = space.world_to_local(cw.x, cw.y);
    let (tipwx, tipwy) = cw.local_to_world(skel.bones[c].length, 0.0);
    let (tipx, tipy) = space.world_to_local(tipwx, tipwy);
    let (ttx, tty) = space.world_to_local(tx, ty);
    let hyp = |x: f32, y: f32| (x * x + y * y).sqrt();
    let l1 = hyp(c0x - p0x, c0y - p0y);
    let l2 = hyp(tipx - c0x, tipy - c0y);
    let d = hyp(ttx - p0x, tty - p0y);
    if l1 < 1e-5 || d < 1e-5 {
        one_bone(skel, parents, pose, c, tx, ty, o);
        return;
    }
    let maxl = l1 + l2;
    let (mut a1, mut a2, mut dd, mut k) = (l1, l2, d, 1.0f32);
    if o.stretch && d > maxl && maxl > 1e-5 {
        k = d / maxl;
        a1 *= k;
        a2 *= k;
    } else {
        if softness > 0.0 && d > maxl - softness {
            let start = maxl - softness;
            dd = start + softness * (1.0 - (-(d - start) / softness).exp());
        }
        dd = dd.min(maxl).max((l1 - l2).abs());
    }
    let cos_a = ((a1 * a1 + dd * dd - a2 * a2) / (2.0 * a1 * dd)).clamp(-1.0, 1.0);
    let ang_a = cos_a.acos().to_degrees();
    let theta_t = (tty - p0y).atan2(ttx - p0x).to_degrees();
    let theta1 = theta_t - bend * ang_a;
    let a_pc = (c0y - p0y).atan2(c0x - p0x).to_degrees();
    let delta = wrap_deg(theta1 - a_pc);
    // 자식이 향할 점(부모 공간): 목표 방향으로 d'만큼
    let dir = theta_t.to_radians();
    let (qx, qy) = space.local_to_world(p0x + dd * dir.cos(), p0y + dd * dir.sin());

    let orig_p = pose.locals[p];
    let orig_c_rot = pose.locals[c].rotation;
    // 부모를 끝까지(mix 1) 돌렸을 때 자식의 목표 회전
    pose.locals[p].rotation = orig_p.rotation + delta;
    if k != 1.0 {
        apply_scale(pose, p, k, o);
    }
    update_world(skel, parents, pose, p);
    let (rot_c, _) = aim(parents, pose, c, qx, qy);
    // mix 반영
    pose.locals[p] = orig_p;
    pose.locals[p].rotation = orig_p.rotation + delta * o.mix;
    if k != 1.0 {
        apply_scale(pose, p, 1.0 + (k - 1.0) * o.mix, o);
    }
    pose.locals[c].rotation = orig_c_rot + wrap_deg(rot_c - orig_c_rot) * o.mix;
    update_world(skel, parents, pose, p.min(c));
}

#[cfg(test)]
mod tests {
    use crate::model::{Bone, Constraint, IkConstraint, Skeleton};
    use crate::pose::{PoseEngine, PoseRequest};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn ik(bones: &[&str], target: &str) -> IkConstraint {
        IkConstraint {
            name: "ik".into(),
            skin_required: false,
            bones: bones.iter().map(|s| s.to_string()).collect(),
            target: target.into(),
            mix: 1.0,
            softness: 0.0,
            bend_positive: true,
            compress: false,
            stretch: false,
            uniform: false,
            volume: false,
        }
    }

    fn bone(name: &str, parent: &str, x: f32, y: f32, len: f32) -> Bone {
        let mut b = Bone::new(name, Some(parent.into()));
        b.x = x;
        b.y = y;
        b.length = len;
        b
    }

    fn one_bone_skel(c: IkConstraint, len: f32, tx: f32, ty: f32) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        s.bones.push(bone("a", "root", 0.0, 0.0, len));
        s.bones.push(bone("t", "root", tx, ty, 0.0));
        s.constraints.push(Constraint::Ik(c));
        s
    }

    #[test]
    fn f_f01_one_bone_ik() {
        // 본 a: 원점, 길이 2. 목표 (3, 4) → 회전 atan2(4, 3) = 53.1301°
        let s = one_bone_skel(ik(&["a"], "t"), 2.0, 3.0, 4.0);
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!(close(out.locals[1].rotation, 53.1301), "{}", out.locals[1].rotation);
        assert!(close(out.locals[1].scale_x, 1.0));
        // mix 0.5 → 26.5651°
        let mut c = ik(&["a"], "t");
        c.mix = 0.5;
        let out = PoseEngine::new().compute(&one_bone_skel(c, 2.0, 3.0, 4.0), &PoseRequest::default());
        assert!(close(out.locals[1].rotation, 26.5651));
        // stretch: 거리 5, 길이 2 → scaleX = 2.5. uniform이면 scaleY도 2.5
        let mut c = ik(&["a"], "t");
        c.stretch = true;
        c.uniform = true;
        let out = PoseEngine::new().compute(&one_bone_skel(c, 2.0, 3.0, 4.0), &PoseRequest::default());
        assert!(close(out.locals[1].scale_x, 2.5) && close(out.locals[1].scale_y, 2.5));
        // compress: 길이 10, 거리 5 → scaleX = 0.5. stretch만 켜면 줄지 않는다
        let mut c = ik(&["a"], "t");
        c.compress = true;
        let out = PoseEngine::new().compute(&one_bone_skel(c, 10.0, 3.0, 4.0), &PoseRequest::default());
        assert!(close(out.locals[1].scale_x, 0.5));
        let mut c = ik(&["a"], "t");
        c.stretch = true;
        let out = PoseEngine::new().compute(&one_bone_skel(c, 10.0, 3.0, 4.0), &PoseRequest::default());
        assert!(close(out.locals[1].scale_x, 1.0));
    }

    #[test]
    fn f_f02_ik_volume() {
        // 길이 10, 목표 (20, 0) → s = 2: scaleX = 2, scaleY = 1/2
        let mut c = ik(&["a"], "t");
        c.stretch = true;
        c.volume = true;
        let out = PoseEngine::new().compute(&one_bone_skel(c, 10.0, 20.0, 0.0), &PoseRequest::default());
        assert!(close(out.locals[1].scale_x, 2.0) && close(out.locals[1].scale_y, 0.5), "{:?}", out.locals[1]);
        // 월드 끝점 = (20, 0)
        let (x, y) = out.bones[1].local_to_world(10.0, 0.0);
        assert!(close(x, 20.0) && close(y, 0.0));
    }

    fn two_bone_skel(c: IkConstraint, tx: f32, ty: f32) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        s.bones.push(bone("p", "root", 0.0, 0.0, 10.0));
        s.bones.push(bone("c", "p", 10.0, 0.0, 10.0));
        s.bones.push(bone("t", "root", tx, ty, 0.0));
        s.constraints.push(Constraint::Ik(c));
        s
    }

    #[test]
    fn f_f01_two_bone_ik() {
        // l1 = l2 = 10, 목표 (10, 10): d = √200 = 14.1421
        // cosA = (100 + 200 − 100) / (2·10·14.1421) = 0.70711 → A = 45°, 목표 방향 45°
        // cosB = (100 + 100 − 200) / 200 = 0 → B = 90°
        // bendPositive: 부모 = 45 − 45 = 0°, 자식(로컬) = +(180 − 90) = 90° → 팔꿈치 (10, 0), 끝 (10, 10)
        let s = two_bone_skel(ik(&["p", "c"], "t"), 10.0, 10.0);
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!(close(out.locals[1].rotation, 0.0), "{}", out.locals[1].rotation);
        assert!(close(out.locals[2].rotation, 90.0), "{}", out.locals[2].rotation);
        let (x, y) = out.bones[2].local_to_world(10.0, 0.0);
        assert!(close(x, 10.0) && close(y, 10.0));
        // bendPositive = false: 부모 = 45 + 45 = 90°, 자식 = −90° → 팔꿈치 (0, 10), 끝 (10, 10)
        let mut c = ik(&["p", "c"], "t");
        c.bend_positive = false;
        let out = PoseEngine::new().compute(&two_bone_skel(c, 10.0, 10.0), &PoseRequest::default());
        assert!(close(out.locals[1].rotation, 90.0) && close(out.locals[2].rotation, -90.0), "{:?}", &out.locals[1..3]);
        assert!(close(out.bones[2].x, 0.0) && close(out.bones[2].y, 10.0));
        // 닿지 않는 목표 (30, 0): 일직선(0°, 0°). stretch면 부모 scaleX = 30/20 = 1.5, 끝이 (30, 0)
        let out = PoseEngine::new().compute(&two_bone_skel(ik(&["p", "c"], "t"), 30.0, 0.0), &PoseRequest::default());
        assert!(close(out.locals[1].rotation, 0.0) && close(out.locals[2].rotation, 0.0));
        let mut c = ik(&["p", "c"], "t");
        c.stretch = true;
        let out = PoseEngine::new().compute(&two_bone_skel(c, 30.0, 0.0), &PoseRequest::default());
        assert!(close(out.locals[1].scale_x, 1.5));
        let (x, _) = out.bones[2].local_to_world(10.0, 0.0);
        assert!(close(x, 30.0), "{x}");
        // softness 5, 목표 (20, 0): 시작점 15, d' = 15 + 5·(1 − e^(−1)) = 18.1606
        // 팔꿈치 각: cosA = (100 + 329.81 − 100) / (2·10·18.1606) = 0.90803 → A = 24.766°
        let mut c = ik(&["p", "c"], "t");
        c.softness = 5.0;
        let out = PoseEngine::new().compute(&two_bone_skel(c, 20.0, 0.0), &PoseRequest::default());
        assert!((out.locals[1].rotation + 24.766).abs() < 1e-2, "{}", out.locals[1].rotation);
        let (x, y) = out.bones[2].local_to_world(10.0, 0.0);
        assert!((x - 18.1606).abs() < 1e-2 && y.abs() < 1e-2, "{x} {y}");
    }
}

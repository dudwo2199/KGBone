//! 물리 제약 계산(고정 시간 간격).
//!
//! 설계(직접 정한 모델):
//! - 질점 두 개를 시뮬레이션한다: 본 원점(이동 x·y에 씀)과 본 끝(회전·scaleX·shearX에 씀).
//!   길이 0인 본은 끝 대신 본 X축 방향 referenceScale 거리의 점을 쓴다.
//! - 상태는 질점마다 "본 위치에서 벗어난 양"(offset)과 속도. 본이 움직이면 offset −= 이동량 × inertia
//!   (inertia 1이면 질점이 월드에 그대로 남아 뒤처진다).
//! - 한 단계(dt = 1/fps): 가속 = (−strength × offset − c × 속도) / mass + (wind, −gravity) × referenceScale/100,
//!   c = damping × √(strength × mass) (damping 1 ≈ 감쇠비 0.5). 속도 += 가속 × dt, |속도| ≤ limit, offset += 속도 × dt.
//! - Advance(dt)는 누적 시간을 고정 간격으로 나눈 단계 수만큼 진행하고 남은 시간은 다음으로 넘긴다.
//!   그 프레임의 본 이동량은 단계 수로 고르게 나눈다. 단계 수가 0이면 이동량을 다음 프레임으로 미룬다.
//! - 적용: 원점 offset × (x, y 비율) × mix만큼 월드 이동, 끝 offset이 만드는 각 × rotate × mix만큼 회전,
//!   같은 각 × shearX × mix만큼 X축만 회전, 끝까지 거리 비율 s로 scaleX를 1 + (s − 1) × scaleX × mix배,
//!   volume이면 scaleY를 그 배율로 나눈다.
//! - 정지 상태(본이 안 움직이고 바람·중력 0)면 offset·속도가 0에서 그대로라 결과가 바뀌지 않는다.
use super::bone_transform::{set_world, update_world};
use super::math::{rotate_world, rotate_world_x_axis, wrap_deg};
use super::{ConstraintSolver, PhysicsStep, SkeletonPose, SolveContext};
use crate::model::{Constraint, Skeleton};
use std::collections::HashMap;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Point {
    /// 지난 단계의 본 위치(월드).
    last: (f32, f32),
    offset: (f32, f32),
    vel: (f32, f32),
}

#[derive(Default, Clone, Debug, PartialEq)]
struct ConstraintState {
    initialized: bool,
    origin: Point,
    tip: Point,
    /// 고정 간격으로 나누고 남은 시간(초).
    remaining: f64,
}

/// 물리 제약별 속도·위치 상태(제약 이름 → 상태).
#[derive(Default, Clone, Debug)]
pub(crate) struct PhysicsState {
    map: HashMap<String, ConstraintState>,
}

impl PhysicsState {
    pub(crate) fn reset_all(&mut self) {
        self.map.clear();
    }
    /// 이름 `""`은 모든 물리 제약.
    pub(crate) fn reset_named(&mut self, name: &str) {
        if name.is_empty() {
            self.map.clear();
        } else {
            self.map.remove(name);
        }
    }
}

pub(crate) struct PhysicsSolver<'a> {
    pub state: &'a mut PhysicsState,
}

struct Params {
    inertia: f32,
    strength: f32,
    damping: f32,
    mass: f32,
    force: (f32, f32),
    limit: f32,
}

fn step_point(p: &mut Point, motion: (f32, f32), prm: &Params, dt: f32) {
    p.offset.0 -= motion.0 * prm.inertia;
    p.offset.1 -= motion.1 * prm.inertia;
    let mass = prm.mass.max(1e-4);
    let c = prm.damping * (prm.strength.max(0.0) * mass).sqrt();
    let ax = (-prm.strength * p.offset.0 - c * p.vel.0) / mass + prm.force.0;
    let ay = (-prm.strength * p.offset.1 - c * p.vel.1) / mass + prm.force.1;
    p.vel.0 += ax * dt;
    p.vel.1 += ay * dt;
    let speed = (p.vel.0 * p.vel.0 + p.vel.1 * p.vel.1).sqrt();
    if speed > prm.limit && speed > 0.0 {
        let k = prm.limit / speed;
        p.vel.0 *= k;
        p.vel.1 *= k;
    }
    p.offset.0 += p.vel.0 * dt;
    p.offset.1 += p.vel.1 * dt;
}

/// 질점 하나를 `steps`단계 진행. 이번 프레임 이동량(cur − last)은 단계마다 고르게 나눈다.
fn advance_point(p: &mut Point, cur: (f32, f32), steps: u32, prm: &Params, dt: f32) {
    if steps == 0 {
        return;
    }
    let total = (cur.0 - p.last.0, cur.1 - p.last.1);
    let per = (total.0 / steps as f32, total.1 / steps as f32);
    for _ in 0..steps {
        step_point(p, per, prm, dt);
    }
    p.last = cur;
}

impl ConstraintSolver for PhysicsSolver<'_> {
    fn apply(&mut self, skel: &Skeleton, index: usize, pose: &mut SkeletonPose, ctx: &mut SolveContext) {
        let Constraint::Physics(c) = &skel.constraints[index] else { return };
        let Some(bi) = skel.bone_index(&c.bone) else { return };
        let v = pose.constraint_values[index].clone();
        let rs = skel.reference_scale / 100.0;
        let prm = Params { inertia: v[0], strength: v[1], damping: v[2], mass: v[3], force: (v[4] * rs, -v[5] * rs), limit: c.limit.max(0.0) };
        let mix = v[6];
        let w = pose.world[bi];
        let len = skel.bones[bi].length;
        let tip_local = if len > 0.0 { len } else { skel.reference_scale.max(1.0) };
        let origin = (w.x, w.y);
        let tip = w.local_to_world(tip_local, 0.0);

        let st = self.state.map.entry(c.name.clone()).or_default();
        if !st.initialized {
            *st = ConstraintState { initialized: true, origin: Point { last: origin, ..Default::default() }, tip: Point { last: tip, ..Default::default() }, remaining: 0.0 };
        }
        if let PhysicsStep::Advance(dt) = ctx.physics {
            let fps = if c.fps > 0.0 { c.fps as f64 } else { 60.0 };
            st.remaining += dt.max(0.0) as f64;
            let steps = (st.remaining * fps + 1e-6).floor().max(0.0);
            st.remaining = (st.remaining - steps / fps).max(0.0);
            let step_dt = (1.0 / fps) as f32;
            advance_point(&mut st.origin, origin, steps as u32, &prm, step_dt);
            advance_point(&mut st.tip, tip, steps as u32, &prm, step_dt);
        }
        let (o, t) = (st.origin.offset, st.tip.offset);
        if (o.0 == 0.0 && o.1 == 0.0 && t.0 == 0.0 && t.1 == 0.0) || mix == 0.0 {
            return;
        }
        let mut nw = w;
        nw.x += o.0 * c.x * mix;
        nw.y += o.1 * c.y * mix;
        let (bx, by) = (tip.0 - origin.0, tip.1 - origin.1);
        let (sx, sy) = (bx + t.0, by + t.1);
        let angle = wrap_deg(sy.atan2(sx).to_degrees() - by.atan2(bx).to_degrees());
        rotate_world(&mut nw, angle * c.rotate * mix);
        rotate_world_x_axis(&mut nw, angle * c.shear_x * mix);
        let base = (bx * bx + by * by).sqrt();
        if base > 1e-6 && c.scale_x != 0.0 {
            let s = (sx * sx + sy * sy).sqrt() / base;
            let f = 1.0 + (s - 1.0) * c.scale_x * mix;
            nw.a *= f;
            nw.c *= f;
            if c.volume && f.abs() > 1e-6 {
                nw.b /= f;
                nw.d /= f;
            }
        }
        set_world(ctx.parents, pose, bi, &nw);
        update_world(skel, ctx.parents, pose, bi);
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Bone, Constraint, PhysicsConstraint, Skeleton};
    use crate::pose::{PhysicsStep, PoseEngine, PoseOutput, PoseRequest};

    fn skel(gravity: f32) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        let mut b = Bone::new("tail", Some("root".into()));
        b.length = 50.0;
        s.bones.push(b);
        s.constraints.push(Constraint::Physics(PhysicsConstraint {
            name: "phys".into(),
            skin_required: false,
            bone: "tail".into(),
            x: 1.0,
            y: 1.0,
            rotate: 1.0,
            scale_x: 0.5,
            shear_x: 0.0,
            limit: 5000.0,
            fps: 60.0,
            inertia: 1.0,
            strength: 100.0,
            damping: 1.0,
            mass: 1.0,
            wind: 0.0,
            gravity,
            mix: 1.0,
            volume: true,
        }));
        s
    }

    fn run(s: &Skeleton, engine: &mut PoseEngine, steps: usize) -> Vec<PoseOutput> {
        (0..steps).map(|_| engine.compute(s, &PoseRequest { physics: PhysicsStep::Advance(1.0 / 60.0), ..Default::default() })).collect()
    }

    fn bits(o: &PoseOutput) -> Vec<u32> {
        o.bones.iter().flat_map(|b| [b.a, b.b, b.c, b.d, b.x, b.y]).map(f32::to_bits).collect()
    }

    #[test]
    fn f_f05_physics_deterministic_and_still() {
        // 중력 30: 1/60초 간격 120단계를 두 번 돌리면 비트 단위로 같다.
        let s = skel(30.0);
        let a = run(&s, &mut PoseEngine::new(), 120);
        let b = run(&s, &mut PoseEngine::new(), 120);
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(bits(x), bits(y));
        }
        // 중력이 있으면 실제로 움직인다(꼬리가 아래로 처짐).
        assert!(a[119].bones[1].y < -0.01, "{:?}", a[119].bones[1]);
        // 정지 상태(중력 0, 본 고정): 셋업 그대로
        let still = skel(0.0);
        let out = run(&still, &mut PoseEngine::new(), 120);
        let setup = PoseEngine::new().compute(&still, &PoseRequest::default());
        assert_eq!(bits(&out[119]), bits(&setup));
        // Reset 후 다시 돌리면 처음과 같다.
        let mut e = PoseEngine::new();
        let first = run(&s, &mut e, 120);
        e.compute(&s, &PoseRequest { physics: PhysicsStep::Reset, ..Default::default() });
        let again = run(&s, &mut e, 120);
        assert_eq!(bits(&first[119]), bits(&again[119]));
        e.reset_physics();
        let again2 = run(&s, &mut e, 120);
        assert_eq!(bits(&first[60]), bits(&again2[60]));
        // 한 번에 2초(120단계)를 진행해도 단계 수는 같다(이동량 0이므로 결과도 같음).
        let mut e2 = PoseEngine::new();
        e2.compute(&s, &PoseRequest::default());
        let big = e2.compute(&s, &PoseRequest { physics: PhysicsStep::Advance(2.0), ..Default::default() });
        for (p, q) in bits(&big).iter().zip(bits(&first[119]).iter()) {
            assert!((f32::from_bits(*p) - f32::from_bits(*q)).abs() < 1e-3);
        }
    }
}

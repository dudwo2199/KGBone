//! 슬라이더 제약(4.3): 애니메이션을 정해진 시점·혼합으로 적용.
//!
//! 시점: bone이 없으면 제약 값 time(키 반영). bone이 있으면
//! `(본 property 값 − propertyFrom) / (propertyTo − propertyFrom) × 애니메이션 길이` (local이면 로컬 값, 아니면 월드 값).
//! loop면 길이로 나눈 나머지, 아니면 [0, 길이]로 자른다. 그 애니메이션을 alpha = mix, additive로 현재 포즈에 적용하고
//! 모든 본의 월드를 다시 계산한다. 적용하는 애니메이션 안의 자기 자신 제약 키는 건너뛴다(순환 방지).
use super::bone_transform::update_world;
use super::transform::bone_property;
use super::{ConstraintSolver, SkeletonPose, SolveContext};
use crate::anim::{apply_scoped, ApplyOptions, ApplyScope};
use crate::model::{Constraint, Skeleton};

pub(crate) struct SliderSolver;

impl ConstraintSolver for SliderSolver {
    fn apply(&mut self, skel: &Skeleton, index: usize, pose: &mut SkeletonPose, ctx: &mut SolveContext) {
        let Constraint::Slider(c) = &skel.constraints[index] else { return };
        let Some(anim) = skel.animation(&c.animation) else { return };
        let duration = anim.duration();
        let (time_value, mix) = (pose.constraint_values[index][0], pose.constraint_values[index][1]);
        let mut t = match c.bone.as_deref().and_then(|b| skel.bone_index(b)) {
            Some(bi) => {
                let v = bone_property(pose, bi, c.property, c.local);
                let span = c.property_to - c.property_from;
                if span.abs() < 1e-9 {
                    0.0
                } else {
                    (v - c.property_from) / span * duration
                }
            }
            None => time_value,
        };
        if c.loop_ {
            t = if duration > 0.0 { t.rem_euclid(duration) } else { 0.0 };
        } else {
            t = t.clamp(0.0, duration.max(0.0));
        }
        let scope = ApplyScope { skin: ctx.skin, all_skins: false, skip_constraint: Some(index) };
        apply_scoped(skel, anim, t, pose, ApplyOptions { alpha: mix, additive: c.additive }, scope);
        update_world(skel, ctx.parents, pose, 0);
    }
}

#[cfg(test)]
mod tests {
    use crate::anim::{apply_animation, ApplyOptions};
    use crate::model::{Animation, Bone, BoneProperty, Constraint, Key, Skeleton, SliderConstraint};
    use crate::pose::{compute_world, PoseEngine, PoseRequest, SkeletonPose};

    fn skel(bone_driven: bool, dial_y: f32) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        let mut arm = Bone::new("arm", Some("root".into()));
        arm.length = 10.0;
        arm.rotation = 10.0;
        s.bones.push(arm);
        let mut dial = Bone::new("dial", Some("root".into()));
        dial.y = dial_y;
        s.bones.push(dial);
        // 0초 회전 +0, 2초 회전 +80 (선형)
        let mut a = Animation::new("swing");
        a.bones.entry("arm").insert("rotate", vec![Key::new(0.0, vec![0.0]), Key::new(2.0, vec![80.0])]);
        s.animations.push(a);
        s.constraints.push(Constraint::Slider(SliderConstraint {
            name: "sl".into(),
            skin_required: false,
            animation: "swing".into(),
            time: 0.5,
            loop_: false,
            additive: false,
            mix: 0.75,
            bone: if bone_driven { Some("dial".into()) } else { None },
            property: BoneProperty::Y,
            property_from: 0.0,
            property_to: 100.0,
            local: false,
        }));
        s
    }

    #[test]
    fn f_f06_slider_equals_animation_with_mix() {
        // 시점 0.5: 애니메이션 값 +20 → 목표 30. mix 0.75: 10 + (30 − 10) × 0.75 = 25
        let s = skel(false, 0.0);
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!((out.locals[1].rotation - 25.0).abs() < 1e-3, "{}", out.locals[1].rotation);
        // 같은 결과 = 애니메이션을 t에 mix로 적용한 결과
        let mut pose = SkeletonPose::setup(&s);
        apply_animation(&s, &s.animations[0], 0.5, &mut pose, ApplyOptions { alpha: 0.75, additive: false });
        let w = compute_world(&s, &pose.locals, &pose.inherits);
        assert!((out.bones[1].rotation_x() - w[1].rotation_x()).abs() < 1e-3);
        // 본 구동: dial 월드 y = 50 → (50 − 0)/100 × 2초 = 1초 → +40 → 목표 50 → 10 + 40 × 0.75 = 40
        let out = PoseEngine::new().compute(&skel(true, 50.0), &PoseRequest::default());
        assert!((out.locals[1].rotation - 40.0).abs() < 1e-3);
        // 범위 밖(y = 300) → 길이로 잘림(2초) → +80 → 10 + 80 × 0.75 = 70
        let out = PoseEngine::new().compute(&skel(true, 300.0), &PoseRequest::default());
        assert!((out.locals[1].rotation - 70.0).abs() < 1e-3);
        // loop: y = 300 → 6초 → 6 mod 2 = 0 → +0 → 10
        let mut s = skel(true, 300.0);
        if let Constraint::Slider(c) = &mut s.constraints[0] {
            c.loop_ = true;
        }
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!((out.locals[1].rotation - 10.0).abs() < 1e-3);
        // additive: 시점 0.5 → 차이 +20 × 0.75 = 15 → 25 (셋업 10에서)
        let mut s = skel(false, 0.0);
        if let Constraint::Slider(c) = &mut s.constraints[0] {
            c.additive = true;
        }
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!((out.locals[1].rotation - 25.0).abs() < 1e-3);
        // 슬라이더 time 키(G06): 애니메이션 "drive"의 sl/time 키 1.5 → +60 → 10 + 60 × 0.75 = 55
        let mut s = skel(false, 0.0);
        let mut d = Animation::new("drive");
        d.constraints.entry("sl").insert("time", vec![Key::new(0.0, vec![1.5])]);
        s.animations.push(d);
        let out = PoseEngine::new().compute(&s, &PoseRequest { animation: Some("drive".into()), ..Default::default() });
        assert!((out.locals[1].rotation - 55.0).abs() < 1e-3);
    }
}

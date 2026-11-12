//! 트랜스폼 제약(4.3 속성 매핑).
//!
//! FORMAT.md 3.3: `d = source.from − from.offset`, `값 = to.offset + d × to.scale`, clamp면 [to.offset, to.max]로 자름,
//! 비가산 `대상 += (값 − 대상) × mix`, 가산 `대상 += 값 × mix`.
//! - 원본 값: localSource면 원본 본의 로컬 값, 아니면 월드 값(회전 = X축 각, x·y = 위치, scaleX·Y = 축 길이, shearY = 축 사이 각 − 90).
//! - 받는 쪽: localTarget면 로컬 값을 바꾸고, 아니면 월드 값을 바꾼 뒤 부모 기준 로컬로 되돌린다.
//! - 비가산 회전은 차이를 (−180, 180]으로 감아 가까운 쪽으로 섞는다.
use super::bone_transform::{set_world, update_world};
use super::math::{world_from_parts, world_shear_y, wrap_deg};
use super::{ConstraintSolver, SkeletonPose, SolveContext, WorldBone};
use crate::model::{BoneProperty, Constraint, Skeleton, Transform};

pub(crate) struct TransformSolver;

/// 속성 6개 값 묶음.
#[derive(Clone, Copy, Debug)]
struct Props([f32; 6]);

fn slot(p: BoneProperty) -> usize {
    match p {
        BoneProperty::Rotate => 0,
        BoneProperty::X => 1,
        BoneProperty::Y => 2,
        BoneProperty::ScaleX => 3,
        BoneProperty::ScaleY => 4,
        BoneProperty::ShearY => 5,
    }
}

impl Props {
    fn of_local(t: &Transform) -> Props {
        Props([t.rotation, t.x, t.y, t.scale_x, t.scale_y, t.shear_y])
    }
    fn of_world(w: &WorldBone) -> Props {
        Props([w.rotation_x(), w.x, w.y, w.scale_x(), w.scale_y(), world_shear_y(w)])
    }
}

/// 본 하나의 속성 값(로컬 또는 월드). 슬라이더도 쓴다.
pub(crate) fn bone_property(pose: &SkeletonPose, bone: usize, p: BoneProperty, local: bool) -> f32 {
    let props = if local { Props::of_local(&pose.locals[bone]) } else { Props::of_world(&pose.world[bone]) };
    props.0[slot(p)]
}

impl ConstraintSolver for TransformSolver {
    fn apply(&mut self, skel: &Skeleton, index: usize, pose: &mut SkeletonPose, ctx: &mut SolveContext) {
        let Constraint::Transform(c) = &skel.constraints[index] else { return };
        let Some(si) = skel.bone_index(&c.source) else { return };
        let mixes = pose.constraint_values[index].clone();
        let mix_of = |p: BoneProperty| mixes.get(slot(p)).copied().unwrap_or_else(|| c.mix_of(p));
        let mappings = c.effective_properties();
        for name in &c.bones {
            let Some(bi) = skel.bone_index(name) else { continue };
            // 원본 값은 대상마다 다시 읽는다(앞 대상이 원본의 조상일 수 있음).
            let src = if c.local_source { Props::of_local(&pose.locals[si]) } else { Props::of_world(&pose.world[si]) };
            let mut tgt = if c.local_target { Props::of_local(&pose.locals[bi]) } else { Props::of_world(&pose.world[bi]) };
            for m in &mappings {
                let d = src.0[slot(m.from)] - m.offset;
                for to in &m.to {
                    let mut v = to.offset + d * to.scale;
                    if c.clamp {
                        let (lo, hi) = if to.offset <= to.max { (to.offset, to.max) } else { (to.max, to.offset) };
                        v = v.clamp(lo, hi);
                    }
                    let k = slot(to.property);
                    let mix = mix_of(to.property);
                    if c.additive {
                        tgt.0[k] += v * mix;
                    } else if to.property == BoneProperty::Rotate {
                        tgt.0[k] += wrap_deg(v - tgt.0[k]) * mix;
                    } else {
                        tgt.0[k] += (v - tgt.0[k]) * mix;
                    }
                }
            }
            let t = tgt.0;
            if c.local_target {
                let l = &mut pose.locals[bi];
                l.rotation = t[0];
                l.x = t[1];
                l.y = t[2];
                l.scale_x = t[3];
                l.scale_y = t[4];
                l.shear_y = t[5];
            } else {
                let w = world_from_parts(t[0], t[3], t[4], t[5], t[1], t[2]);
                set_world(ctx.parents, pose, bi, &w);
            }
            update_world(skel, ctx.parents, pose, bi);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{Bone, BoneProperty, Constraint, FromProperty, Skeleton, ToProperty, TransformConstraint};
    use crate::pose::{PoseEngine, PoseRequest};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    /// 다이얼(원본) x → 대상 회전, scale 2, max 90.
    fn skel(local: bool, clamp: bool, dial_x: f32) -> Skeleton {
        let mut s = Skeleton::new_with_root();
        // root를 (100, 0)으로 옮겨 월드와 로컬 값이 다르게 한다.
        s.bones[0].x = 100.0;
        let mut dial = Bone::new("dial", Some("root".into()));
        dial.x = dial_x;
        let mut gem = Bone::new("gem", Some("root".into()));
        gem.rotation = 10.0;
        s.bones.push(dial);
        s.bones.push(gem);
        s.constraints.push(Constraint::Transform(TransformConstraint {
            name: "tc".into(),
            skin_required: false,
            bones: vec!["gem".into()],
            source: "dial".into(),
            local_source: local,
            local_target: local,
            additive: false,
            clamp,
            mix_rotate: 1.0,
            mix_x: 0.0,
            mix_y: 0.0,
            mix_scale_x: 0.0,
            mix_scale_y: 0.0,
            mix_shear_y: 0.0,
            properties: vec![FromProperty {
                from: BoneProperty::X,
                offset: 0.0,
                to: vec![ToProperty { property: BoneProperty::Rotate, offset: 0.0, max: 90.0, scale: 2.0 }],
            }],
        }));
        s
    }

    #[test]
    fn f_f03_translate_x_to_rotate() {
        // 로컬: 다이얼 로컬 x = 30 → 값 = 0 + 30 × 2 = 60. 대상 로컬 회전 10 → 10 + (60 − 10) × 1 = 60
        let out = PoseEngine::new().compute(&skel(true, false, 30.0), &PoseRequest::default());
        assert!(close(out.locals[2].rotation, 60.0), "{}", out.locals[2].rotation);
        // clamp: x = 60 → 120 → [0, 90]로 잘려 90
        let out = PoseEngine::new().compute(&skel(true, true, 60.0), &PoseRequest::default());
        assert!(close(out.locals[2].rotation, 90.0));
        // 월드: 다이얼 월드 x = 100 + 0.2 = 100.2 → 값 = 200.4, clamp로 90. 월드 회전 90 → 로컬 회전 90 (root 회전 0)
        let out = PoseEngine::new().compute(&skel(false, true, 0.2), &PoseRequest::default());
        assert!(close(out.bones[2].rotation_x(), 90.0), "{}", out.bones[2].rotation_x());
        // 월드, clamp 없음: 다이얼 월드 x = 100 − 80 = 20 → 값 40 → 월드 회전 40
        let out = PoseEngine::new().compute(&skel(false, false, -80.0), &PoseRequest::default());
        assert!(close(out.bones[2].rotation_x(), 40.0));
        assert!(close(out.bones[2].x, 100.0) && close(out.bones[2].y, 0.0));
    }

    #[test]
    fn f_f03_mix_and_additive_and_default_mapping() {
        // mixRotate 0.5: 10 + (60 − 10) × 0.5 = 35
        let mut s = skel(true, false, 30.0);
        if let Constraint::Transform(c) = &mut s.constraints[0] {
            c.mix_rotate = 0.5;
        }
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!(close(out.locals[2].rotation, 35.0));
        // 가산: 10 + 60 × 0.5 = 40
        if let Constraint::Transform(c) = &mut s.constraints[0] {
            c.additive = true;
        }
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!(close(out.locals[2].rotation, 40.0));
        // 기본 매핑(월드, mix 모두 1): 대상 월드 = 원본 월드 (위치 (100 + 30, 0))
        if let Constraint::Transform(c) = &mut s.constraints[0] {
            c.additive = false;
            c.properties.clear();
            c.local_source = false;
            c.local_target = false;
            c.mix_x = 1.0;
            c.mix_y = 1.0;
            c.mix_rotate = 1.0;
        }
        let out = PoseEngine::new().compute(&s, &PoseRequest::default());
        assert!(close(out.bones[2].x, 130.0) && close(out.bones[2].rotation_x(), 0.0), "{:?}", out.bones[2]);
    }
}

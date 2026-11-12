//! 본 타임라인: rotate, translate(x/y), scale(x/y), shear(x/y), inherit.
use super::apply::Mix;
use super::curve::{step_index, value_at};
use crate::model::{Animation, Inherit, Skeleton};
use crate::pose::SkeletonPose;

pub(super) fn apply(skel: &Skeleton, anim: &Animation, t: f32, pose: &mut SkeletonPose, mix: Mix) {
    for (bone_name, timelines) in anim.bones.iter() {
        let Some(i) = skel.bone_index(bone_name) else { continue };
        let setup = skel.bones[i].local();
        let cur = &mut pose.locals[i];
        for (name, keys) in timelines.iter() {
            if keys.is_empty() {
                continue;
            }
            let v = |ch: usize| value_at(keys, ch, t);
            match name.as_str() {
                "rotate" => cur.rotation = mix.blend_angle(cur.rotation, setup.rotation + v(0), setup.rotation),
                "translate" => {
                    cur.x = mix.blend(cur.x, setup.x + v(0), setup.x);
                    cur.y = mix.blend(cur.y, setup.y + v(1), setup.y);
                }
                "translatex" => cur.x = mix.blend(cur.x, setup.x + v(0), setup.x),
                "translatey" => cur.y = mix.blend(cur.y, setup.y + v(0), setup.y),
                "scale" => {
                    cur.scale_x = mix.blend(cur.scale_x, setup.scale_x * v(0), setup.scale_x);
                    cur.scale_y = mix.blend(cur.scale_y, setup.scale_y * v(1), setup.scale_y);
                }
                "scalex" => cur.scale_x = mix.blend(cur.scale_x, setup.scale_x * v(0), setup.scale_x),
                "scaley" => cur.scale_y = mix.blend(cur.scale_y, setup.scale_y * v(0), setup.scale_y),
                "shear" => {
                    cur.shear_x = mix.blend(cur.shear_x, setup.shear_x + v(0), setup.shear_x);
                    cur.shear_y = mix.blend(cur.shear_y, setup.shear_y + v(1), setup.shear_y);
                }
                "shearx" => cur.shear_x = mix.blend(cur.shear_x, setup.shear_x + v(0), setup.shear_x),
                "sheary" => cur.shear_y = mix.blend(cur.shear_y, setup.shear_y + v(0), setup.shear_y),
                "inherit" => {
                    if mix.discrete() {
                        if let Some(k) = step_index(keys, t) {
                            if let Some(inh) = keys[k].s.as_deref().and_then(Inherit::parse) {
                                pose.inherits[i] = inh;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

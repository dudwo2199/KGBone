//! 제약 값 타임라인 → `pose.constraint_values` (배열 모양은 `SkeletonPose::constraint_values` 설명).
//! 이름 `""`의 키는 모든 물리 제약에 적용된다. 물리 `reset` 키는 엔진이 구간으로 처리한다.
use super::apply::Mix;
use super::curve::{step_index, value_at};
use crate::model::{Animation, Constraint, Key, Skeleton};
use crate::pose::{constraint_setup_values, SkeletonPose};

pub(super) fn apply(skel: &Skeleton, anim: &Animation, t: f32, pose: &mut SkeletonPose, mix: Mix, skip: Option<usize>) {
    for (cname, timelines) in anim.constraints.iter() {
        let targets: Vec<usize> = if cname.is_empty() {
            skel.constraints.iter().enumerate().filter(|(_, c)| matches!(c, Constraint::Physics(_))).map(|(i, _)| i).collect()
        } else {
            skel.constraint_index(cname).into_iter().collect()
        };
        for ci in targets {
            if Some(ci) == skip {
                continue;
            }
            let c = &skel.constraints[ci];
            let setup = constraint_setup_values(c);
            for (name, keys) in timelines.iter() {
                if keys.is_empty() {
                    continue;
                }
                // (타임라인 채널 → 값 배열 자리, 연속인가)
                let slots: Vec<(usize, bool)> = match (c, name.as_str()) {
                    (Constraint::Ik(_), "ik") => vec![(0, true), (1, true), (2, false), (3, false), (4, false)],
                    (Constraint::Transform(_), "transform") => (0..6).map(|i| (i, true)).collect(),
                    (Constraint::Path(_), "position") => vec![(0, true)],
                    (Constraint::Path(_), "spacing") => vec![(1, true)],
                    (Constraint::Path(_), "mix") => vec![(2, true), (3, true), (4, true)],
                    (Constraint::Physics(_), n) => match ["inertia", "strength", "damping", "mass", "wind", "gravity", "mix"].iter().position(|x| *x == n) {
                        Some(p) => vec![(p, true)],
                        None => vec![],
                    },
                    (Constraint::Slider(_), "time") => vec![(0, true)],
                    (Constraint::Slider(_), "mix") => vec![(1, true)],
                    _ => vec![],
                };
                let vals = &mut pose.constraint_values[ci];
                for (ch, (dst, continuous)) in slots.into_iter().enumerate() {
                    if dst >= vals.len() {
                        continue;
                    }
                    if continuous {
                        vals[dst] = mix.blend(vals[dst], value_at(keys, ch, t), setup[dst]);
                    } else if mix.discrete() {
                        vals[dst] = stepped(keys, ch, t);
                    }
                }
            }
        }
    }
}

fn stepped(keys: &[Key], ch: usize, t: f32) -> f32 {
    step_index(keys, t).and_then(|k| keys[k].v.get(ch).copied()).unwrap_or(0.0)
}

//! 슬롯 타임라인(attachment, rgba, rgb, alpha, rgba2, rgb2)과 그리기 순서 키.
use super::apply::Mix;
use super::curve::{step_index, value_at};
use crate::model::{Animation, DrawOrderKey, Skeleton};
use crate::pose::SkeletonPose;

pub(super) fn apply(skel: &Skeleton, anim: &Animation, t: f32, pose: &mut SkeletonPose, mix: Mix) {
    for (slot_name, timelines) in anim.slots.iter() {
        let Some(i) = skel.slot_index(slot_name) else { continue };
        let setup_c = skel.slots[i].color.to_f32();
        let setup_d = skel.slots[i].dark.map(|d| d.0.to_f32()).unwrap_or([0.0, 0.0, 0.0, 1.0]);
        for (name, keys) in timelines.iter() {
            if keys.is_empty() {
                continue;
            }
            let v = |ch: usize| value_at(keys, ch, t);
            let color = &mut pose.slot_colors[i];
            let set_c = |color: &mut [f32; 4], ch: usize, val: f32| color[ch] = mix.blend(color[ch], val, setup_c[ch]).clamp(0.0, 1.0);
            let set_dark = |pose_dark: &mut Option<[f32; 3]>, r: f32, g: f32, b: f32| {
                let mut d = pose_dark.unwrap_or([setup_d[0], setup_d[1], setup_d[2]]);
                for (ch, val) in [r, g, b].into_iter().enumerate() {
                    d[ch] = mix.blend(d[ch], val, setup_d[ch]).clamp(0.0, 1.0);
                }
                *pose_dark = Some(d);
            };
            match name.as_str() {
                "rgba" => (0..4).for_each(|ch| set_c(color, ch, v(ch))),
                "rgb" => (0..3).for_each(|ch| set_c(color, ch, v(ch))),
                "alpha" => set_c(color, 3, v(0)),
                "rgba2" => {
                    (0..4).for_each(|ch| set_c(color, ch, v(ch)));
                    set_dark(&mut pose.slot_darks[i], v(4), v(5), v(6));
                }
                "rgb2" => {
                    (0..3).for_each(|ch| set_c(color, ch, v(ch)));
                    set_dark(&mut pose.slot_darks[i], v(3), v(4), v(5));
                }
                "attachment" => {
                    if mix.discrete() {
                        if let Some(k) = step_index(keys, t) {
                            pose.slot_attachments[i] = keys[k].s.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if mix.discrete() && !anim.draw_order.is_empty() {
        let k = last_key_at(&anim.draw_order, t);
        pose.draw_order = draw_order_of(skel, &anim.draw_order[k]);
    }
}

/// 시점 이전(같음 포함) 마지막 그리기 순서 키. 첫 키 앞이면 첫 키.
fn last_key_at(keys: &[DrawOrderKey], t: f32) -> usize {
    keys.iter().rposition(|k| k.time <= t).unwrap_or(0)
}

/// 키 하나의 그리기 순서: 이름이 적힌 슬롯은 셋업 자리 + offset으로 옮기고, 나머지는 셋업 순서대로 빈 자리를 채운다.
/// 범위를 벗어나거나 자리가 겹치는 항목은 무시한다.
pub(crate) fn draw_order_of(skel: &Skeleton, key: &DrawOrderKey) -> Vec<usize> {
    let n = skel.slots.len();
    let mut placed: Vec<Option<usize>> = vec![None; n];
    let mut moved = vec![false; n];
    for o in &key.offsets {
        let Some(i) = skel.slot_index(&o.slot) else { continue };
        let target = i as i64 + o.offset as i64;
        if target < 0 || target >= n as i64 || placed[target as usize].is_some() || moved[i] {
            continue;
        }
        placed[target as usize] = Some(i);
        moved[i] = true;
    }
    let mut rest = (0..n).filter(|i| !moved[*i]);
    placed.into_iter().map(|p| p.unwrap_or_else(|| rest.next().unwrap_or(0))).collect()
}

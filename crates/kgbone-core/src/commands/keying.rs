//! 애니메이트 모드 자동 키: 편집 명령이 셋업 값 대신 현재 애니메이션의 현재 시점에 키를 만든다.
//!
//! - 키 목록은 항상 시간순. 같은 시점(|Δt| < [`TIME_EPS`])의 키가 있으면 새로 만들지 않고 값을 고친다.
//! - 본 타임라인 값은 셋업 기준 상대값(FORMAT.md 3.7): rotate·translate·shear는 차이, scale은 비율.
use super::CommandContext;
use crate::anim::curve;
use crate::json::JValue as Value;
use crate::model::{Animation, Bone, Color, Constraint, EditorMode, Key, Skeleton, Slot, Timelines, Transform};
use crate::json::ToJson;

/// 같은 시점으로 보는 시간 차이.
pub(crate) const TIME_EPS: f32 = 1e-4;

/// 애니메이트 모드이고 현재 애니메이션이 실제로 있으면 (애니메이션 이름, 현재 시점).
pub(super) fn animate_target(ctx: &CommandContext) -> Option<(String, f32)> {
    let e = &ctx.project.editor;
    if e.mode != EditorMode::Animate {
        return None;
    }
    let name = e.active_animation.as_ref()?;
    ctx.project.skeleton.animation(name)?;
    Some((name.clone(), e.time))
}

pub(super) fn find_key(keys: &[Key], time: f32) -> Option<usize> {
    keys.iter().position(|k| (k.time - time).abs() < TIME_EPS)
}

/// 시간순 자리에 키를 넣는다. 같은 시점 키가 있으면 통째로 바꾼다.
pub(super) fn insert_key(keys: &mut Vec<Key>, key: Key) {
    if let Some(i) = find_key(keys, key.time) {
        keys[i] = key;
        return;
    }
    let i = keys.iter().position(|k| k.time > key.time).unwrap_or(keys.len());
    keys.insert(i, key);
}

/// 값만 바꾼다(같은 시점 키가 있으면 곡선은 그대로 둔다).
pub(super) fn set_key_value(keys: &mut Vec<Key>, time: f32, v: Vec<f32>, s: Option<String>) {
    match find_key(keys, time) {
        Some(i) => {
            keys[i].v = v;
            keys[i].s = s;
        }
        None => insert_key(keys, Key { time, v, s, ..Default::default() }),
    }
}

/// `time` 시점의 `channel`번째 값. 키가 없으면 `default`, 첫 키 앞·마지막 키 뒤는 그 키 값, 사이는 보간.
pub(super) fn sample(keys: &[Key], channel: usize, time: f32, default: f32) -> f32 {
    let val = |k: &Key| k.v.get(channel).copied().unwrap_or(default);
    let (Some(first), Some(last)) = (keys.first(), keys.last()) else { return default };
    if let Some(i) = find_key(keys, time) {
        return val(&keys[i]);
    }
    if time <= first.time {
        return val(first);
    }
    if time >= last.time {
        return val(last);
    }
    curve::value_at(keys, channel, time)
}

/// `time` 시점의 문자열 값(그 시점 이전 마지막 키). 키가 없으면 바깥 None, 키가 "비움"이면 Some(None).
pub(super) fn sample_str(keys: &[Key], time: f32) -> Option<Option<String>> {
    keys.iter().rev().find(|k| k.time <= time + TIME_EPS).or(keys.first()).map(|k| k.s.clone())
}

/// 셋업 기준 상대 비율(셋업이 0이면 값 그대로).
fn ratio(value: f32, setup: f32) -> f32 {
    if setup == 0.0 {
        value
    } else {
        value / setup
    }
}

/// 본 하나의 원하는 절대값(주어진 것만).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct BoneEdit {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub rotation: Option<f32>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub shear_x: Option<f32>,
    pub shear_y: Option<f32>,
}

/// 본 4개 타임라인의 현재 상대값 [rotate, translate, scale, shear].
pub(super) fn current_bone_values(anim: &Animation, bone: &str, time: f32) -> [Vec<f32>; 4] {
    let empty = vec![];
    let tl = anim.bones.get(bone);
    let keys = |n: &str| tl.and_then(|t| t.get(n)).unwrap_or(&empty);
    let two = |n: &str, d: f32| vec![sample(keys(n), 0, time, d), sample(keys(n), 1, time, d)];
    [vec![sample(keys("rotate"), 0, time, 0.0)], two("translate", 0.0), two("scale", 1.0), two("shear", 0.0)]
}

/// 절대 로컬 값 `l`을 셋업 기준 상대값 [rotate, translate, scale, shear]로.
pub(super) fn relative_values(setup: &Bone, l: &Transform) -> [Vec<f32>; 4] {
    [
        vec![l.rotation - setup.rotation],
        vec![l.x - setup.x, l.y - setup.y],
        vec![ratio(l.scale_x, setup.scale_x), ratio(l.scale_y, setup.scale_y)],
        vec![l.shear_x - setup.shear_x, l.shear_y - setup.shear_y],
    ]
}

/// 본 4개 타임라인에 상대값 키를 넣는다.
pub(super) fn key_bone_values(anim: &mut Animation, bone: &str, time: f32, values: [Vec<f32>; 4]) {
    let tl = anim.bones.entry(bone);
    for (name, v) in ["rotate", "translate", "scale", "shear"].into_iter().zip(values) {
        set_key_value(tl.entry(name), time, v, None);
    }
}

/// 원하는 절대값 `edit`를 셋업 기준 상대값 키로 넣는다. 주어진 속성의 타임라인만 키를 만든다.
pub(super) fn key_bone_edit(anim: &mut Animation, setup: &Bone, time: f32, edit: &BoneEdit) {
    let cur = current_bone_values(anim, &setup.name, time);
    let tl: &mut Timelines = anim.bones.entry(&setup.name);
    if let Some(r) = edit.rotation {
        set_key_value(tl.entry("rotate"), time, vec![r - setup.rotation], None);
    }
    if edit.x.is_some() || edit.y.is_some() {
        let v = vec![edit.x.map_or(cur[1][0], |x| x - setup.x), edit.y.map_or(cur[1][1], |y| y - setup.y)];
        set_key_value(tl.entry("translate"), time, v, None);
    }
    if edit.scale_x.is_some() || edit.scale_y.is_some() {
        let v = vec![
            edit.scale_x.map_or(cur[2][0], |s| ratio(s, setup.scale_x)),
            edit.scale_y.map_or(cur[2][1], |s| ratio(s, setup.scale_y)),
        ];
        set_key_value(tl.entry("scale"), time, v, None);
    }
    if edit.shear_x.is_some() || edit.shear_y.is_some() {
        let v = vec![edit.shear_x.map_or(cur[3][0], |s| s - setup.shear_x), edit.shear_y.map_or(cur[3][1], |s| s - setup.shear_y)];
        set_key_value(tl.entry("shear"), time, v, None);
    }
}

/// 슬롯의 현재 색 [r, g, b, a] (rgba 키 → 셋업 색).
pub(super) fn current_slot_color(anim: &Animation, slot: &Slot, time: f32) -> Vec<f32> {
    let setup = slot.color.to_f32();
    let keys = anim.slots.get(&slot.name).and_then(|t| t.get("rgba"));
    (0..4).map(|c| keys.map_or(setup[c], |k| sample(k, c, time, setup[c]))).collect()
}

/// 슬롯 색 키(rgba).
pub(super) fn key_slot_color(anim: &mut Animation, slot: &str, time: f32, color: Color) {
    set_key_value(anim.slots.entry(slot).entry("rgba"), time, color.to_f32().to_vec(), None);
}

/// 슬롯 밝은 색 + 어두운 색 키(rgba2). 밝은 색은 현재 값(rgba2 키 → rgba 키 → 셋업)을 유지한다.
pub(super) fn key_slot_dark(anim: &mut Animation, slot: &Slot, time: f32, dark: Color) {
    let light = match anim.slots.get(&slot.name).and_then(|t| t.get("rgba2")).filter(|k| !k.is_empty()) {
        Some(k) => (0..4).map(|c| sample(k, c, time, slot.color.to_f32()[c])).collect(),
        None => current_slot_color(anim, slot, time),
    };
    let d = dark.to_f32();
    let mut v = light;
    v.extend_from_slice(&d[..3]);
    set_key_value(anim.slots.entry(&slot.name).entry("rgba2"), time, v, None);
}

/// 슬롯의 현재 보이는 첨부물(attachment 키 → 셋업).
pub(super) fn current_slot_attachment(anim: &Animation, slot: &Slot, time: f32) -> Option<String> {
    match anim.slots.get(&slot.name).and_then(|t| t.get("attachment")).and_then(|k| sample_str(k, time)) {
        Some(s) => s,
        None => slot.attachment.clone(),
    }
}

/// 첨부물 전환 키.
pub(super) fn key_slot_attachment(anim: &mut Animation, slot: &str, time: f32, attachment: Option<String>) {
    set_key_value(anim.slots.entry(slot).entry("attachment"), time, vec![], attachment);
}

/// 제약 혼합류 속성 → (타임라인, 채널) 표(FORMAT.md 3.7).
pub(super) fn keyable_props(c: &Constraint) -> &'static [(&'static str, &'static str, usize)] {
    match c {
        Constraint::Ik(_) => &[("mix", "ik", 0), ("softness", "ik", 1), ("bendPositive", "ik", 2), ("compress", "ik", 3), ("stretch", "ik", 4)],
        Constraint::Transform(_) => &[
            ("mixRotate", "transform", 0),
            ("mixX", "transform", 1),
            ("mixY", "transform", 2),
            ("mixScaleX", "transform", 3),
            ("mixScaleY", "transform", 4),
            ("mixShearY", "transform", 5),
        ],
        Constraint::Path(_) => &[("position", "position", 0), ("spacing", "spacing", 0), ("mixRotate", "mix", 0), ("mixX", "mix", 1), ("mixY", "mix", 2)],
        Constraint::Physics(_) => &[
            ("inertia", "inertia", 0),
            ("strength", "strength", 0),
            ("damping", "damping", 0),
            ("mass", "mass", 0),
            ("wind", "wind", 0),
            ("gravity", "gravity", 0),
            ("mix", "mix", 0),
        ],
        Constraint::Slider(_) => &[("time", "time", 0), ("mix", "mix", 0)],
    }
}

/// 속성 값(JSON)을 키 값 숫자로. bendPositive는 1/−1, compress·stretch는 1/0.
pub(super) fn prop_number(prop: &str, v: &Value) -> Option<f32> {
    match (prop, v) {
        ("bendPositive", Value::Bool(b)) => Some(if *b { 1.0 } else { -1.0 }),
        ("bendPositive", _) => v.as_f64().map(|f| if f < 0.0 { -1.0 } else { 1.0 }),
        (_, Value::Bool(b)) => Some(if *b { 1.0 } else { 0.0 }),
        _ => v.as_f64().map(|f| f as f32),
    }
}

/// 제약 혼합류 값 `props`(속성 이름 → JSON 값)를 현재 시점 키로 넣는다. 같은 타임라인의 다른 채널은 현재 값을 유지한다.
pub(super) fn key_constraint_props(anim: &mut Animation, c: &Constraint, time: f32, props: &[(String, Value)]) {
    let table = keyable_props(c);
    let setup = c.to_json(true);
    let mut timelines: Vec<&str> = vec![];
    for (p, _) in props {
        if let Some((_, tl, _)) = table.iter().find(|(n, _, _)| n == p) {
            if !timelines.contains(tl) {
                timelines.push(tl);
            }
        }
    }
    for tl in timelines {
        let channels: Vec<&(&str, &str, usize)> = table.iter().filter(|(_, t, _)| *t == tl).collect();
        let keys = anim.constraints.entry(c.name()).entry(tl);
        let mut v: Vec<f32> = channels
            .iter()
            .map(|(p, _, ch)| {
                let d = setup.get(p).and_then(|x| prop_number(p, x)).unwrap_or(0.0);
                sample(keys, *ch, time, d)
            })
            .collect();
        for (p, val) in props {
            if let Some((_, _, ch)) = channels.iter().find(|(n, _, _)| n == p) {
                if let Some(x) = prop_number(p, val) {
                    v[*ch] = x;
                }
            }
        }
        set_key_value(keys, time, v, None);
    }
}

/// 애니메이션 안의 빈 타임라인·빈 묶음을 정리한다.
pub(super) fn prune(anim: &mut Animation) {
    for m in [&mut anim.bones, &mut anim.slots, &mut anim.constraints] {
        for t in m.values_mut() {
            t.retain(|_, k| !k.is_empty());
        }
        m.retain(|_, t| !t.is_empty());
    }
    for m in [&mut anim.deform, &mut anim.sequence] {
        for s in m.values_mut() {
            for a in s.values_mut() {
                a.retain(|_, k| !k.is_empty());
            }
            s.retain(|_, a| !a.is_empty());
        }
        m.retain(|_, s| !s.is_empty());
    }
}

/// 이름으로 애니메이션을 찾는다(명령 오류 메시지 포함).
pub(super) fn animation_mut<'a>(skel: &'a mut Skeleton, name: &str) -> crate::Result<&'a mut Animation> {
    skel.animation_mut(name).ok_or_else(|| crate::CoreError::NotFound(format!("애니메이션 '{name}'")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_keeps_time_order_and_replaces_same_time() {
        let mut keys = vec![];
        insert_key(&mut keys, Key::new(1.0, vec![1.0]));
        insert_key(&mut keys, Key::new(0.0, vec![0.0]));
        insert_key(&mut keys, Key::new(0.5, vec![5.0]));
        set_key_value(&mut keys, 0.50005, vec![7.0], None);
        let t: Vec<f32> = keys.iter().map(|k| k.time).collect();
        assert_eq!(t, vec![0.0, 0.5, 1.0]);
        assert_eq!(keys[1].v, vec![7.0]);
    }

    #[test]
    fn sample_outside_and_on_keys() {
        let keys = vec![Key::new(1.0, vec![2.0]), Key::new(2.0, vec![4.0])];
        assert_eq!(sample(&keys, 0, 0.0, 9.0), 2.0);
        assert_eq!(sample(&keys, 0, 3.0, 9.0), 4.0);
        assert_eq!(sample(&keys, 0, 2.0, 9.0), 4.0);
        assert_eq!(sample(&[], 0, 2.0, 9.0), 9.0);
    }
}

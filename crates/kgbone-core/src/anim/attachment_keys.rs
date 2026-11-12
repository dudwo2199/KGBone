//! 첨부물 타임라인: deform(정점 변형)과 sequence(프레임 넘김).
use super::apply::{ApplyScope, Mix};
use super::curve::{ratio_at, step_index};
use crate::model::{Animation, Attachment, Key, Skeleton};
use crate::pose::{deform_len, resolve_mesh, SkeletonPose};

pub(super) fn apply(skel: &Skeleton, anim: &Animation, t: f32, pose: &mut SkeletonPose, mix: Mix, scope: &ApplyScope) {
    for (skin_name, slots) in anim.deform.iter() {
        if !scope.uses_skin(skin_name) {
            continue;
        }
        let Some(skin) = skel.skin(skin_name) else { continue };
        for (slot, atts) in slots.iter() {
            for (att_name, keys) in atts.iter() {
                let Some(att) = skin.get(slot, att_name) else { continue };
                let Some(len) = vertex_data_len(skel, slot, att_name, att) else { continue };
                let Some(target) = deform_at(keys, t, len) else { continue };
                let entry = pose.deform.entry((slot.clone(), att_name.clone())).or_insert_with(|| vec![0.0; len]);
                entry.resize(len, 0.0);
                for (cur, tv) in entry.iter_mut().zip(target) {
                    // 셋업 변형은 0이다.
                    *cur = mix.blend(*cur, tv, 0.0);
                }
            }
        }
    }
    if !mix.discrete() {
        return;
    }
    for (skin_name, slots) in anim.sequence.iter() {
        if !scope.uses_skin(skin_name) {
            continue;
        }
        let Some(skin) = skel.skin(skin_name) else { continue };
        for (slot, atts) in slots.iter() {
            for (att_name, keys) in atts.iter() {
                let count = skin.get(slot, att_name).and_then(sequence_count).unwrap_or(1);
                if let Some(idx) = sequence_frame(keys, t, count) {
                    pose.sequence_index.insert((slot.clone(), att_name.clone()), idx);
                }
            }
        }
    }
}

/// 첨부물의 변형 값 길이(정점 데이터가 없으면 None).
fn vertex_data_len(skel: &Skeleton, slot: &str, name: &str, att: &Attachment) -> Option<usize> {
    if let Some((weighted, v)) = att.vertices() {
        return Some(deform_len(weighted, v));
    }
    resolve_mesh(skel, slot, name, att).map(|(m, _)| deform_len(m.weighted, &m.vertices))
}

/// 키 하나를 길이 `len`의 전체 변형 배열로 편다(offset 위치부터 v).
fn expand(k: &Key, len: usize) -> Vec<f32> {
    let mut out = vec![0.0; len];
    for (j, v) in k.v.iter().enumerate() {
        if let Some(slot) = out.get_mut(k.offset as usize + j) {
            *slot = *v;
        }
    }
    out
}

/// 시점의 변형 배열: 앞 키와 뒤 키를 곡선 비율로 섞는다.
pub(crate) fn deform_at(keys: &[Key], t: f32, len: usize) -> Option<Vec<f32>> {
    let (i, r) = ratio_at(keys, t)?;
    let a = expand(&keys[i], len);
    if r <= 0.0 || i + 1 >= keys.len() {
        return Some(a);
    }
    let b = expand(&keys[i + 1], len);
    Some(a.iter().zip(&b).map(|(x, y)| x + (y - x) * r).collect())
}

fn sequence_count(att: &Attachment) -> Option<u32> {
    match att {
        Attachment::Region(r) => r.sequence.as_ref().map(|s| s.count),
        Attachment::Mesh(m) => m.sequence.as_ref().map(|s| s.count),
        Attachment::LinkedMesh(l) => l.sequence.as_ref().map(|s| s.count),
        _ => None,
    }
}

/// 시퀀스 키(s = 모드, v = [index, delay])로 시점의 프레임 번호를 구한다.
/// 지난 프레임 수 f = ⌊(t − 키 시간) / delay⌋. 역방향 모드는 정방향 결과를 (count − 1 − 번호)로 뒤집는다.
pub(crate) fn sequence_frame(keys: &[Key], t: f32, count: u32) -> Option<i32> {
    let k = &keys[step_index(keys, t)?];
    let n = count.max(1) as i32;
    let start = k.v.first().copied().unwrap_or(0.0) as i32;
    let delay = k.v.get(1).copied().unwrap_or(0.0);
    let f = if delay > 0.0 { (((t - k.time).max(0.0) / delay) + 1e-4).floor() as i32 } else { 0 };
    let p = start + f;
    let pingpong = |p: i32| {
        let n2 = (n - 1) * 2;
        if n2 <= 0 {
            return 0;
        }
        let q = p.rem_euclid(n2);
        if q >= n {
            n2 - q
        } else {
            q
        }
    };
    let mode = k.s.as_deref().unwrap_or("hold");
    let idx = match mode {
        "once" => p.min(n - 1),
        "loop" => p.rem_euclid(n),
        "pingpong" => pingpong(p),
        "onceReverse" => n - 1 - p.min(n - 1),
        "loopReverse" => n - 1 - p.rem_euclid(n),
        "pingpongReverse" => n - 1 - pingpong(p),
        _ => start,
    };
    Some(idx.clamp(0, n - 1))
}

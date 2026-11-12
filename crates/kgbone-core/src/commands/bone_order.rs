//! 본 배열 순서 지키기와 가중치 정점의 본 번호 다시 매기기.
//!
//! 규칙: `skeleton.bones`는 항상 부모가 자식보다 앞이다. 가중치 정점(`weighted: true`)의 본 번호는
//! `skeleton.bones` 배열 위치이므로, 배열 순서가 바뀌거나 본이 빠지면 번호를 새 위치로 고쳐야 한다.
use crate::model::{Attachment, Skeleton};

/// 가중치 정점 목록(가중치가 있는 첨부물만).
fn weighted_vertices_mut(a: &mut Attachment) -> Option<&mut Vec<f32>> {
    match a {
        Attachment::Mesh(m) if m.weighted => Some(&mut m.vertices),
        Attachment::BoundingBox(m) if m.weighted => Some(&mut m.vertices),
        Attachment::Clipping(m) if m.weighted => Some(&mut m.vertices),
        Attachment::Path(m) if m.weighted => Some(&mut m.vertices),
        _ => None,
    }
}

/// 가중치 정점 배열에서 본 번호가 들어 있는 자리들.
/// 모양: 정점마다 `[본 수 n, (본 번호, x, y, 가중치) × n]`.
fn bone_index_slots(vertices: &[f32]) -> Vec<usize> {
    let mut out = vec![];
    let mut i = 0;
    while i < vertices.len() {
        let n = vertices[i].max(0.0) as usize;
        for j in 0..n {
            let at = i + 1 + 4 * j;
            if at < vertices.len() {
                out.push(at);
            }
        }
        i += 1 + 4 * n;
    }
    out
}

/// 가중치 정점이 쓰는 본 번호 가운데 `used(번호)`가 참인 것이 하나라도 있는가.
pub(super) fn weights_use(skel: &Skeleton, used: impl Fn(usize) -> bool) -> bool {
    for skin in &skel.skins {
        for atts in skin.attachments.values() {
            for a in atts.values() {
                if let Some((true, v)) = a.vertices() {
                    if bone_index_slots(v).into_iter().any(|at| used(v[at] as usize)) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 모든 가중치 정점의 본 번호를 `map(옛 번호) → 새 번호`로 바꾼다.
pub(super) fn remap_weights(skel: &mut Skeleton, map: impl Fn(usize) -> usize) {
    for skin in &mut skel.skins {
        for atts in skin.attachments.values_mut() {
            for a in atts.values_mut() {
                if let Some(v) = weighted_vertices_mut(a) {
                    for at in bone_index_slots(v) {
                        v[at] = map(v[at] as usize) as f32;
                    }
                }
            }
        }
    }
}

/// 부모가 자식보다 앞에 오도록 본 배열을 다시 늘어놓는다(그 밖의 순서는 최대한 유지).
/// 부모보다 앞에 있던 본(과 그 부분 트리)은 부모 바로 뒤로 옮긴다. 가중치 번호도 함께 고친다.
pub(super) fn sort_parents_first(skel: &mut Skeleton) {
    let n = skel.bones.len();
    let mut placed = vec![false; n];
    let mut order: Vec<usize> = Vec::with_capacity(n);
    let parent_of: Vec<Option<usize>> =
        skel.bones.iter().map(|b| b.parent.as_deref().and_then(|p| skel.bone_index(p))).collect();

    // 본 i를 놓고, 그 본을 기다리던 자식들도 이어서 놓는다(원래 순서대로).
    fn place(i: usize, parent_of: &[Option<usize>], placed: &mut [bool], order: &mut Vec<usize>, waiting: &mut Vec<usize>) {
        placed[i] = true;
        order.push(i);
        let ready: Vec<usize> = waiting.iter().copied().filter(|&w| parent_of[w] == Some(i)).collect();
        waiting.retain(|w| !ready.contains(w));
        for r in ready {
            place(r, parent_of, placed, order, waiting);
        }
    }

    let mut waiting: Vec<usize> = vec![];
    for i in 0..n {
        match parent_of[i] {
            Some(p) if !placed[p] => waiting.push(i),
            _ => place(i, &parent_of, &mut placed, &mut order, &mut waiting),
        }
    }
    // 부모를 끝내 못 찾은 본(고리 등 잘못된 데이터)은 원래 순서대로 뒤에 붙인다.
    order.extend(waiting);
    if order.iter().enumerate().all(|(new, &old)| new == old) {
        return;
    }
    let mut new_of = vec![0usize; n];
    for (new, &old) in order.iter().enumerate() {
        new_of[old] = new;
    }
    let old_bones = std::mem::take(&mut skel.bones);
    let mut slots: Vec<Option<_>> = old_bones.into_iter().map(Some).collect();
    skel.bones = order.iter().map(|&old| slots[old].take().expect("본")).collect();
    remap_weights(skel, |old| new_of.get(old).copied().unwrap_or(old));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Bone, MeshAttachment, Skin};

    #[test]
    fn bone_index_slots_reads_weighted_layout() {
        // 정점 2개: [1, b0, x, y, w] , [2, b1, x, y, w, b2, x, y, w]
        let v = vec![1.0, 0.0, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 0.0, 0.5, 2.0, 0.0, 0.0, 0.5];
        assert_eq!(bone_index_slots(&v), vec![1, 6, 10]);
    }

    #[test]
    fn sort_moves_subtree_after_parent_and_remaps_weights() {
        let mut skel = Skeleton::new_with_root();
        // root, a(부모 c), a2(부모 a), c(부모 root) → root, c, a, a2
        skel.bones.push(Bone::new("a", Some("c".into())));
        skel.bones.push(Bone::new("a2", Some("a".into())));
        skel.bones.push(Bone::new("c", Some("root".into())));
        let mut skin = Skin::new("default");
        // 본 번호 1(a)과 3(c)을 쓰는 가중치 정점 하나
        let mesh = MeshAttachment { weighted: true, vertices: vec![2.0, 1.0, 0.0, 0.0, 0.5, 3.0, 0.0, 0.0, 0.5], ..Default::default() };
        skin.attachments.entry("s").insert("m", Attachment::Mesh(mesh));
        skel.skins = vec![skin];
        sort_parents_first(&mut skel);
        let names: Vec<_> = skel.bones.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["root", "c", "a", "a2"]);
        let Some(Attachment::Mesh(m)) = skel.skins[0].get("s", "m") else { panic!() };
        // a: 1 → 2, c: 3 → 1
        assert_eq!(m.vertices, vec![2.0, 2.0, 0.0, 0.0, 0.5, 1.0, 0.0, 0.0, 0.5]);
    }
}

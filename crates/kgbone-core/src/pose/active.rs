//! 활성 판정: 스킨 전용(skinRequired) 본·제약은 켠 스킨에 들어 있을 때만 계산한다.
use crate::model::Skeleton;

/// 본마다 활성 여부. skinRequired 본은 켠 스킨(`skin`)의 `bones`에 있어야 하고, 부모가 비활성이면 자식도 비활성.
pub(crate) fn active_bones(skel: &Skeleton, parents: &[Option<usize>], skin: Option<&str>) -> Vec<bool> {
    let skin_bones: &[String] = skin.and_then(|s| skel.skin(s)).map(|s| s.bones.as_slice()).unwrap_or(&[]);
    let mut out = Vec::with_capacity(skel.bones.len());
    for (i, b) in skel.bones.iter().enumerate() {
        let own = !b.skin_required || skin_bones.iter().any(|n| *n == b.name);
        let parent_ok = parents.get(i).copied().flatten().map(|p| out[p]).unwrap_or(true);
        out.push(own && parent_ok);
    }
    out
}

/// 제약마다 활성 여부. skinRequired 제약은 켠 스킨의 `constraints`에 있어야 하고,
/// 참조하는 본(받는 본·목표·원본) 중 하나라도 비활성이면 계산하지 않는다.
pub(crate) fn active_constraints(skel: &Skeleton, bone_active: &[bool], skin: Option<&str>) -> Vec<bool> {
    let skin_cons: &[String] = skin.and_then(|s| skel.skin(s)).map(|s| s.constraints.as_slice()).unwrap_or(&[]);
    skel.constraints
        .iter()
        .map(|c| {
            let own = !c.skin_required() || skin_cons.iter().any(|n| n == c.name());
            let bones_ok = c.referenced_bones().iter().all(|n| skel.bone_index(n).map(|i| bone_active[i]).unwrap_or(false));
            own && bones_ok
        })
        .collect()
}

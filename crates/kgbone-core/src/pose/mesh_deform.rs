//! 메시 정점의 월드 좌표(가중치·변형 키 반영).
use super::{SkeletonPose, WorldBone};
use crate::model::{Attachment, MeshAttachment, Skeleton, Skin};

/// 슬롯의 첨부물을 찾는다: 켠 스킨에 있으면 그것, 없으면 기본 스킨. (스킨 이름, 첨부물)을 돌려준다.
pub(crate) fn find_attachment<'a>(skel: &'a Skeleton, skin: Option<&str>, slot: &str, name: &str) -> Option<(&'a str, &'a Attachment)> {
    if let Some(s) = skin.and_then(|s| skel.skin(s)) {
        if let Some(a) = s.get(slot, name) {
            return Some((s.name.as_str(), a));
        }
    }
    let d = skel.skin(Skin::DEFAULT)?;
    d.get(slot, name).map(|a| (d.name.as_str(), a))
}

/// 정점 배열 → 월드 좌표 [x0, y0, …].
/// - 가중치 없음: [x, y, …]가 슬롯 본 기준. 변형 값 `deform[2i], deform[2i+1]`을 더한 뒤 슬롯 본으로 옮긴다.
/// - 가중치 있음: 정점마다 [n, (본 번호, x, y, 가중치)×n]. 본별 항목마다 변형 값 2개(항목 순서)를 더하고
///   월드 = Σ 가중치 × 본월드(x + dx, y + dy).
pub(crate) fn vertex_world_positions(weighted: bool, vertices: &[f32], slot_bone: &WorldBone, world: &[WorldBone], deform: Option<&[f32]>) -> Vec<f32> {
    let d = |k: usize| deform.and_then(|v| v.get(k)).copied().unwrap_or(0.0);
    let mut out = Vec::new();
    if !weighted {
        for (i, p) in vertices.chunks_exact(2).enumerate() {
            let (x, y) = slot_bone.local_to_world(p[0] + d(2 * i), p[1] + d(2 * i + 1));
            out.push(x);
            out.push(y);
        }
        return out;
    }
    let mut i = 0usize;
    let mut entry = 0usize;
    while i < vertices.len() {
        let n = vertices[i].max(0.0) as usize;
        i += 1;
        let (mut wx, mut wy) = (0.0f32, 0.0f32);
        for _ in 0..n {
            if i + 3 >= vertices.len() {
                break;
            }
            let bone = vertices[i] as usize;
            let (lx, ly, w) = (vertices[i + 1] + d(2 * entry), vertices[i + 2] + d(2 * entry + 1), vertices[i + 3]);
            if let Some(b) = world.get(bone) {
                let (x, y) = b.local_to_world(lx, ly);
                wx += x * w;
                wy += y * w;
            }
            i += 4;
            entry += 1;
        }
        out.push(wx);
        out.push(wy);
    }
    out
}

/// 가중치 정점 배열의 본별 항목 수(= 변형 값 길이 / 2).
pub(crate) fn weighted_entry_count(vertices: &[f32]) -> usize {
    let (mut i, mut count) = (0usize, 0usize);
    while i < vertices.len() {
        let n = vertices[i].max(0.0) as usize;
        count += n;
        i += 1 + 4 * n;
    }
    count
}

/// 첨부물 정점 데이터의 변형 값 길이.
pub(crate) fn deform_len(weighted: bool, vertices: &[f32]) -> usize {
    if weighted {
        weighted_entry_count(vertices) * 2
    } else {
        vertices.len()
    }
}

/// 메시(또는 연결 메시)의 실제 정점 데이터: (메시 본체, 변형 키를 찾을 첨부물 이름).
/// 연결 메시는 부모 메시의 uv·삼각형·정점을 쓰고, `timelines`가 참이면 부모 이름의 변형을 쓴다.
pub(crate) fn resolve_mesh<'a>(skel: &'a Skeleton, slot: &str, name: &'a str, att: &'a Attachment) -> Option<(&'a MeshAttachment, &'a str)> {
    match att {
        Attachment::Mesh(m) => Some((m, name)),
        Attachment::LinkedMesh(l) => {
            let skin = skel.skin(l.skin.as_deref().unwrap_or(Skin::DEFAULT))?;
            match skin.get(slot, &l.parent)? {
                Attachment::Mesh(m) => Some((m, if l.timelines { l.parent.as_str() } else { name })),
                _ => None,
            }
        }
        _ => None,
    }
}

/// 슬롯 `slot`의 메시 첨부물 `name`의 월드 정점(현재 포즈의 변형 반영).
pub(crate) fn mesh_world_vertices(skel: &Skeleton, pose: &SkeletonPose, slot: usize, mesh: &MeshAttachment, deform_name: &str) -> Vec<f32> {
    let s = &skel.slots[slot];
    let bone = skel.bone_index(&s.bone).map(|i| pose.world[i]).unwrap_or_default();
    let deform = pose.deform.get(&(s.name.clone(), deform_name.to_string())).map(Vec::as_slice);
    vertex_world_positions(mesh.weighted, &mesh.vertices, &bone, &pose.world, deform)
}

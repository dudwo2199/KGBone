//! 가중치: 바인딩, 자동 가중치, 페인팅.
use crate::pose::WorldBone;

/// 가중치 계산에 쓰는 본 정보(월드).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneInfo {
    /// `skeleton.bones` 안의 번호.
    pub index: usize,
    pub world: WorldBone,
    pub length: f32,
}

/// 정점 하나의 (본 번호, 가중치) 목록. 합은 1.
pub type VertexWeights = Vec<(usize, f32)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintMode {
    Add,
    Remove,
    Replace,
}

/// 정점 월드 좌표마다 가까운 본일수록 큰 가중치(거리 반비례, 합 = 1).
pub fn auto_weights(world_vertices: &[f32], bones: &[BoneInfo]) -> Vec<VertexWeights> {
    let _ = (world_vertices, bones);
    vec![]
}

/// 가중치 + 월드 좌표 → Spine식 가중치 정점 배열([본 수, (본 번호, 본 기준 x, y, 가중치)×n]…).
/// 셋업 포즈에서 바인딩 전후 정점 월드 위치가 같다.
pub fn bind_vertices(world_vertices: &[f32], weights: &[VertexWeights], world: &[WorldBone]) -> Vec<f32> {
    let _ = (world_vertices, weights, world);
    vec![]
}

/// 가중치 정점 배열 → 슬롯 본 기준 가중치 없는 정점 [x, y, …] (셋업 포즈 위치 유지).
pub fn unbind_vertices(weighted: &[f32], world: &[WorldBone], slot_bone: &WorldBone) -> Vec<f32> {
    let _ = (weighted, world, slot_bone);
    vec![]
}

/// 붓질 한 번: 중심 (x, y)에서 `radius` 안 정점의 `bone` 가중치를 `strength`만큼 바꾸고 다시 합 = 1로 맞춘다.
pub fn paint_weights(
    weights: &mut [VertexWeights],
    world_vertices: &[f32],
    bone: usize,
    x: f32,
    y: f32,
    radius: f32,
    strength: f32,
    mode: PaintMode,
) {
    let _ = (weights, world_vertices, bone, x, y, radius, strength, mode);
}

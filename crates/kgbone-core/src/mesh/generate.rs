//! 이미지 알파로 메시 자동 생성.

/// 이미지의 알파 채널(행 우선, 위쪽 행부터).
#[derive(Clone, Debug, PartialEq)]
pub struct AlphaMask {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// 생성된 메시. 정점은 이미지 중심이 원점인 로컬 좌표(y 위로), uv는 0~1(v 아래로).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct GeneratedMesh {
    pub vertices: Vec<f32>,
    pub uvs: Vec<f32>,
    pub triangles: Vec<u32>,
    /// 외곽선 정점 수(앞에서부터).
    pub hull: u32,
}

/// `detail`(0~1): 클수록 외곽선 정점이 많고 내부 정점을 더 넣는다.
pub fn generate_from_alpha(mask: &AlphaMask, detail: f32) -> GeneratedMesh {
    let _ = (mask, detail);
    GeneratedMesh::default()
}

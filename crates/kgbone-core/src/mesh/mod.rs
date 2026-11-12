//! 메시 도구: 삼각분할, 이미지 알파로 자동 메시 생성, 가중치(바인딩·자동·페인팅).
mod generate;
mod triangulate;
mod weights;

pub use generate::{generate_from_alpha, AlphaMask, GeneratedMesh};
pub use triangulate::triangulate;
pub(crate) use weights::read_weights;
pub use weights::{auto_weights, bind_vertices, paint_weights, unbind_vertices, BoneInfo, PaintMode, VertexWeights};

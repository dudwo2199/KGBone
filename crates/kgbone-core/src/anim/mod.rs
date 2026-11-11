//! 애니메이션: 곡선 보간과 포즈 적용.
mod apply;
pub mod curve;

pub use apply::{apply_animation, ApplyOptions};

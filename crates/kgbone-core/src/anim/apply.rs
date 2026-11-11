//! 애니메이션 한 개를 포즈에 적용.
use crate::model::{Animation, Skeleton};
use crate::pose::SkeletonPose;

#[derive(Clone, Copy, Debug)]
pub struct ApplyOptions {
    /// 0~1. 1이면 애니메이션 값으로 바꾸고, 그보다 작으면 섞는다.
    pub alpha: f32,
    /// 참이면 셋업 값에서의 차이를 현재 값에 더한다(슬라이더 additive).
    pub additive: bool,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        ApplyOptions { alpha: 1.0, additive: false }
    }
}

/// `time` 시점의 값을 `pose`에 적용한다.
pub fn apply_animation(skel: &Skeleton, anim: &Animation, time: f32, pose: &mut SkeletonPose, opts: ApplyOptions) {
    let _ = (skel, anim, time, pose, opts);
}

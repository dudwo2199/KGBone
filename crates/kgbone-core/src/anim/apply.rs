//! 애니메이션 한 개를 포즈에 적용.
//!
//! 값의 기준(FORMAT.md 3.7): 본 값은 셋업 기준 상대값(rotate·translate·shear는 더하고 scale은 곱한다),
//! 슬롯 색·제약 값은 절대값, deform은 셋업 정점에 더할 차이값. 이산 값(첨부물·상속·그리기 순서·시퀀스·
//! 참거짓 제약 값)은 계단으로 바뀐다.
//!
//! 섞기(`ApplyOptions`):
//! - 비가산: `현재 = 현재 + (목표 − 현재) × alpha`. alpha < 1에서 회전 차이는 (−180, 180]으로 감아 가까운 쪽으로 섞는다.
//! - 가산: `현재 = 현재 + (목표 − 셋업) × alpha` (셋업 대비 차이를 더한다).
//! - 이산 값은 alpha ≥ 0.5일 때만 바꾼다.
use super::{attachment_keys, bone_keys, constraint_keys, slot_keys};
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

/// 적용 범위(엔진 내부용).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ApplyScope<'a> {
    /// 켠 스킨. deform·sequence 키는 기본 스킨과 이 스킨 것만 쓴다(`all_skins`가 참이면 모든 스킨).
    pub skin: Option<&'a str>,
    pub all_skins: bool,
    /// 이 번호의 제약 값 키는 건너뛴다(슬라이더가 자기 자신을 다시 바꾸는 순환 방지).
    pub skip_constraint: Option<usize>,
}

impl ApplyScope<'_> {
    pub(crate) fn uses_skin(&self, name: &str) -> bool {
        self.all_skins || name == crate::model::Skin::DEFAULT || self.skin == Some(name)
    }
}

/// 섞는 방법 하나(내부 공용).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Mix {
    pub alpha: f32,
    pub additive: bool,
}

impl Mix {
    /// 연속 값 섞기. `setup`은 가산일 때의 기준.
    pub(crate) fn blend(self, current: f32, target: f32, setup: f32) -> f32 {
        if self.additive {
            current + (target - setup) * self.alpha
        } else if self.alpha >= 1.0 {
            target
        } else {
            current + (target - current) * self.alpha
        }
    }
    /// 각도 섞기(비가산이고 alpha < 1이면 가까운 쪽으로 돈다).
    pub(crate) fn blend_angle(self, current: f32, target: f32, setup: f32) -> f32 {
        if self.additive || self.alpha >= 1.0 {
            self.blend(current, target, setup)
        } else {
            current + crate::pose::wrap_deg(target - current) * self.alpha
        }
    }
    /// 이산 값을 바꿀지.
    pub(crate) fn discrete(self) -> bool {
        self.alpha >= 0.5
    }
}

/// `time` 시점의 값을 `pose`에 적용한다(모든 스킨의 deform·sequence 키 포함).
pub fn apply_animation(skel: &Skeleton, anim: &Animation, time: f32, pose: &mut SkeletonPose, opts: ApplyOptions) {
    apply_scoped(skel, anim, time, pose, opts, ApplyScope { all_skins: true, ..Default::default() });
}

/// 범위를 정해 적용한다. 포즈 배열 길이가 뼈대와 다르면(빈 포즈 등) 셋업으로 먼저 채운다.
pub(crate) fn apply_scoped(skel: &Skeleton, anim: &Animation, time: f32, pose: &mut SkeletonPose, opts: ApplyOptions, scope: ApplyScope) {
    if pose.locals.len() != skel.bones.len() || pose.slot_colors.len() != skel.slots.len() || pose.constraint_values.len() != skel.constraints.len() {
        *pose = SkeletonPose::setup(skel);
    }
    let mix = Mix { alpha: opts.alpha.clamp(0.0, 1.0), additive: opts.additive };
    if mix.alpha <= 0.0 {
        return;
    }
    bone_keys::apply(skel, anim, time, pose, mix);
    slot_keys::apply(skel, anim, time, pose, mix);
    constraint_keys::apply(skel, anim, time, pose, mix, scope.skip_constraint);
    attachment_keys::apply(skel, anim, time, pose, mix, &scope);
}

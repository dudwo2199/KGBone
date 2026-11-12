//! Spine JSON 구조 정보(내보낼 때 원본과 합치기용): 이름 → 값 사전인 곳과, 모델이 아는 키.
use crate::format::preserve::{path_is, Schema};
use crate::model::timeline;

pub(super) struct SpineSchema;

/// 애니메이션 안의 섹션(모두 이름 → 값 사전).
const ANIM_SECTIONS: [&str; 8] = ["bones", "slots", "ik", "transform", "path", "physics", "attachments", "deform"];

impl Schema for SpineSchema {
    fn is_dict(&self, p: &[String]) -> bool {
        path_is(p, &["skins", "#", "attachments"])
            || path_is(p, &["skins", "#", "attachments", "*"])
            || path_is(p, &["events"])
            || path_is(p, &["animations"])
            || ANIM_SECTIONS.iter().any(|s| path_is(p, &["animations", "*", s]))
            || path_is(p, &["animations", "*", "attachments", "*"])
            || path_is(p, &["animations", "*", "attachments", "*", "*"])
    }
    fn owns(&self, p: &[String], key: &str) -> bool {
        let anim_tl = |section: &str| path_is(p, &["animations", "*", section, "*"]);
        if p.is_empty() {
            super::read::TOP_KEYS.contains(&key)
        } else if path_is(p, &["animations", "*"]) {
            ANIM_SECTIONS.contains(&key) || matches!(key, "drawOrder" | "draworder" | "events")
        } else if anim_tl("bones") {
            timeline::BONE.contains(&key)
        } else if anim_tl("slots") {
            timeline::SLOT.contains(&key)
        } else if anim_tl("path") {
            timeline::PATH.contains(&key)
        } else if anim_tl("physics") {
            timeline::PHYSICS.contains(&key)
        } else if path_is(p, &["animations", "*", "attachments", "*", "*", "*"]) {
            matches!(key, "deform" | "sequence")
        } else if path_is(p, &["bones", "#"]) {
            // 구버전 상속 키(지금은 `inherit`로 쓴다).
            key == "transform"
        } else if path_is(p, &["skins", "#", "attachments", "*", "*"]) {
            // 이미지 경로는 `name`/`path` 중 하나로, 링크드 메시 구버전 `deform`은 `timelines`로 쓴다.
            matches!(key, "name" | "path" | "deform")
        } else {
            false
        }
    }
}

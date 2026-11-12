//! 이름 바꾸기·삭제 때 참조를 따라 고치는 도우미.
//!
//! 이름으로 서로를 가리키는 곳(FORMAT.md): 본 부모, 슬롯 본, 제약 참조, 스킨 bones/constraints/첨부물 맵,
//! 링크드 메시 parent/skin, 클리핑 end, 애니메이션의 타임라인 맵 키·첨부물 키·그리기 순서·이벤트 키, 슬라이더 animation.
use super::draw_order;
use crate::model::{Attachment, Constraint, Skeleton, Skin};
use crate::{CoreError, Result};

/// 새 항목 이름: 주어진 이름(없으면 `base`)에서 겹치지 않는 이름(`base`, `base2`…). 빈 이름은 거부.
pub(super) fn pick_name(skel: &Skeleton, given: Option<&str>, base: &str, taken: impl Fn(&str) -> bool) -> Result<String> {
    let base = match given {
        Some(g) => {
            check_name(g)?;
            g
        }
        None => base,
    };
    Ok(skel.unique_name(base, taken))
}

/// 빈 이름(공백뿐 포함) 거부.
pub(super) fn check_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(CoreError::InvalidArgs("이름이 비어 있습니다".into()));
    }
    Ok(())
}

pub(super) fn bone_exists(skel: &Skeleton, name: &str) -> Result<()> {
    skel.bone(name).map(|_| ()).ok_or_else(|| CoreError::NotFound(format!("본 '{name}'")))
}

pub(super) fn slot_exists(skel: &Skeleton, name: &str) -> Result<()> {
    skel.slot(name).map(|_| ()).ok_or_else(|| CoreError::NotFound(format!("슬롯 '{name}'")))
}

fn rename_in(list: &mut [String], old: &str, new: &str) {
    for n in list.iter_mut().filter(|n| *n == old) {
        *n = new.to_string();
    }
}

fn swap(s: &mut String, old: &str, new: &str) {
    if s == old {
        *s = new.to_string();
    }
}

/// 본 이름 바꾸기.
pub(super) fn rename_bone(skel: &mut Skeleton, old: &str, new: &str) {
    for b in &mut skel.bones {
        swap(&mut b.name, old, new);
        if let Some(p) = &mut b.parent {
            swap(p, old, new);
        }
    }
    for s in &mut skel.slots {
        swap(&mut s.bone, old, new);
    }
    for c in &mut skel.constraints {
        match c {
            Constraint::Ik(c) => {
                rename_in(&mut c.bones, old, new);
                swap(&mut c.target, old, new);
            }
            Constraint::Transform(c) => {
                rename_in(&mut c.bones, old, new);
                swap(&mut c.source, old, new);
            }
            Constraint::Path(c) => rename_in(&mut c.bones, old, new),
            Constraint::Physics(c) => swap(&mut c.bone, old, new),
            Constraint::Slider(c) => {
                if let Some(b) = &mut c.bone {
                    swap(b, old, new);
                }
            }
        }
    }
    for s in &mut skel.skins {
        rename_in(&mut s.bones, old, new);
    }
    for a in &mut skel.animations {
        a.bones.rename(old, new);
    }
}

/// 슬롯 이름 바꾸기.
pub(super) fn rename_slot(skel: &mut Skeleton, old: &str, new: &str) {
    for s in &mut skel.slots {
        swap(&mut s.name, old, new);
    }
    for skin in &mut skel.skins {
        skin.attachments.rename(old, new);
        for atts in skin.attachments.values_mut() {
            for a in atts.values_mut() {
                if let Attachment::Clipping(c) = a {
                    if let Some(e) = &mut c.end {
                        swap(e, old, new);
                    }
                }
            }
        }
    }
    for c in &mut skel.constraints {
        if let Constraint::Path(p) = c {
            swap(&mut p.target, old, new);
        }
    }
    for a in &mut skel.animations {
        a.slots.rename(old, new);
        for m in [&mut a.deform, &mut a.sequence] {
            for s in m.values_mut() {
                s.rename(old, new);
            }
        }
        for k in &mut a.draw_order {
            for o in &mut k.offsets {
                swap(&mut o.slot, old, new);
            }
        }
    }
}

/// 제약 이름 바꾸기.
pub(super) fn rename_constraint(skel: &mut Skeleton, old: &str, new: &str) {
    for c in &mut skel.constraints {
        swap(c.name_mut(), old, new);
    }
    for s in &mut skel.skins {
        rename_in(&mut s.constraints, old, new);
    }
    for a in &mut skel.animations {
        a.constraints.rename(old, new);
    }
}

/// 스킨 이름 바꾸기(작업 상태의 activeSkin은 부르는 쪽이 고친다).
pub(super) fn rename_skin(skel: &mut Skeleton, old: &str, new: &str) {
    for s in &mut skel.skins {
        swap(&mut s.name, old, new);
        for atts in s.attachments.values_mut() {
            for a in atts.values_mut() {
                if let Attachment::LinkedMesh(l) = a {
                    if let Some(k) = &mut l.skin {
                        swap(k, old, new);
                    }
                }
            }
        }
    }
    for a in &mut skel.animations {
        a.deform.rename(old, new);
        a.sequence.rename(old, new);
    }
}

/// 애니메이션 이름 바꾸기(작업 상태의 activeAnimation은 부르는 쪽이 고친다).
pub(super) fn rename_animation(skel: &mut Skeleton, old: &str, new: &str) {
    for a in &mut skel.animations {
        swap(&mut a.name, old, new);
    }
    for c in &mut skel.constraints {
        if let Constraint::Slider(s) = c {
            swap(&mut s.animation, old, new);
        }
    }
}

/// 이벤트 이름 바꾸기.
pub(super) fn rename_event(skel: &mut Skeleton, old: &str, new: &str) {
    for e in &mut skel.events {
        swap(&mut e.name, old, new);
    }
    for a in &mut skel.animations {
        for k in &mut a.events {
            swap(&mut k.name, old, new);
        }
    }
}

/// 링크드 메시가 가리키는 부모 스킨 이름(없으면 기본 스킨).
fn linked_skin(l: &crate::model::LinkedMeshAttachment) -> &str {
    l.skin.as_deref().unwrap_or(Skin::DEFAULT)
}

/// 다른 스킨에도 같은 슬롯·같은 이름의 첨부물이 있는가.
pub(super) fn attachment_shared(skel: &Skeleton, skin: &str, slot: &str, name: &str) -> bool {
    skel.skins.iter().any(|s| s.name != skin && s.get(slot, name).is_some())
}

/// 첨부물 이름 바꾸기(`skin` 스킨의 `slot` 슬롯). 이미지 경로가 키 이름이었으면 `path`에 옛 이름을 적어 그림을 유지한다.
/// 슬롯 셋업 첨부물·첨부물 키는 다른 스킨이 같은 이름을 쓰지 않을 때만 고친다.
pub(super) fn rename_attachment(skel: &mut Skeleton, skin: &str, slot: &str, old: &str, new: &str) {
    let shared = attachment_shared(skel, skin, slot, old);
    for s in &mut skel.skins {
        let is_this = s.name == skin;
        let Some(atts) = s.attachments.get_mut(slot) else { continue };
        if is_this {
            if let Some(a) = atts.get_mut(old) {
                let path = match a {
                    Attachment::Region(r) => Some(&mut r.path),
                    Attachment::Mesh(m) => Some(&mut m.path),
                    Attachment::LinkedMesh(l) => Some(&mut l.path),
                    _ => None,
                };
                if let Some(p) = path {
                    if p.is_none() {
                        *p = Some(old.to_string());
                    }
                }
            }
            atts.rename(old, new);
        }
        for a in atts.values_mut() {
            if let Attachment::LinkedMesh(l) = a {
                if l.parent == old && linked_skin(l) == skin {
                    l.parent = new.to_string();
                }
            }
        }
    }
    for a in &mut skel.animations {
        for m in [&mut a.deform, &mut a.sequence] {
            if let Some(s) = m.get_mut(skin).and_then(|s| s.get_mut(slot)) {
                s.rename(old, new);
            }
        }
        if !shared {
            if let Some(keys) = a.slots.get_mut(slot).and_then(|t| t.get_mut("attachment")) {
                for k in keys {
                    if k.s.as_deref() == Some(old) {
                        k.s = Some(new.to_string());
                    }
                }
            }
        }
    }
    if !shared {
        if let Some(s) = skel.slot_mut(slot) {
            if s.attachment.as_deref() == Some(old) {
                s.attachment = Some(new.to_string());
            }
        }
    }
}

/// 제약 하나와 그 참조(스킨 constraints 목록, 애니메이션 제약 타임라인)를 지운다.
pub(super) fn remove_constraint(skel: &mut Skeleton, name: &str) {
    skel.constraints.retain(|c| c.name() != name);
    for s in &mut skel.skins {
        s.constraints.retain(|c| c != name);
    }
    for a in &mut skel.animations {
        a.constraints.remove(name);
    }
}

/// 슬롯 하나와 그 참조를 지운다: 스킨 첨부물, 슬롯 타임라인, deform·sequence, 그리기 순서 키,
/// 이 슬롯을 목표로 하는 패스 제약, 클리핑 end(비움).
pub(super) fn remove_slot(skel: &mut Skeleton, name: &str) -> Result<()> {
    draw_order::preserving_keys(skel, |skel| {
        skel.slots.retain(|s| s.name != name);
        Ok(())
    })?;
    for skin in &mut skel.skins {
        skin.attachments.remove(name);
        for atts in skin.attachments.values_mut() {
            for a in atts.values_mut() {
                if let Attachment::Clipping(c) = a {
                    if c.end.as_deref() == Some(name) {
                        c.end = None;
                    }
                }
            }
        }
    }
    let paths: Vec<String> = skel
        .constraints
        .iter()
        .filter(|c| matches!(c, Constraint::Path(p) if p.target == name))
        .map(|c| c.name().to_string())
        .collect();
    for p in paths {
        remove_constraint(skel, &p);
    }
    for a in &mut skel.animations {
        a.slots.remove(name);
        for m in [&mut a.deform, &mut a.sequence] {
            for s in m.values_mut() {
                s.remove(name);
            }
            m.retain(|_, s| !s.is_empty());
        }
    }
    Ok(())
}

//! 첨부물 명령.

use super::refs::{self, check_name, slot_exists};
use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::{FromJson, JValue as Value, ToJson};
use crate::model::{Attachment, Skeleton, Skin};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("attachment.create", "첨부물", attachment_create),
    cmd("attachment.delete", "첨부물 삭제", attachment_delete),
    cmd("attachment.set", "첨부물 속성", attachment_set),
];

/// 첨부물 전체 값에 `props`의 키만 덮어쓴다. `type`을 다른 종류로 바꾸려 하면 거부.
fn apply_props(att: &Attachment, props: Option<&Value>) -> Result<Attachment> {
    let Some(Value::Obj(m)) = props else { return Ok(att.clone()) };
    let mut j = att.to_json(true);
    for (k, v) in m {
        if k == "type" {
            if v.as_str() != Some(att.type_name()) {
                return Err(CoreError::Rejected("첨부물 종류는 바꿀 수 없습니다".into()));
            }
            continue;
        }
        j.set(k, v.clone());
    }
    Attachment::from_json(&j).map_err(|e| CoreError::InvalidArgs(e.to_string()))
}

/// 링크드 메시의 부모가 같은 슬롯의 메시인지 확인한다.
fn check_linked(skel: &Skeleton, slot: &str, att: &Attachment) -> Result<()> {
    let Attachment::LinkedMesh(l) = att else { return Ok(()) };
    if l.parent.is_empty() {
        return Err(CoreError::InvalidArgs("링크드 메시는 'parent'(부모 메시 이름)가 필요합니다".into()));
    }
    let skin = l.skin.as_deref().unwrap_or(Skin::DEFAULT);
    match skel.skin(skin).and_then(|s| s.get(slot, &l.parent)) {
        Some(Attachment::Mesh(_)) => Ok(()),
        Some(_) => Err(CoreError::Rejected(format!("'{}'은(는) 메시가 아닙니다", l.parent))),
        None => Err(CoreError::NotFound(format!("스킨 '{skin}' 슬롯 '{slot}'의 부모 메시 '{}'", l.parent))),
    }
}

/// 이미지 크기에 맞춘 기본값(region: 폭·높이, mesh: 폭·높이 + 그 크기의 사각형 정점).
fn fit_image(att: &mut Attachment, w: f32, h: f32) {
    match att {
        Attachment::Region(r) => {
            r.width = w;
            r.height = h;
        }
        Attachment::Mesh(m) => {
            m.width = w;
            m.height = h;
            let (x, y) = (w / 2.0, h / 2.0);
            m.vertices = vec![-x, -y, x, -y, x, y, -x, y];
            m.uvs = vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
            m.triangles = vec![0, 1, 2, 0, 2, 3];
            m.hull = 4;
        }
        Attachment::LinkedMesh(l) => {
            l.width = w;
            l.height = h;
        }
        _ => {}
    }
}

fn skin_name<'a>(args: &Args<'a>) -> &'a str {
    args.opt_str("skin").unwrap_or(Skin::DEFAULT)
}

/// 인자: {"slot": "string", "name": "string", "type": "region|mesh|linkedmesh|boundingbox|point|clipping|path", "skin?": "string", "props?": "object", "show?": "bool(기본 true)"}
/// 결과: {"name"} (같은 스킨·슬롯에 이미 있으면 `이름2`…).
fn attachment_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let slot = args.str("slot")?;
    let given = args.str("name")?;
    check_name(given)?;
    let t = args.str("type")?;
    let skin = skin_name(args);
    check_name(skin)?;
    slot_exists(&ctx.project.skeleton, slot)?;
    let mut att = Attachment::new_of_type(t).ok_or_else(|| CoreError::InvalidArgs(format!("모르는 첨부물 종류: {t}")))?;
    let props = args.obj("props");
    let path = props.and_then(|p| p.get("path")).and_then(Value::as_str).unwrap_or(given);
    if let Some(img) = ctx.images.get(path) {
        fit_image(&mut att, img.width as f32, img.height as f32);
    }
    let att = apply_props(&att, props)?;
    let skel = &mut ctx.project.skeleton;
    check_linked(skel, slot, &att)?;
    let name = {
        let existing = skel.skin(skin).and_then(|s| s.attachments.get(slot));
        skel.unique_name(given, |n| existing.is_some_and(|m| m.contains_key(n)))
    };
    if skel.skin(skin).is_none() {
        if skin == Skin::DEFAULT {
            skel.default_skin_mut();
        } else {
            skel.skins.push(Skin::new(skin));
        }
    }
    skel.skin_mut(skin).expect("스킨").attachments.entry(slot).insert(name.clone(), att);
    if args.opt_bool("show").unwrap_or(true) {
        skel.slot_mut(slot).expect("슬롯").attachment = Some(name.clone());
    }
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string"}
/// 슬롯의 셋업 첨부물이면 비우고(다른 스킨에 같은 이름이 없을 때), deform·sequence 키를 지운다.
fn attachment_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let slot = args.str("slot")?;
    let name = args.str("name")?;
    let skin = skin_name(args);
    let skel = &mut ctx.project.skeleton;
    if skel.skin(skin).and_then(|s| s.get(slot, name)).is_none() {
        return Err(CoreError::NotFound(format!("스킨 '{skin}' 슬롯 '{slot}'의 첨부물 '{name}'")));
    }
    let used_as_parent = skel.skins.iter().any(|s| {
        s.attachments.get(slot).is_some_and(|m| {
            m.values().any(|a| matches!(a, Attachment::LinkedMesh(l) if l.parent == name && l.skin.as_deref().unwrap_or(Skin::DEFAULT) == skin))
        })
    });
    if used_as_parent {
        return Err(CoreError::Rejected("링크드 메시의 부모인 메시는 먼저 링크드 메시를 지우세요".into()));
    }
    let shared = refs::attachment_shared(skel, skin, slot, name);
    let s = skel.skin_mut(skin).expect("스킨");
    if let Some(m) = s.attachments.get_mut(slot) {
        m.remove(name);
        if m.is_empty() {
            s.attachments.remove(slot);
        }
    }
    if !shared {
        let sl = skel.slot_mut(slot).expect("슬롯");
        if sl.attachment.as_deref() == Some(name) {
            sl.attachment = None;
        }
    }
    for a in &mut skel.animations {
        for m in [&mut a.deform, &mut a.sequence] {
            if let Some(sk) = m.get_mut(skin) {
                if let Some(sl) = sk.get_mut(slot) {
                    sl.remove(name);
                }
                sk.retain(|_, sl| !sl.is_empty());
            }
            m.retain(|_, sk| !sk.is_empty());
        }
    }
    Ok(Value::Null)
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "props": "object(바꿀 필드만)"}
fn attachment_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let slot = args.str("slot")?;
    let name = args.str("name")?;
    let skin = skin_name(args);
    let props = args.obj("props").ok_or_else(|| CoreError::InvalidArgs("'props' 객체가 필요합니다".into()))?;
    let skel = &mut ctx.project.skeleton;
    let cur = skel
        .skin(skin)
        .and_then(|s| s.get(slot, name))
        .ok_or_else(|| CoreError::NotFound(format!("스킨 '{skin}' 슬롯 '{slot}'의 첨부물 '{name}'")))?;
    let att = apply_props(cur, Some(props))?;
    check_linked(skel, slot, &att)?;
    *skel.skin_mut(skin).expect("스킨").get_mut(slot, name).expect("첨부물") = att;
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::model::Attachment;
    use crate::{CoreError, Document};

    #[test]
    fn f_d01_region_create_uses_image_size_and_props() {
        let mut d = Document::new();
        d.set_image("몸/팔", 64, 20, None);
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        let r = run(&mut d, "attachment.create", r#"{"slot": "s", "name": "몸/팔", "type": "region", "props": {"x": 3, "rotation": 45}}"#);
        assert_eq!(r.get("name").unwrap().as_str(), Some("몸/팔"));
        let skel = &d.project().skeleton;
        assert_eq!(skel.slots[0].attachment.as_deref(), Some("몸/팔"));
        let Some(Attachment::Region(a)) = skel.skin("default").unwrap().get("s", "몸/팔") else { panic!() };
        assert_eq!((a.width, a.height, a.x, a.rotation), (64.0, 20.0, 3.0, 45.0));
        // 같은 이름은 겹치지 않게
        let r = run(&mut d, "attachment.create", r#"{"slot": "s", "name": "몸/팔", "type": "point"}"#);
        assert_eq!(r.get("name").unwrap().as_str(), Some("몸/팔2"));
        // attachment.set: 부분 덮어쓰기, 종류 변경 거부
        run(&mut d, "attachment.set", r#"{"slot": "s", "name": "몸/팔", "props": {"scaleX": 2}}"#);
        let Some(Attachment::Region(a)) = d.project().skeleton.skin("default").unwrap().get("s", "몸/팔") else { panic!() };
        assert_eq!((a.scale_x, a.width, a.x), (2.0, 64.0, 3.0));
        assert!(matches!(exec(&mut d, "attachment.set", r#"{"slot": "s", "name": "몸/팔", "props": {"type": "mesh"}}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "attachment.create", r#"{"slot": "s", "name": "x", "type": "모름"}"#), Err(CoreError::InvalidArgs(_))));
    }

    #[test]
    fn f_d08_mesh_from_image_and_linkedmesh_parent() {
        let mut d = Document::new();
        d.set_image("천", 40, 10, None);
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "천", "type": "mesh"}"#);
        let Some(Attachment::Mesh(m)) = d.project().skeleton.skin("default").unwrap().get("s", "천") else { panic!() };
        assert_eq!(m.vertices, vec![-20.0, -5.0, 20.0, -5.0, 20.0, 5.0, -20.0, 5.0]);
        assert_eq!((m.width, m.height, m.hull), (40.0, 10.0, 4));
        assert!(matches!(exec(&mut d, "attachment.create", r#"{"slot": "s", "name": "l", "type": "linkedmesh"}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(
            exec(&mut d, "attachment.create", r#"{"slot": "s", "name": "l", "type": "linkedmesh", "props": {"parent": "없음"}}"#),
            Err(CoreError::NotFound(_))
        ));
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "l", "type": "linkedmesh", "skin": "옷", "props": {"parent": "천", "timelines": false}}"#);
        let Some(Attachment::LinkedMesh(l)) = d.project().skeleton.skin("옷").unwrap().get("s", "l") else { panic!() };
        assert_eq!((l.parent.as_str(), l.timelines), ("천", false));
        // 링크드 메시의 부모는 먼저 지울 수 없다
        assert!(matches!(exec(&mut d, "attachment.delete", r#"{"slot": "s", "name": "천"}"#), Err(CoreError::Rejected(_))));
    }

    #[test]
    fn attachment_delete_clears_setup_and_deform_keys() {
        let mut d = Document::new();
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "m", "type": "mesh"}"#);
        run(&mut d, "animation.create", "{}");
        run(&mut d, "key.set", r#"{"target": {"kind": "deform", "slot": "s", "attachment": "m"}, "timeline": "", "time": 0, "v": [1, 1]}"#);
        assert!(!d.project().skeleton.animations[0].deform.is_empty());
        run(&mut d, "attachment.delete", r#"{"slot": "s", "name": "m"}"#);
        let skel = &d.project().skeleton;
        assert_eq!(skel.slots[0].attachment, None);
        assert!(skel.animations[0].deform.is_empty());
        assert!(skel.skin("default").unwrap().attachments.is_empty());
        assert!(matches!(exec(&mut d, "attachment.delete", r#"{"slot": "s", "name": "m"}"#), Err(CoreError::NotFound(_))));
    }
}

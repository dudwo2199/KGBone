//! 스킨 명령.

use super::refs::{bone_exists, pick_name};
use super::{cmd, state_cmd, Args, CommandContext, FnCommand};
use crate::json::JValue as Value;
use crate::model::{Attachment, Constraint, Skeleton, Skin};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("skin.create", "스킨", skin_create),
    cmd("skin.delete", "스킨 삭제", skin_delete),
    state_cmd("skin.setActive", "스킨 전환", skin_set_active),
    cmd("skin.addItem", "스킨에 본/제약 추가", skin_add_item),
    cmd("skin.removeItem", "스킨에서 본/제약 빼기", skin_remove_item),
];

fn skin_exists(skel: &Skeleton, name: &str) -> Result<()> {
    skel.skin(name).map(|_| ()).ok_or_else(|| CoreError::NotFound(format!("스킨 '{name}'")))
}

/// 인자: {"name?": "string"} → {"name"}
fn skin_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let skel = &mut ctx.project.skeleton;
    let name = pick_name(skel, args.opt_str("name"), "skin", |n| skel.skin(n).is_some())?;
    if name == Skin::DEFAULT {
        skel.default_skin_mut();
    } else {
        skel.skins.push(Skin::new(name.clone()));
    }
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"name": "string"} — 기본 스킨은 지울 수 없다. 이 스킨의 deform·sequence 키도 지운다.
fn skin_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    if name == Skin::DEFAULT {
        return Err(CoreError::Rejected("기본 스킨은 지울 수 없습니다".into()));
    }
    let skel = &mut ctx.project.skeleton;
    skin_exists(skel, name)?;
    let linked = skel.skins.iter().filter(|s| s.name != name).any(|s| {
        s.attachments.values().any(|m| m.values().any(|a| matches!(a, Attachment::LinkedMesh(l) if l.skin.as_deref() == Some(name))))
    });
    if linked {
        return Err(CoreError::Rejected("다른 스킨의 링크드 메시가 이 스킨의 메시를 쓰고 있습니다".into()));
    }
    let removed = skel.skins.iter().position(|s| s.name == name).map(|i| skel.skins.remove(i)).expect("스킨");
    for a in &mut skel.animations {
        a.deform.remove(name);
        a.sequence.remove(name);
    }
    // 스킨 전용 표시를 다시 계산한다.
    for b in &removed.bones {
        let still = skel.skins.iter().any(|s| s.bones.contains(b));
        if let Some(bone) = skel.bone_mut(b) {
            bone.skin_required = still;
        }
    }
    for c in &removed.constraints {
        let still = skel.skins.iter().any(|s| s.constraints.contains(c));
        if let Some(con) = skel.constraint_mut(c) {
            set_constraint_skin_required(con, still);
        }
    }
    if ctx.project.editor.active_skin.as_deref() == Some(name) {
        ctx.project.editor.active_skin = None;
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string|null"} — 되돌리기 대상 아님.
fn skin_set_active(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.opt_str("name");
    if let Some(n) = name {
        skin_exists(&ctx.project.skeleton, n)?;
    }
    ctx.project.editor.active_skin = name.map(String::from);
    Ok(Value::Null)
}

fn set_constraint_skin_required(c: &mut Constraint, v: bool) {
    match c {
        Constraint::Ik(c) => c.skin_required = v,
        Constraint::Transform(c) => c.skin_required = v,
        Constraint::Path(c) => c.skin_required = v,
        Constraint::Physics(c) => c.skin_required = v,
        Constraint::Slider(c) => c.skin_required = v,
    }
}

/// 본/제약을 스킨 목록에 넣거나 빼고, 어느 스킨에라도 들어 있는지에 따라 skinRequired를 맞춘다.
fn change_item(ctx: &mut CommandContext, args: &Args, add: bool) -> Result<Value> {
    let skin = args.str("skin")?;
    let kind = args.str("kind")?;
    let name = args.str("name")?;
    let skel = &mut ctx.project.skeleton;
    skin_exists(skel, skin)?;
    match kind {
        "bone" => bone_exists(skel, name)?,
        "constraint" => {
            skel.constraint(name).ok_or_else(|| CoreError::NotFound(format!("제약 '{name}'")))?;
        }
        _ => return Err(CoreError::InvalidArgs(format!("kind는 bone 또는 constraint여야 합니다: {kind}"))),
    }
    let s = skel.skin_mut(skin).expect("스킨");
    let list = if kind == "bone" { &mut s.bones } else { &mut s.constraints };
    if add {
        if !list.iter().any(|n| n == name) {
            list.push(name.to_string());
        }
    } else {
        list.retain(|n| n != name);
    }
    let required = skel.skins.iter().any(|s| if kind == "bone" { s.bones.iter().any(|n| n == name) } else { s.constraints.iter().any(|n| n == name) });
    if kind == "bone" {
        skel.bone_mut(name).expect("본").skin_required = required;
    } else {
        set_constraint_skin_required(skel.constraint_mut(name).expect("제약"), required);
    }
    Ok(Value::Null)
}

/// 인자: {"skin": "string", "kind": "bone|constraint", "name": "string"}
fn skin_add_item(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    change_item(ctx, args, true)
}

/// 인자: {"skin": "string", "kind": "bone|constraint", "name": "string"}
fn skin_remove_item(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    change_item(ctx, args, false)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::model::{Attachment, Skin};
    use crate::{CoreError, Document};

    /// 켜진 스킨에서 먼저 찾고, 없으면 기본 스킨(전환 규칙: 스킨 첨부물이 기본 스킨보다 우선).
    fn lookup<'a>(d: &'a Document, slot: &str, name: &str) -> Option<&'a Attachment> {
        let skel = &d.project().skeleton;
        let active = d.project().editor.active_skin.as_deref();
        active.and_then(|s| skel.skin(s)).and_then(|s| s.get(slot, name)).or_else(|| skel.skin(Skin::DEFAULT)?.get(slot, name))
    }

    #[test]
    fn f_e01_e02_skin_create_switch_and_per_skin_attachments() {
        let mut d = Document::new();
        assert_eq!(run(&mut d, "skin.create", "{}").get("name").unwrap().as_str(), Some("skin"));
        run(&mut d, "skin.create", r#"{"name": "빨강"}"#);
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "몸"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "몸", "name": "옷", "type": "region", "props": {"width": 10}}"#);
        run(&mut d, "attachment.create", r#"{"slot": "몸", "name": "옷", "type": "region", "skin": "빨강", "props": {"width": 20}}"#);
        let width = |d: &Document| match lookup(d, "몸", "옷") {
            Some(Attachment::Region(r)) => r.width,
            _ => panic!(),
        };
        assert_eq!(width(&d), 10.0);
        run(&mut d, "skin.setActive", r#"{"name": "빨강"}"#);
        assert_eq!(width(&d), 20.0);
        run(&mut d, "skin.setActive", r#"{"name": "skin"}"#);
        assert_eq!(width(&d), 10.0);
        assert!(matches!(exec(&mut d, "skin.setActive", r#"{"name": "없음"}"#), Err(CoreError::NotFound(_))));
        // 스킨 전환은 되돌리기 기록에 남지 않는다
        assert_eq!(d.undo_label(), Some("첨부물"));
        assert!(matches!(exec(&mut d, "skin.delete", r#"{"name": "default"}"#), Err(CoreError::Rejected(_))));
        run(&mut d, "skin.delete", r#"{"name": "skin"}"#);
        assert!(d.project().skeleton.skin("skin").is_none());
        assert_eq!(d.project().editor.active_skin, None);
        run(&mut d, "edit.undo", "{}");
        assert!(d.project().skeleton.skin("skin").is_some());
    }

    #[test]
    fn f_e03_skin_items_set_skin_required() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "b"}"#);
        run(&mut d, "bone.create", r#"{"name": "t"}"#);
        run(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["b"], "target": "t"}"#);
        run(&mut d, "skin.create", r#"{"name": "a"}"#);
        run(&mut d, "skin.create", r#"{"name": "c"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "a", "kind": "bone", "name": "b"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "c", "kind": "bone", "name": "b"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "a", "kind": "constraint", "name": "ik"}"#);
        let skel = &d.project().skeleton;
        assert!(skel.bone("b").unwrap().skin_required);
        assert!(skel.constraint("ik").unwrap().skin_required());
        assert_eq!(skel.skin("a").unwrap().bones, vec!["b".to_string()]);
        run(&mut d, "skin.removeItem", r#"{"skin": "a", "kind": "bone", "name": "b"}"#);
        assert!(d.project().skeleton.bone("b").unwrap().skin_required);
        run(&mut d, "skin.removeItem", r#"{"skin": "c", "kind": "bone", "name": "b"}"#);
        assert!(!d.project().skeleton.bone("b").unwrap().skin_required);
        run(&mut d, "skin.removeItem", r#"{"skin": "a", "kind": "constraint", "name": "ik"}"#);
        assert!(!d.project().skeleton.constraint("ik").unwrap().skin_required());
        assert!(matches!(exec(&mut d, "skin.addItem", r#"{"skin": "a", "kind": "slot", "name": "b"}"#), Err(CoreError::InvalidArgs(_))));
    }
}

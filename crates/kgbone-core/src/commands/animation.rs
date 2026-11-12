//! 애니메이션 명령.

use super::refs::pick_name;
use super::{cmd, state_cmd, Args, CommandContext, FnCommand};
use crate::json::JValue as Value;
use crate::model::{Animation, Constraint};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("animation.create", "애니메이션", animation_create),
    cmd("animation.delete", "애니메이션 삭제", animation_delete),
    cmd("animation.duplicate", "애니메이션 복제", animation_duplicate),
    state_cmd("animation.setActive", "애니메이션 고르기", animation_set_active),
];

/// 인자: {"name?": "string"} → {"name"}. 만든 애니메이션을 현재 애니메이션으로 고른다.
fn animation_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let skel = &mut ctx.project.skeleton;
    let name = pick_name(skel, args.opt_str("name"), "animation", |n| skel.animation(n).is_some())?;
    skel.animations.push(Animation::new(name.clone()));
    ctx.project.editor.active_animation = Some(name.clone());
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"name": "string"} — 슬라이더 제약이 쓰는 애니메이션은 지울 수 없다.
fn animation_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let skel = &mut ctx.project.skeleton;
    let i = skel.animations.iter().position(|a| a.name == name).ok_or_else(|| CoreError::NotFound(format!("애니메이션 '{name}'")))?;
    if let Some(c) = skel.constraints.iter().find(|c| matches!(c, Constraint::Slider(s) if s.animation == name)) {
        return Err(CoreError::Rejected(format!("슬라이더 제약 '{}'이(가) 쓰는 애니메이션입니다", c.name())));
    }
    skel.animations.remove(i);
    if ctx.project.editor.active_animation.as_deref() == Some(name) {
        ctx.project.editor.active_animation = None;
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string"} → {"name"} (`원래이름2` 식). 원본 바로 뒤에 넣는다.
fn animation_duplicate(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let skel = &mut ctx.project.skeleton;
    let i = skel.animations.iter().position(|a| a.name == name).ok_or_else(|| CoreError::NotFound(format!("애니메이션 '{name}'")))?;
    let new = skel.unique_name(name, |n| skel.animation(n).is_some());
    let mut copy = skel.animations[i].clone();
    copy.name = new.clone();
    skel.animations.insert(i + 1, copy);
    Ok(Value::obj().with("name", Value::Str(new)))
}

/// 인자: {"name": "string|null"} — 되돌리기 대상 아님.
fn animation_set_active(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.opt_str("name");
    if let Some(n) = name {
        if ctx.project.skeleton.animation(n).is_none() {
            return Err(CoreError::NotFound(format!("애니메이션 '{n}'")));
        }
    }
    ctx.project.editor.active_animation = name.map(String::from);
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::Project;
    use crate::{CoreError, Document};

    #[test]
    fn f_g01_animation_create_delete_rename_undo() {
        let mut d = Document::new();
        assert_eq!(run(&mut d, "animation.create", "{}").get("name").unwrap().as_str(), Some("animation"));
        assert_eq!(d.project().editor.active_animation.as_deref(), Some("animation"));
        run(&mut d, "animation.create", r#"{"name": "걷기"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "root"}, "timeline": "rotate", "time": 1, "v": [90]}"#);
        let r = run(&mut d, "animation.duplicate", r#"{"name": "걷기"}"#);
        assert_eq!(r.get("name").unwrap().as_str(), Some("걷기2"));
        let skel = &d.project().skeleton;
        let names: Vec<_> = skel.animations.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["animation", "걷기", "걷기2"]);
        assert_eq!(skel.animations[2].bones, skel.animations[1].bones);
        run(&mut d, "item.rename", r#"{"kind": "animation", "name": "걷기", "newName": "뛰기"}"#);
        assert_eq!(d.project().editor.active_animation.as_deref(), Some("뛰기"));
        run(&mut d, "animation.delete", r#"{"name": "뛰기"}"#);
        assert!(d.project().skeleton.animation("뛰기").is_none());
        assert_eq!(d.project().editor.active_animation, None);
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        assert_eq!(Project::from_json(&parse(&text).unwrap()).unwrap().skeleton.animations, d.project().skeleton.animations);
        run(&mut d, "edit.undo", "{}");
        assert!(d.project().skeleton.animation("뛰기").is_some());
        run(&mut d, "edit.undo", "{}");
        assert!(d.project().skeleton.animation("걷기").is_some());
        run(&mut d, "animation.setActive", r#"{"name": null}"#);
        assert_eq!(d.project().editor.active_animation, None);
        assert!(matches!(exec(&mut d, "animation.setActive", r#"{"name": "없음"}"#), Err(CoreError::NotFound(_))));
    }
}

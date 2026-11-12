//! 슬롯 명령.

use super::keying;
use super::refs::{self, bone_exists, pick_name};
use super::{cmd, draw_order, Args, CommandContext, FnCommand};
use crate::json::JValue as Value;
use crate::model::{BlendMode, Color, DarkColor, DrawOrderKey, Slot};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("slot.moveUp", "그리기 순서 위로", slot_move_up),
    cmd("slot.moveDown", "그리기 순서 아래로", slot_move_down),
    cmd("slot.create", "슬롯", slot_create),
    cmd("slot.delete", "슬롯 삭제", slot_delete),
    cmd("slot.setColor", "슬롯 색상", slot_set_color),
    cmd("slot.setDarkColor", "어두운 색", slot_set_dark_color),
    cmd("slot.setBlend", "블렌드 모드", slot_set_blend),
    cmd("slot.reorder", "그리기 순서 변경", slot_reorder),
    cmd("slot.setAttachment", "보이는 첨부물", slot_set_attachment),
    cmd("slot.setProps", "슬롯 속성", slot_set_props),
];

fn slot_ref<'a>(ctx: &'a CommandContext, name: &str) -> Result<&'a Slot> {
    ctx.project.skeleton.slot(name).ok_or_else(|| CoreError::NotFound(format!("슬롯 '{name}'")))
}

fn slot_mut<'a>(ctx: &'a mut CommandContext, name: &str) -> Result<&'a mut Slot> {
    ctx.project.skeleton.slot_mut(name).ok_or_else(|| CoreError::NotFound(format!("슬롯 '{name}'")))
}

fn parse_color(s: &str) -> Result<Color> {
    Color::parse_hex(s).ok_or_else(|| CoreError::InvalidArgs(format!("색 형식이 잘못됨: {s}")))
}

/// 슬롯을 그리기 순서의 `to` 자리로 옮긴다(배열 끝 = 맨 위). 애니메이트 모드면 그리기 순서 키를 만든다.
fn move_to(ctx: &mut CommandContext, name: &str, to: impl FnOnce(usize, usize) -> usize) -> Result<Value> {
    slot_ref(ctx, name)?;
    match keying::animate_target(ctx) {
        Some((anim, time)) => {
            let skel = &mut ctx.project.skeleton;
            let keys = &skel.animation(&anim).expect("애니메이션").draw_order;
            let mut order = draw_order::order_at(skel, keys, time);
            let from = order.iter().position(|n| n == name).expect("슬롯");
            let dest = to(from, order.len()).min(order.len() - 1);
            let n = order.remove(from);
            order.insert(dest, n);
            let offsets = draw_order::encode(&draw_order::setup_names(skel), &order);
            let keys = &mut keying::animation_mut(skel, &anim)?.draw_order;
            match keys.iter_mut().find(|k| (k.time - time).abs() < keying::TIME_EPS) {
                Some(k) => k.offsets = offsets,
                None => {
                    let i = keys.iter().position(|k| k.time > time).unwrap_or(keys.len());
                    keys.insert(i, DrawOrderKey { time, offsets });
                }
            }
        }
        None => {
            draw_order::preserving_keys(&mut ctx.project.skeleton, |skel| {
                let from = skel.slot_index(name).expect("슬롯");
                let dest = to(from, skel.slots.len()).min(skel.slots.len() - 1);
                let s = skel.slots.remove(from);
                skel.slots.insert(dest, s);
                Ok(())
            })?;
        }
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string"} — 한 칸 위로(배열에서 +1, 더 나중에 그려짐).
fn slot_move_up(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    move_to(ctx, args.str("name")?, |i, _| i + 1)
}

/// 인자: {"name": "string"} — 한 칸 아래로(배열에서 −1).
fn slot_move_down(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    move_to(ctx, args.str("name")?, |i, _| i.saturating_sub(1))
}

/// 인자: {"bone": "string", "name?": "string"} → {"name"}. 새 슬롯은 그리기 순서 맨 위(배열 끝).
fn slot_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let bone = args.str("bone")?;
    let skel = &mut ctx.project.skeleton;
    bone_exists(skel, bone)?;
    let name = pick_name(skel, args.opt_str("name"), "slot", |n| skel.slot(n).is_some())?;
    skel.slots.push(Slot::new(name.clone(), bone));
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"name": "string"} — 스킨 첨부물·타임라인·참조도 함께 지운다.
fn slot_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    slot_ref(ctx, name)?;
    refs::remove_slot(&mut ctx.project.skeleton, name)?;
    Ok(Value::Null)
}

/// 인자: {"name": "string", "color": "RRGGBBAA"}
fn slot_set_color(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let color = parse_color(args.str("color")?)?;
    match keying::animate_target(ctx) {
        Some((anim, time)) => {
            slot_ref(ctx, name)?;
            keying::key_slot_color(keying::animation_mut(&mut ctx.project.skeleton, &anim)?, name, time, color);
        }
        None => slot_mut(ctx, name)?.color = color,
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string", "dark": "RRGGBB|null"} — null이면 어두운 색을 끈다(셋업 모드에서만).
fn slot_set_dark_color(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let dark = args.opt_str("dark").map(parse_color).transpose()?;
    match keying::animate_target(ctx) {
        Some((anim, time)) => {
            let slot = slot_ref(ctx, name)?.clone();
            let dark = dark.ok_or_else(|| CoreError::Rejected("애니메이트 모드에서는 어두운 색을 끌 수 없습니다".into()))?;
            keying::key_slot_dark(keying::animation_mut(&mut ctx.project.skeleton, &anim)?, &slot, time, dark);
        }
        None => slot_mut(ctx, name)?.dark = dark.map(|c| DarkColor(Color { a: 255, ..c })),
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string", "blend": "normal|additive|multiply|screen"}
fn slot_set_blend(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let s = args.str("blend")?;
    let blend = BlendMode::parse(s).ok_or_else(|| CoreError::InvalidArgs(format!("모르는 블렌드 모드: {s}")))?;
    slot_mut(ctx, name)?.blend = blend;
    Ok(Value::Null)
}

/// 인자: {"name": "string", "index": "number"} — 그리기 순서의 index 자리로 옮긴다(0 = 맨 아래).
fn slot_reorder(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let index = args.opt_usize("index").ok_or_else(|| CoreError::InvalidArgs("'index'는 0 이상의 숫자여야 합니다".into()))?;
    move_to(ctx, name, |_, _| index)
}

/// 인자: {"name": "string", "attachment": "string|null"} — null이면 비운다.
fn slot_set_attachment(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let att = args.opt_str("attachment").map(String::from);
    slot_ref(ctx, name)?;
    if let Some(a) = &att {
        let skel = &ctx.project.skeleton;
        if !skel.skins.iter().any(|s| s.get(name, a).is_some()) {
            return Err(CoreError::NotFound(format!("슬롯 '{name}'의 첨부물 '{a}'")));
        }
    }
    match keying::animate_target(ctx) {
        Some((anim, time)) => keying::key_slot_attachment(keying::animation_mut(&mut ctx.project.skeleton, &anim)?, name, time, att),
        None => slot_mut(ctx, name)?.attachment = att,
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string", "visible?": "bool", "bone?": "string"}
fn slot_set_props(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    if let Some(b) = args.opt_str("bone") {
        bone_exists(&ctx.project.skeleton, b)?;
    }
    let slot = slot_mut(ctx, name)?;
    if let Some(v) = args.opt_bool("visible") {
        slot.visible = v;
    }
    if let Some(b) = args.opt_str("bone") {
        slot.bone = b.to_string();
    }
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::draw_order;
    use super::super::test_util::{exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::Project;
    use crate::{CoreError, Document};

    fn order(d: &Document) -> Vec<String> {
        d.project().skeleton.slots.iter().map(|s| s.name.clone()).collect()
    }

    fn reopen(d: &Document) -> Project {
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        Project::from_json(&parse(&text).unwrap()).unwrap()
    }

    #[test]
    fn f_c01_slot_create_delete_undo() {
        let mut d = Document::new();
        let r = run(&mut d, "slot.create", r#"{"bone": "root"}"#);
        assert_eq!(r.get("name").unwrap().as_str(), Some("slot"));
        let r = run(&mut d, "slot.create", r#"{"bone": "root"}"#);
        assert_eq!(r.get("name").unwrap().as_str(), Some("slot2"));
        assert!(matches!(exec(&mut d, "slot.create", r#"{"bone": "없음"}"#), Err(CoreError::NotFound(_))));
        assert!(matches!(exec(&mut d, "slot.create", "{}"), Err(CoreError::InvalidArgs(_))));
        run(&mut d, "attachment.create", r#"{"slot": "slot", "name": "a", "type": "region"}"#);
        run(&mut d, "animation.create", "{}");
        run(&mut d, "key.set", r#"{"target": {"kind": "slot", "name": "slot"}, "timeline": "attachment", "time": 0, "s": "a"}"#);
        let before = d.project().skeleton.clone();
        run(&mut d, "slot.delete", r#"{"name": "slot"}"#);
        let skel = &d.project().skeleton;
        assert_eq!(order(&d), ["slot2"]);
        assert!(skel.skin("default").unwrap().attachments.get("slot").is_none());
        assert!(skel.animations[0].slots.is_empty());
        run(&mut d, "edit.undo", "{}");
        assert_eq!(d.project().skeleton, before);
    }

    #[test]
    fn f_c02_c03_c04_color_dark_blend_roundtrip() {
        let mut d = Document::new();
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        run(&mut d, "slot.setColor", r#"{"name": "s", "color": "11223344"}"#);
        run(&mut d, "slot.setDarkColor", r#"{"name": "s", "dark": "AABBCC"}"#);
        run(&mut d, "slot.setBlend", r#"{"name": "s", "blend": "screen"}"#);
        assert!(matches!(exec(&mut d, "slot.setBlend", r#"{"name": "s", "blend": "x"}"#), Err(CoreError::InvalidArgs(_))));
        let json = d.project().skeleton.slots[0].to_json(false);
        assert_eq!(json.get("color").unwrap().as_str(), Some("11223344"));
        assert_eq!(json.get("dark").unwrap().as_str(), Some("AABBCC"));
        assert_eq!(json.get("blend").unwrap().as_str(), Some("screen"));
        assert_eq!(reopen(&d).skeleton, d.project().skeleton);
        // 끄면 필드가 없어진다
        run(&mut d, "slot.setDarkColor", r#"{"name": "s", "dark": null}"#);
        assert!(d.project().skeleton.slots[0].to_json(false).get("dark").is_none());
    }

    #[test]
    fn f_c02_animate_mode_color_keys() {
        let mut d = Document::new();
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        run(&mut d, "animation.create", r#"{"name": "a"}"#);
        run(&mut d, "mode.set", r#"{"mode": "animate"}"#);
        run(&mut d, "time.set", r#"{"time": 1}"#);
        run(&mut d, "slot.setColor", r#"{"name": "s", "color": "FF000000"}"#);
        run(&mut d, "slot.setDarkColor", r#"{"name": "s", "dark": "00FF00"}"#);
        let skel = &d.project().skeleton;
        assert_eq!(skel.slots[0].color.to_hex_rgba(), "FFFFFFFF");
        let tl = skel.animations[0].slots.get("s").unwrap();
        assert_eq!(tl.get("rgba").unwrap()[0].v, vec![1.0, 0.0, 0.0, 0.0]);
        // rgba2 = 현재 밝은 색(rgba 키 값) + 어두운 색
        assert_eq!(tl.get("rgba2").unwrap()[0].v, vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        assert!(matches!(exec(&mut d, "slot.setDarkColor", r#"{"name": "s", "dark": null}"#), Err(CoreError::Rejected(_))));
    }

    #[test]
    fn f_c05_reorder_move_up_down_undo() {
        let mut d = Document::new();
        for n in ["a", "b", "c", "d"] {
            run(&mut d, "slot.create", &format!(r#"{{"bone": "root", "name": "{n}"}}"#));
        }
        run(&mut d, "slot.reorder", r#"{"name": "a", "index": 2}"#);
        assert_eq!(order(&d), ["b", "c", "a", "d"]);
        run(&mut d, "slot.moveUp", r#"{"name": "a"}"#);
        assert_eq!(order(&d), ["b", "c", "d", "a"]);
        run(&mut d, "slot.moveUp", r#"{"name": "a"}"#);
        assert_eq!(order(&d), ["b", "c", "d", "a"]);
        run(&mut d, "slot.moveDown", r#"{"name": "b"}"#);
        assert_eq!(order(&d), ["b", "c", "d", "a"]);
        run(&mut d, "slot.moveDown", r#"{"name": "d"}"#);
        assert_eq!(order(&d), ["b", "d", "c", "a"]);
        run(&mut d, "edit.undo", "{}");
        assert_eq!(order(&d), ["b", "c", "d", "a"]);
        run(&mut d, "edit.undo", "{}");
        run(&mut d, "edit.undo", "{}");
        assert_eq!(order(&d), ["a", "b", "c", "d"]);
        assert_eq!(reopen(&d).skeleton.slots, d.project().skeleton.slots);
    }

    #[test]
    fn f_c05_animate_mode_draw_order_key_and_setup_reorder_keeps_keys() {
        let mut d = Document::new();
        for n in ["a", "b", "c"] {
            run(&mut d, "slot.create", &format!(r#"{{"bone": "root", "name": "{n}"}}"#));
        }
        run(&mut d, "animation.create", r#"{"name": "x"}"#);
        run(&mut d, "mode.set", r#"{"mode": "animate"}"#);
        run(&mut d, "time.set", r#"{"time": 0.25}"#);
        run(&mut d, "slot.moveUp", r#"{"name": "a"}"#);
        assert_eq!(order(&d), ["a", "b", "c"]);
        let skel = &d.project().skeleton;
        let keys = &skel.animations[0].draw_order;
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].time, 0.25);
        assert_eq!(draw_order::decode(&draw_order::setup_names(skel), &keys[0].offsets), ["b", "a", "c"]);
        // 같은 시점에서 한 번 더: 키를 고친다
        run(&mut d, "slot.moveUp", r#"{"name": "a"}"#);
        let skel = &d.project().skeleton;
        assert_eq!(skel.animations[0].draw_order.len(), 1);
        assert_eq!(draw_order::decode(&draw_order::setup_names(skel), &skel.animations[0].draw_order[0].offsets), ["b", "c", "a"]);
        // 셋업 순서를 바꿔도 키가 뜻하는 순서는 그대로
        run(&mut d, "mode.set", r#"{"mode": "setup"}"#);
        run(&mut d, "slot.reorder", r#"{"name": "c", "index": 0}"#);
        let skel = &d.project().skeleton;
        assert_eq!(order(&d), ["c", "a", "b"]);
        assert_eq!(draw_order::decode(&draw_order::setup_names(skel), &skel.animations[0].draw_order[0].offsets), ["b", "c", "a"]);
    }

    #[test]
    fn slot_set_attachment_checks_name_and_keys_in_animate_mode() {
        let mut d = Document::new();
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "img", "type": "region", "show": false}"#);
        assert_eq!(d.project().skeleton.slots[0].attachment, None);
        assert!(matches!(exec(&mut d, "slot.setAttachment", r#"{"name": "s", "attachment": "없음"}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "slot.setAttachment", r#"{"name": "s", "attachment": "img"}"#);
        assert_eq!(d.project().skeleton.slots[0].attachment.as_deref(), Some("img"));
        run(&mut d, "animation.create", "{}");
        run(&mut d, "mode.set", r#"{"mode": "animate"}"#);
        run(&mut d, "slot.setAttachment", r#"{"name": "s", "attachment": null}"#);
        let skel = &d.project().skeleton;
        assert_eq!(skel.slots[0].attachment.as_deref(), Some("img"));
        let k = &skel.animations[0].slots.get("s").unwrap().get("attachment").unwrap()[0];
        assert_eq!((k.time, k.s.clone()), (0.0, None));
        run(&mut d, "mode.set", r#"{"mode": "setup"}"#);
        run(&mut d, "bone.create", r#"{"name": "b"}"#);
        run(&mut d, "slot.setProps", r#"{"name": "s", "bone": "b", "visible": false}"#);
        assert_eq!((d.project().skeleton.slots[0].bone.as_str(), d.project().skeleton.slots[0].visible), ("b", false));
    }
}

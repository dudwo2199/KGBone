//! 이름 바꾸기 명령.

use super::refs::{self, check_name};
use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::JValue as Value;
use crate::model::Skin;
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("item.rename", "이름 바꾸기", item_rename),
];

/// 인자: {"kind": "bone|slot|constraint|skin|animation|event|attachment", "name": "string", "newName": "string", "skin?": "string", "slot?": "string"}
/// 이름을 가리키는 모든 참조를 함께 고친다. 이미 있는 이름이면 거부. 첨부물은 skin(기본 default)·slot이 필요하다.
fn item_rename(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let kind = args.str("kind")?;
    let old = args.str("name")?;
    let new = args.str("newName")?;
    check_name(new)?;
    if old == new {
        return Ok(Value::Null);
    }
    let skel = &mut ctx.project.skeleton;
    let taken = |exists: bool, what: &str| -> Result<()> {
        if exists {
            Err(CoreError::Rejected(format!("같은 이름의 {what}이(가) 이미 있습니다: {new}")))
        } else {
            Ok(())
        }
    };
    let missing = |what: &str| CoreError::NotFound(format!("{what} '{old}'"));
    match kind {
        "bone" => {
            skel.bone(old).ok_or_else(|| missing("본"))?;
            taken(skel.bone(new).is_some(), "본")?;
            refs::rename_bone(skel, old, new);
        }
        "slot" => {
            skel.slot(old).ok_or_else(|| missing("슬롯"))?;
            taken(skel.slot(new).is_some(), "슬롯")?;
            refs::rename_slot(skel, old, new);
        }
        "constraint" => {
            skel.constraint(old).ok_or_else(|| missing("제약"))?;
            taken(skel.constraint(new).is_some(), "제약")?;
            refs::rename_constraint(skel, old, new);
        }
        "skin" => {
            skel.skin(old).ok_or_else(|| missing("스킨"))?;
            if old == Skin::DEFAULT || new == Skin::DEFAULT {
                return Err(CoreError::Rejected("기본 스킨(default)의 이름은 바꿀 수 없습니다".into()));
            }
            taken(skel.skin(new).is_some(), "스킨")?;
            refs::rename_skin(skel, old, new);
            if ctx.project.editor.active_skin.as_deref() == Some(old) {
                ctx.project.editor.active_skin = Some(new.to_string());
            }
        }
        "animation" => {
            skel.animation(old).ok_or_else(|| missing("애니메이션"))?;
            taken(skel.animation(new).is_some(), "애니메이션")?;
            refs::rename_animation(skel, old, new);
            if ctx.project.editor.active_animation.as_deref() == Some(old) {
                ctx.project.editor.active_animation = Some(new.to_string());
            }
        }
        "event" => {
            skel.event(old).ok_or_else(|| missing("이벤트"))?;
            taken(skel.event(new).is_some(), "이벤트")?;
            refs::rename_event(skel, old, new);
        }
        "attachment" => {
            let skin = args.opt_str("skin").unwrap_or(Skin::DEFAULT);
            let slot = args.str("slot")?;
            let s = skel.skin(skin).ok_or_else(|| CoreError::NotFound(format!("스킨 '{skin}'")))?;
            s.get(slot, old).ok_or_else(|| missing("첨부물"))?;
            taken(s.get(slot, new).is_some(), "첨부물")?;
            refs::rename_attachment(skel, skin, slot, old, new);
        }
        _ => return Err(CoreError::InvalidArgs(format!("모르는 종류: {kind}"))),
    }
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::draw_order;
    use super::super::test_util::{exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::{Attachment, Constraint, Project};
    use crate::{CoreError, Document};

    fn reopen(d: &Document) -> Project {
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        Project::from_json(&parse(&text).unwrap()).unwrap()
    }

    /// 본·슬롯·제약·스킨·애니메이션·이벤트·첨부물이 서로 참조하는 뼈대.
    fn rig() -> Document {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "팔"}"#);
        run(&mut d, "bone.create", r#"{"name": "손", "parent": "팔"}"#);
        run(&mut d, "bone.create", r#"{"name": "목표"}"#);
        run(&mut d, "slot.create", r#"{"bone": "손", "name": "손슬롯"}"#);
        run(&mut d, "slot.create", r#"{"bone": "팔", "name": "길"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "손슬롯", "name": "손그림", "type": "mesh"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "손슬롯", "name": "자르기", "type": "clipping", "show": false, "props": {"end": "손슬롯"}}"#);
        run(&mut d, "attachment.create", r#"{"slot": "길", "name": "p", "type": "path"}"#);
        run(&mut d, "skin.create", r#"{"name": "옷"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "손슬롯", "name": "링크", "type": "linkedmesh", "skin": "옷", "show": false, "props": {"parent": "손그림"}}"#);
        run(&mut d, "constraint.create", r#"{"type": "ik", "name": "팔IK", "bones": ["팔", "손"], "target": "목표"}"#);
        run(&mut d, "constraint.create", r#"{"type": "path", "name": "길따라", "bones": ["목표"], "target": "길"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "옷", "kind": "bone", "name": "손"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "옷", "kind": "constraint", "name": "팔IK"}"#);
        run(&mut d, "event.create", r#"{"name": "발소리"}"#);
        run(&mut d, "animation.create", r#"{"name": "걷기"}"#);
        run(&mut d, "constraint.create", r#"{"type": "slider", "name": "슬", "animation": "걷기", "bone": "손"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "손"}, "timeline": "rotate", "v": [1]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "slot", "name": "손슬롯"}, "timeline": "attachment", "s": "손그림"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "constraint", "name": "팔IK"}, "timeline": "ik", "v": [1, 0, 1, 0, 0]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "deform", "slot": "손슬롯", "attachment": "손그림"}, "timeline": "", "v": [1]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "deform", "skin": "옷", "slot": "손슬롯", "attachment": "링크"}, "timeline": "", "v": [1]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "event", "name": "발소리"}, "timeline": "", "time": 0.5}"#);
        run(&mut d, "slot.reorder", r#"{"name": "길", "index": 0}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "drawOrder"}, "timeline": "", "time": 1, "offsets": [{"slot": "손슬롯", "offset": -1}]}"#);
        d
    }

    #[test]
    fn f_b07_rename_bone_korean_updates_refs() {
        let mut d = rig();
        run(&mut d, "item.rename", r#"{"kind": "bone", "name": "손", "newName": "왼손"}"#);
        let skel = &d.project().skeleton;
        assert!(skel.bone("손").is_none());
        assert_eq!(skel.slot("손슬롯").unwrap().bone, "왼손");
        assert!(matches!(skel.constraint("팔IK"), Some(Constraint::Ik(c)) if c.bones == ["팔", "왼손"]));
        assert!(matches!(skel.constraint("슬"), Some(Constraint::Slider(c)) if c.bone.as_deref() == Some("왼손")));
        assert_eq!(skel.skin("옷").unwrap().bones, ["왼손"]);
        assert!(skel.animations[0].bones.get("왼손").is_some());
        // 한글 이름 저장·다시 열기 후 같다
        assert_eq!(reopen(&d).skeleton, d.project().skeleton);
        assert!(matches!(exec(&mut d, "item.rename", r#"{"kind": "bone", "name": "왼손", "newName": "팔"}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "item.rename", r#"{"kind": "bone", "name": "왼손", "newName": ""}"#), Err(CoreError::InvalidArgs(_))));
        // 부모 참조
        run(&mut d, "item.rename", r#"{"kind": "bone", "name": "팔", "newName": "윗팔"}"#);
        assert_eq!(d.project().skeleton.bone("왼손").unwrap().parent.as_deref(), Some("윗팔"));
        run(&mut d, "edit.undo", "{}");
        run(&mut d, "edit.undo", "{}");
        assert!(d.project().skeleton.bone("손").is_some());
    }

    #[test]
    fn rename_slot_updates_refs() {
        let mut d = rig();
        let order_before = {
            let s = &d.project().skeleton;
            draw_order::decode(&draw_order::setup_names(s), &s.animations[0].draw_order[0].offsets)
        };
        assert_eq!(order_before, ["손슬롯", "길"]);
        run(&mut d, "item.rename", r#"{"kind": "slot", "name": "손슬롯", "newName": "오른손"}"#);
        run(&mut d, "item.rename", r#"{"kind": "slot", "name": "길", "newName": "선"}"#);
        let skel = &d.project().skeleton;
        assert!(skel.skin("default").unwrap().attachments.get("오른손").is_some());
        assert!(skel.skin("옷").unwrap().attachments.get("오른손").is_some());
        assert!(matches!(skel.skin("default").unwrap().get("오른손", "자르기"), Some(Attachment::Clipping(c)) if c.end.as_deref() == Some("오른손")));
        assert!(matches!(skel.constraint("길따라"), Some(Constraint::Path(p)) if p.target == "선"));
        let a = &skel.animations[0];
        assert!(a.slots.get("오른손").is_some());
        assert!(a.deform.get("default").unwrap().get("오른손").is_some());
        assert!(a.deform.get("옷").unwrap().get("오른손").is_some());
        assert_eq!(draw_order::decode(&draw_order::setup_names(skel), &a.draw_order[0].offsets), ["오른손", "선"]);
    }

    #[test]
    fn rename_constraint_skin_animation_event() {
        let mut d = rig();
        run(&mut d, "skin.setActive", r#"{"name": "옷"}"#);
        run(&mut d, "item.rename", r#"{"kind": "constraint", "name": "팔IK", "newName": "팔잡기"}"#);
        run(&mut d, "item.rename", r#"{"kind": "skin", "name": "옷", "newName": "갑옷"}"#);
        run(&mut d, "item.rename", r#"{"kind": "animation", "name": "걷기", "newName": "산책"}"#);
        run(&mut d, "item.rename", r#"{"kind": "event", "name": "발소리", "newName": "쿵"}"#);
        let p = d.project();
        let skel = &p.skeleton;
        assert_eq!(skel.skin("갑옷").unwrap().constraints, ["팔잡기"]);
        let a = skel.animation("산책").unwrap();
        assert!(a.constraints.get("팔잡기").is_some());
        assert!(a.deform.get("갑옷").is_some());
        assert_eq!(a.events[0].name, "쿵");
        assert_eq!(skel.events[0].name, "쿵");
        assert!(matches!(skel.constraint("슬"), Some(Constraint::Slider(s)) if s.animation == "산책"));
        assert_eq!(p.editor.active_skin.as_deref(), Some("갑옷"));
        assert_eq!(p.editor.active_animation.as_deref(), Some("산책"));
        assert!(matches!(exec(&mut d, "item.rename", r#"{"kind": "skin", "name": "default", "newName": "x"}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "item.rename", r#"{"kind": "event", "name": "없음", "newName": "x"}"#), Err(CoreError::NotFound(_))));
        assert_eq!(reopen(&d).skeleton, d.project().skeleton);
    }

    #[test]
    fn rename_attachment_updates_refs_and_keeps_image() {
        let mut d = rig();
        run(&mut d, "item.rename", r#"{"kind": "attachment", "slot": "손슬롯", "name": "손그림", "newName": "주먹"}"#);
        let skel = &d.project().skeleton;
        let def = skel.skin("default").unwrap();
        assert!(def.get("손슬롯", "손그림").is_none());
        // 이미지 경로는 옛 이름 그대로
        assert!(matches!(def.get("손슬롯", "주먹"), Some(Attachment::Mesh(m)) if m.path.as_deref() == Some("손그림")));
        assert_eq!(skel.slot("손슬롯").unwrap().attachment.as_deref(), Some("주먹"));
        assert!(matches!(skel.skin("옷").unwrap().get("손슬롯", "링크"), Some(Attachment::LinkedMesh(l)) if l.parent == "주먹"));
        let a = &skel.animations[0];
        assert_eq!(a.slots.get("손슬롯").unwrap().get("attachment").unwrap()[0].s.as_deref(), Some("주먹"));
        assert!(a.deform.get("default").unwrap().get("손슬롯").unwrap().get("주먹").is_some());
        assert!(matches!(
            exec(&mut d, "item.rename", r#"{"kind": "attachment", "slot": "손슬롯", "name": "주먹", "newName": "자르기"}"#),
            Err(CoreError::Rejected(_))
        ));
    }
}

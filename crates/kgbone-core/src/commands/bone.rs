//! 본 명령.

use super::bone_order::{remap_weights, sort_parents_first, weights_use};
use super::keying::{self, BoneEdit};
use super::refs::{self, bone_exists, pick_name};
use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::JValue as Value;
use crate::model::{Bone, Color, Inherit};
use crate::pose::{setup_world, world_to_local_transform};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("bone.create", "본", bone_create),
    cmd("bone.delete", "본 삭제", bone_delete),
    cmd("bone.setParent", "부모 변경", bone_set_parent),
    cmd("bone.setTransform", "본 변환", bone_set_transform),
    cmd("bone.setInherit", "상속 설정", bone_set_inherit),
    cmd("bone.setColor", "본 색상", bone_set_color),
    cmd("bone.setLength", "본 길이", bone_set_length),
    cmd("bone.setProps", "본 속성", bone_set_props),
];

fn bone_mut<'a>(ctx: &'a mut CommandContext, name: &str) -> Result<&'a mut Bone> {
    ctx.project.skeleton.bone_mut(name).ok_or_else(|| CoreError::NotFound(format!("본 '{name}'")))
}

/// 인자: {"parent?": "string", "name?": "string", "x?": "number", "y?": "number", "rotation?": "number", "length?": "number"}
/// 결과: {"name": 만든 이름}. 부모 기본값은 루트(첫 본). 새 본은 배열 끝에 붙는다(부모가 항상 앞).
fn bone_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let skel = &mut ctx.project.skeleton;
    let parent = match args.opt_str("parent") {
        Some(p) => {
            bone_exists(skel, p)?;
            Some(p.to_string())
        }
        None => skel.bones.first().map(|b| b.name.clone()),
    };
    let name = pick_name(skel, args.opt_str("name"), "bone", |n| skel.bone(n).is_some())?;
    let length = args.opt_f32("length").unwrap_or(0.0);
    if length < 0.0 {
        return Err(CoreError::InvalidArgs("길이는 0 이상이어야 합니다".into()));
    }
    let mut b = Bone::new(name.clone(), parent);
    b.x = args.opt_f32("x").unwrap_or(0.0);
    b.y = args.opt_f32("y").unwrap_or(0.0);
    b.rotation = args.opt_f32("rotation").unwrap_or(0.0);
    b.length = length;
    skel.bones.push(b);
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"name": "string"}
/// 자손 본, 그 본들에 붙은 슬롯(과 그 참조), 그 본들을 참조하는 제약(과 그 참조), 스킨 bones 목록, 본 타임라인을 함께 지운다.
fn bone_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let skel = &mut ctx.project.skeleton;
    let idx = skel.bone_index(name).ok_or_else(|| CoreError::NotFound(format!("본 '{name}'")))?;
    if idx == 0 && skel.bones[0].parent.is_none() {
        return Err(CoreError::Rejected("루트 본은 지울 수 없습니다".into()));
    }
    let doomed: Vec<bool> = skel.bones.iter().map(|b| skel.is_descendant(&b.name, name)).collect();
    if weights_use(skel, |i| doomed.get(i).copied().unwrap_or(false)) {
        return Err(CoreError::Rejected("가중치에 쓰인 본은 먼저 가중치를 해제하세요".into()));
    }
    let names: Vec<String> = skel.bones.iter().zip(&doomed).filter(|(_, d)| **d).map(|(b, _)| b.name.clone()).collect();
    let gone = |n: &str| names.iter().any(|x| x == n);

    let slots: Vec<String> = skel.slots.iter().filter(|s| gone(&s.bone)).map(|s| s.name.clone()).collect();
    for s in &slots {
        refs::remove_slot(skel, s)?;
    }
    let constraints: Vec<String> =
        skel.constraints.iter().filter(|c| c.referenced_bones().into_iter().any(gone)).map(|c| c.name().to_string()).collect();
    for c in &constraints {
        refs::remove_constraint(skel, c);
    }
    for s in &mut skel.skins {
        s.bones.retain(|b| !gone(b));
    }
    for a in &mut skel.animations {
        for n in &names {
            a.bones.remove(n);
        }
    }
    // 남은 본의 새 번호.
    let mut new_of = vec![0usize; doomed.len()];
    let mut next = 0;
    for (i, d) in doomed.iter().enumerate() {
        new_of[i] = next;
        if !d {
            next += 1;
        }
    }
    let mut i = 0;
    skel.bones.retain(|_| {
        i += 1;
        !doomed[i - 1]
    });
    remap_weights(skel, |old| new_of.get(old).copied().unwrap_or(old));
    Ok(Value::Null)
}

/// 인자: {"name": "string", "parent": "string", "keepWorld?": "bool(기본 true)"}
/// keepWorld면 셋업 월드 변환이 그대로 남도록 새 부모 아래 로컬 값을 다시 계산한다.
fn bone_set_parent(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let parent = args.str("parent")?;
    let skel = &mut ctx.project.skeleton;
    bone_exists(skel, name)?;
    bone_exists(skel, parent)?;
    if skel.is_descendant(parent, name) {
        return Err(CoreError::Rejected("자기 자신이나 자손 본을 부모로 지정할 수 없습니다".into()));
    }
    if args.opt_bool("keepWorld").unwrap_or(true) {
        let world = setup_world(skel);
        let i = skel.bone_index(name).expect("본");
        let pi = skel.bone_index(parent).expect("부모");
        let local = world_to_local_transform(&world[i], Some(&world[pi]), skel.bones[i].inherit);
        skel.bones[i].set_local(local);
    }
    skel.bone_mut(name).expect("본").parent = Some(parent.to_string());
    sort_parents_first(skel);
    Ok(Value::Null)
}

/// 인자: {"name": "string", "x?": "number", "y?": "number", "rotation?": "number", "scaleX?": "number", "scaleY?": "number", "shearX?": "number", "shearY?": "number"}
/// 애니메이트 모드면 셋업 값 대신 현재 시점에 상대값 키를 만든다.
fn bone_set_transform(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let edit = BoneEdit {
        x: args.opt_f32("x"),
        y: args.opt_f32("y"),
        rotation: args.opt_f32("rotation"),
        scale_x: args.opt_f32("scaleX"),
        scale_y: args.opt_f32("scaleY"),
        shear_x: args.opt_f32("shearX"),
        shear_y: args.opt_f32("shearY"),
    };
    let target = keying::animate_target(ctx);
    let bone = bone_mut(ctx, name)?;
    match target {
        Some((anim, time)) => {
            let setup = bone.clone();
            let anim = keying::animation_mut(&mut ctx.project.skeleton, &anim)?;
            keying::key_bone_edit(anim, &setup, time, &edit);
        }
        None => {
            let set = |dst: &mut f32, v: Option<f32>| {
                if let Some(v) = v {
                    *dst = v
                }
            };
            set(&mut bone.x, edit.x);
            set(&mut bone.y, edit.y);
            set(&mut bone.rotation, edit.rotation);
            set(&mut bone.scale_x, edit.scale_x);
            set(&mut bone.scale_y, edit.scale_y);
            set(&mut bone.shear_x, edit.shear_x);
            set(&mut bone.shear_y, edit.shear_y);
        }
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string", "inherit": "normal|onlyTranslation|noRotationOrReflection|noScale|noScaleOrReflection"}
fn bone_set_inherit(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let s = args.str("inherit")?;
    let inherit = Inherit::parse(s).ok_or_else(|| CoreError::InvalidArgs(format!("모르는 상속 종류: {s}")))?;
    bone_mut(ctx, name)?.inherit = inherit;
    Ok(Value::Null)
}

/// 인자: {"name": "string", "color": "RRGGBBAA"}
fn bone_set_color(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let s = args.str("color")?;
    let color = Color::parse_hex(s).ok_or_else(|| CoreError::InvalidArgs(format!("색 형식이 잘못됨: {s}")))?;
    bone_mut(ctx, name)?.color = color;
    Ok(Value::Null)
}

/// 인자: {"name": "string", "length": "number"}
fn bone_set_length(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let length = args.f32("length")?;
    if length < 0.0 || !length.is_finite() {
        return Err(CoreError::InvalidArgs("길이는 0 이상이어야 합니다".into()));
    }
    bone_mut(ctx, name)?.length = length;
    Ok(Value::Null)
}

/// 인자: {"name": "string", "skinRequired?": "bool", "icon?": "string|null", "visible?": "bool"}
fn bone_set_props(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let bone = bone_mut(ctx, name)?;
    if let Some(v) = args.opt_bool("skinRequired") {
        bone.skin_required = v;
    }
    if args.has("icon") {
        bone.icon = args.opt_str("icon").filter(|s| !s.is_empty()).map(String::from);
    }
    if let Some(v) = args.opt_bool("visible") {
        bone.visible = v;
    }
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{close, exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::Project;
    use crate::pose::setup_world;
    use crate::{CoreError, Document};

    #[test]
    fn f_a01_animate_mode_keys_instead_of_setup() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "arm", "x": 10, "rotation": 30}"#);
        run(&mut d, "animation.create", r#"{"name": "walk"}"#);
        run(&mut d, "mode.set", r#"{"mode": "animate"}"#);
        run(&mut d, "time.set", r#"{"time": 0.5}"#);
        run(&mut d, "bone.setTransform", r#"{"name": "arm", "x": 15, "rotation": 45, "scaleX": 2}"#);
        let skel = &d.project().skeleton;
        let arm = skel.bone("arm").unwrap();
        // 셋업 값 그대로
        assert_eq!((arm.x, arm.rotation, arm.scale_x), (10.0, 30.0, 1.0));
        let tl = skel.animation("walk").unwrap().bones.get("arm").unwrap();
        // rotate = 45 − 30 = 15, translate = (15 − 10, 0 유지) = (5, 0), scale = 2 / 1 = 2, scaleY는 현재 값 1 유지
        assert_eq!(tl.get("rotate").unwrap()[0].v, vec![15.0]);
        assert_eq!(tl.get("rotate").unwrap()[0].time, 0.5);
        assert_eq!(tl.get("translate").unwrap()[0].v, vec![5.0, 0.0]);
        assert_eq!(tl.get("scale").unwrap()[0].v, vec![2.0, 1.0]);
        assert!(tl.get("shear").is_none());
        // 같은 시점에서 y만 바꾸면 키를 새로 만들지 않고 고친다: (5 유지, 3 − 0 = 3)
        run(&mut d, "bone.setTransform", r#"{"name": "arm", "y": 3}"#);
        let tl = d.project().skeleton.animation("walk").unwrap().bones.get("arm").unwrap();
        assert_eq!(tl.get("translate").unwrap().len(), 1);
        assert_eq!(tl.get("translate").unwrap()[0].v, vec![5.0, 3.0]);
        // 셋업 모드로 돌아오면 셋업 값이 바뀐다
        run(&mut d, "mode.toggle", "{}");
        run(&mut d, "bone.setTransform", r#"{"name": "arm", "x": 20}"#);
        assert_eq!(d.project().skeleton.bone("arm").unwrap().x, 20.0);
        assert_eq!(d.project().skeleton.animation("walk").unwrap().bones.get("arm").unwrap().get("translate").unwrap()[0].v, vec![5.0, 3.0]);
    }

    #[test]
    fn f_b01_bone_create_unique_names_and_undo() {
        let mut d = Document::new();
        let a = run(&mut d, "bone.create", "{}");
        let b = run(&mut d, "bone.create", "{}");
        assert_eq!(a.get("name").unwrap().as_str(), Some("bone"));
        assert_eq!(b.get("name").unwrap().as_str(), Some("bone2"));
        let c = run(&mut d, "bone.create", r#"{"parent": "bone2", "name": "bone"}"#);
        assert_eq!(c.get("name").unwrap().as_str(), Some("bone3"));
        let skel = &d.project().skeleton;
        assert_eq!(skel.bone("bone").unwrap().parent.as_deref(), Some("root"));
        assert_eq!(skel.bone("bone3").unwrap().parent.as_deref(), Some("bone2"));
        assert!(matches!(exec(&mut d, "bone.create", r#"{"name": " "}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "bone.create", r#"{"parent": "없음"}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "edit.undo", "{}");
        assert!(d.project().skeleton.bone("bone3").is_none());
        assert_eq!(d.project().skeleton.bones.len(), 3);
        run(&mut d, "edit.redo", "{}");
        assert!(d.project().skeleton.bone("bone3").is_some());
    }

    #[test]
    fn f_b02_bone_delete_with_children_slots_constraints_keys() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "a"}"#);
        run(&mut d, "bone.create", r#"{"name": "b", "parent": "a"}"#);
        run(&mut d, "bone.create", r#"{"name": "t"}"#);
        run(&mut d, "bone.create", r#"{"name": "keep"}"#);
        run(&mut d, "slot.create", r#"{"bone": "b", "name": "sb"}"#);
        run(&mut d, "slot.create", r#"{"bone": "keep", "name": "sk"}"#);
        run(&mut d, "attachment.create", r#"{"slot": "sb", "name": "img", "type": "region"}"#);
        run(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["t"], "target": "b", "name": "ik1"}"#);
        run(&mut d, "skin.create", r#"{"name": "s2"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "s2", "kind": "bone", "name": "b"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "s2", "kind": "constraint", "name": "ik1"}"#);
        run(&mut d, "animation.create", r#"{"name": "anim"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "b"}, "timeline": "rotate", "time": 0, "v": [10]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "keep"}, "timeline": "rotate", "time": 0, "v": [5]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "slot", "name": "sb"}, "timeline": "rgba", "time": 0, "v": [1, 0, 0, 1]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "constraint", "name": "ik1"}, "timeline": "ik", "time": 0, "v": [0.5, 0, 1, 0, 0]}"#);
        let before = d.project().skeleton.clone();

        run(&mut d, "bone.delete", r#"{"name": "a"}"#);
        let skel = &d.project().skeleton;
        let names: Vec<_> = skel.bones.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["root", "t", "keep"]);
        assert_eq!(skel.slots.len(), 1);
        assert_eq!(skel.slots[0].name, "sk");
        assert!(skel.constraints.is_empty());
        let s2 = skel.skin("s2").unwrap();
        assert!(s2.bones.is_empty() && s2.constraints.is_empty());
        assert!(skel.skin("default").unwrap().attachments.get("sb").is_none());
        let anim = skel.animation("anim").unwrap();
        assert!(anim.bones.get("b").is_none() && anim.bones.get("keep").is_some());
        assert!(anim.slots.is_empty() && anim.constraints.is_empty());

        run(&mut d, "edit.undo", "{}");
        assert_eq!(d.project().skeleton, before);
        // 루트 본은 지울 수 없다
        assert!(matches!(exec(&mut d, "bone.delete", r#"{"name": "root"}"#), Err(CoreError::Rejected(_))));
    }

    #[test]
    fn f_b02_bone_delete_rejects_weighted_and_remaps_indices() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "a"}"#);
        run(&mut d, "bone.create", r#"{"name": "b"}"#);
        run(&mut d, "slot.create", r#"{"bone": "root", "name": "s"}"#);
        // 본 번호 2(b)를 쓰는 가중치 메시
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "m", "type": "mesh", "props": {"weighted": true, "vertices": [1, 2, 0, 0, 1, 1, 2, 5, 0, 1, 1, 2, 5, 5, 1, 1, 2, 0, 5, 1]}}"#);
        assert_eq!(exec(&mut d, "bone.delete", r#"{"name": "b"}"#), Err(CoreError::Rejected("가중치에 쓰인 본은 먼저 가중치를 해제하세요".into())));
        // a를 지우면 b의 번호가 2 → 1
        run(&mut d, "bone.delete", r#"{"name": "a"}"#);
        let skel = &d.project().skeleton;
        let (_, v) = skel.skin("default").unwrap().get("s", "m").unwrap().vertices().unwrap();
        assert_eq!(v[1], 1.0);
        assert_eq!(v[6], 1.0);
    }

    #[test]
    fn f_b03_set_parent_keeps_world_and_rejects_descendant() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "p"}"#);
        run(&mut d, "bone.create", r#"{"name": "c", "x": 5, "y": 5, "rotation": 30}"#);
        run(&mut d, "bone.create", r#"{"name": "cc", "parent": "c", "x": 3}"#);
        // 회전된 부모: p = (10, 0), 90°
        run(&mut d, "bone.setTransform", r#"{"name": "p", "x": 10, "rotation": 90}"#);
        // c를 p 앞으로 끌어오기 위해 p를 c 아래로 두지 않고, c → p로 옮긴다. 이때 배열 순서가 [root, p, c, cc]라 이미 맞다.
        // 순서가 깨지는 경우도 시험하려고 먼저 p를 cc 뒤로 보낸다: p의 부모를 cc로(월드 유지)
        let w0 = setup_world(&d.project().skeleton);
        run(&mut d, "bone.setParent", r#"{"name": "p", "parent": "cc"}"#);
        let skel = &d.project().skeleton;
        let names: Vec<_> = skel.bones.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["root", "c", "cc", "p"]);
        let w1 = setup_world(skel);
        let wp0 = w0[1];
        let wp1 = w1[3];
        assert!(close(wp0.x, wp1.x) && close(wp0.y, wp1.y) && close(wp0.a, wp1.a) && close(wp0.c, wp1.c), "{wp0:?} {wp1:?}");
        run(&mut d, "edit.undo", "{}");

        // c(월드 (5,5), 30°)를 p((10,0), 90°) 아래로: 로컬 = R(−90°)·((5,5) − (10,0)) = R(−90°)·(−5, 5) = (5, 5), 회전 30 − 90 = −60
        run(&mut d, "bone.setParent", r#"{"name": "c", "parent": "p"}"#);
        let skel = &d.project().skeleton;
        let c = skel.bone("c").unwrap();
        assert!(close(c.x, 5.0) && close(c.y, 5.0) && close(c.rotation, -60.0), "{c:?}");
        let w = setup_world(skel);
        let ci = skel.bone_index("c").unwrap();
        assert!(close(w[ci].x, 5.0) && close(w[ci].y, 5.0) && close(w[ci].rotation_x(), 30.0));
        // 자식 cc의 월드 위치도 그대로: (5,5) + 3·(cos30, sin30) = (7.598, 6.5)
        let cci = skel.bone_index("cc").unwrap();
        assert!(close(w[cci].x, 7.598) && close(w[cci].y, 6.5), "{:?}", w[cci]);

        assert!(matches!(exec(&mut d, "bone.setParent", r#"{"name": "c", "parent": "cc"}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "bone.setParent", r#"{"name": "c", "parent": "c"}"#), Err(CoreError::Rejected(_))));
    }

    #[test]
    fn f_b06_color_length_props_roundtrip() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "a"}"#);
        run(&mut d, "bone.setColor", r#"{"name": "a", "color": "ff0080cc"}"#);
        run(&mut d, "bone.setLength", r#"{"name": "a", "length": 42.5}"#);
        run(&mut d, "bone.setInherit", r#"{"name": "a", "inherit": "noScale"}"#);
        run(&mut d, "bone.setProps", r#"{"name": "a", "icon": "손", "visible": false, "skinRequired": true}"#);
        assert!(matches!(exec(&mut d, "bone.setLength", r#"{"name": "a", "length": -1}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "bone.setColor", r#"{"name": "a", "color": "zz"}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "bone.setInherit", r#"{"name": "a", "inherit": "x"}"#), Err(CoreError::InvalidArgs(_))));
        // 파일 글자로 썼다가 다시 읽어도 같다
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        let back = Project::from_json(&parse(&text).unwrap()).unwrap();
        let a = back.skeleton.bone("a").unwrap();
        assert_eq!(a.color.to_hex_rgba(), "FF0080CC");
        assert_eq!(a.length, 42.5);
        assert_eq!(a.inherit, crate::model::Inherit::NoScale);
        assert_eq!((a.icon.as_deref(), a.visible, a.skin_required), (Some("손"), false, true));
        assert_eq!(&back.skeleton, &d.project().skeleton);
    }

    #[test]
    fn f_i03_history_group_set_transform_is_one_undo() {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "a"}"#);
        run(&mut d, "history.begin", r#"{"label": "드래그"}"#);
        for x in 1..=5 {
            run(&mut d, "bone.setTransform", &format!(r#"{{"name": "a", "x": {x}}}"#));
        }
        run(&mut d, "history.end", "{}");
        assert_eq!(d.project().skeleton.bone("a").unwrap().x, 5.0);
        let undone = run(&mut d, "edit.undo", "{}");
        assert_eq!(undone.get("label").unwrap().as_str(), Some("드래그"));
        assert_eq!(d.project().skeleton.bone("a").unwrap().x, 0.0);
        run(&mut d, "edit.undo", "{}");
        assert!(d.project().skeleton.bone("a").is_none());
    }
}

//! 제약 명령.

use super::keying;
use super::refs::{self, bone_exists, pick_name, slot_exists};
use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::{FromJson, JValue as Value, ToJson};
use crate::model::{Constraint, Skeleton};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("constraint.create", "제약", constraint_create),
    cmd("constraint.delete", "제약 삭제", constraint_delete),
    cmd("constraint.set", "제약 속성", constraint_set),
    cmd("constraint.reorder", "제약 순서 변경", constraint_reorder),
];

fn constraint_exists(skel: &Skeleton, name: &str) -> Result<()> {
    skel.constraint(name).map(|_| ()).ok_or_else(|| CoreError::NotFound(format!("제약 '{name}'")))
}

/// 제약이 가리키는 본·슬롯·애니메이션이 모두 있고, 종류별 규칙에 맞는지 확인한다.
fn check_refs(skel: &Skeleton, c: &Constraint) -> Result<()> {
    let need = |s: &str, what: &str| {
        if s.is_empty() {
            Err(CoreError::InvalidArgs(format!("'{what}'가 필요합니다")))
        } else {
            Ok(())
        }
    };
    match c {
        Constraint::Ik(ik) => {
            if ik.bones.is_empty() || ik.bones.len() > 2 {
                return Err(CoreError::InvalidArgs("IK 제약의 본은 1개 또는 2개여야 합니다".into()));
            }
            need(&ik.target, "target")?;
            if ik.bones.contains(&ik.target) {
                return Err(CoreError::Rejected("IK 목표 본은 제약을 받는 본과 달라야 합니다".into()));
            }
        }
        Constraint::Transform(t) => {
            if t.bones.is_empty() {
                return Err(CoreError::InvalidArgs("트랜스폼 제약은 본이 하나 이상 필요합니다".into()));
            }
            need(&t.source, "source")?;
            if t.bones.contains(&t.source) {
                return Err(CoreError::Rejected("원본 본은 제약을 받는 본과 달라야 합니다".into()));
            }
        }
        Constraint::Path(p) => {
            if p.bones.is_empty() {
                return Err(CoreError::InvalidArgs("패스 제약은 본이 하나 이상 필요합니다".into()));
            }
            need(&p.target, "target")?;
            slot_exists(skel, &p.target)?;
        }
        Constraint::Physics(p) => need(&p.bone, "bone")?,
        Constraint::Slider(s) => {
            need(&s.animation, "animation")?;
            if skel.animation(&s.animation).is_none() {
                return Err(CoreError::NotFound(format!("애니메이션 '{}'", s.animation)));
            }
        }
    }
    for b in c.referenced_bones() {
        bone_exists(skel, b)?;
    }
    if let Constraint::Ik(ik) = c {
        if ik.bones.len() == 2 && skel.bone(&ik.bones[1]).and_then(|b| b.parent.as_deref()) != Some(ik.bones[0].as_str()) {
            return Err(CoreError::Rejected("2본 IK의 두 번째 본은 첫 번째 본의 자식이어야 합니다".into()));
        }
    }
    Ok(())
}

/// 인자: {"type": "ik|transform|path|physics|slider", "name?": "string", "bones?": "string[]", "target?": "string", "source?": "string", "bone?": "string", "animation?": "string"}
/// 결과: {"name"}. 이름 기본값은 종류 이름. 적용 순서 맨 끝에 붙는다.
fn constraint_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let t = args.str("type")?;
    if !Constraint::TYPES.contains(&t) {
        return Err(CoreError::InvalidArgs(format!("모르는 제약 종류: {t}")));
    }
    let skel = &mut ctx.project.skeleton;
    let name = pick_name(skel, args.opt_str("name"), t, |n| skel.constraint(n).is_some())?;
    let bones = args.str_list("bones");
    let mut j = Value::obj().with("type", Value::str(t)).with("name", Value::Str(name.clone()));
    let str_or_empty = |k: &str| Value::str(args.opt_str(k).unwrap_or(""));
    match t {
        "ik" => j = j.with("bones", bones.to_json(true)).with("target", str_or_empty("target")),
        "transform" => j = j.with("bones", bones.to_json(true)).with("source", str_or_empty("source")),
        "path" => j = j.with("bones", bones.to_json(true)).with("target", str_or_empty("target")),
        "physics" => {
            let bone = args.opt_str("bone").map(String::from).or_else(|| bones.first().cloned()).unwrap_or_default();
            j = j.with("bone", Value::Str(bone));
        }
        _ => {
            let anim = match args.opt_str("animation") {
                Some(a) => a.to_string(),
                None => skel
                    .animations
                    .first()
                    .map(|a| a.name.clone())
                    .ok_or_else(|| CoreError::Rejected("슬라이더에 쓸 애니메이션이 없습니다. 먼저 애니메이션을 만드세요".into()))?,
            };
            j = j.with("animation", Value::Str(anim));
            if let Some(b) = args.opt_str("bone") {
                j = j.with("bone", Value::str(b));
            }
        }
    }
    let c = Constraint::from_json(&j).map_err(|e| CoreError::InvalidArgs(e.to_string()))?;
    check_refs(skel, &c)?;
    skel.constraints.push(c);
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"name": "string"} — 스킨 constraints 목록과 애니메이션 제약 타임라인도 지운다.
fn constraint_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    constraint_exists(&ctx.project.skeleton, name)?;
    refs::remove_constraint(&mut ctx.project.skeleton, name);
    Ok(Value::Null)
}

/// 인자: {"name": "string", "props": "object(바꿀 필드만)"}
/// 애니메이트 모드면 혼합류 값(mix 등)은 셋업 대신 현재 시점에 제약 타임라인 키로 넣는다.
fn constraint_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let props = args.obj("props").and_then(Value::as_object).ok_or_else(|| CoreError::InvalidArgs("'props' 객체가 필요합니다".into()))?;
    let target = keying::animate_target(ctx);
    let skel = &mut ctx.project.skeleton;
    let cur = skel.constraint(name).ok_or_else(|| CoreError::NotFound(format!("제약 '{name}'")))?.clone();
    let keyable = keying::keyable_props(&cur);
    let mut j = cur.to_json(true);
    let mut keyed: Vec<(String, Value)> = vec![];
    for (k, v) in props {
        match k.as_str() {
            "type" if v.as_str() != Some(cur.type_name()) => return Err(CoreError::Rejected("제약 종류는 바꿀 수 없습니다".into())),
            "name" if v.as_str() != Some(name) => return Err(CoreError::Rejected("제약 이름은 '이름 바꾸기'로 바꾸세요".into())),
            "type" | "name" => {}
            _ if target.is_some() && keyable.iter().any(|(p, _, _)| p == k) => keyed.push((k.clone(), v.clone())),
            _ => j.set(k, v.clone()),
        }
    }
    let c = Constraint::from_json(&j).map_err(|e| CoreError::InvalidArgs(e.to_string()))?;
    check_refs(skel, &c)?;
    *skel.constraint_mut(name).expect("제약") = c.clone();
    if let (Some((anim, time)), false) = (target, keyed.is_empty()) {
        keying::key_constraint_props(keying::animation_mut(skel, &anim)?, &c, time, &keyed);
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string", "index": "number"} — 적용 순서의 index 자리로 옮긴다.
fn constraint_reorder(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let index = args.opt_usize("index").ok_or_else(|| CoreError::InvalidArgs("'index'는 0 이상의 숫자여야 합니다".into()))?;
    let skel = &mut ctx.project.skeleton;
    let from = skel.constraint_index(name).ok_or_else(|| CoreError::NotFound(format!("제약 '{name}'")))?;
    let c = skel.constraints.remove(from);
    let to = index.min(skel.constraints.len());
    skel.constraints.insert(to, c);
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::{Constraint, Project};
    use crate::{CoreError, Document};

    fn doc() -> Document {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "a"}"#);
        run(&mut d, "bone.create", r#"{"name": "a2", "parent": "a"}"#);
        run(&mut d, "bone.create", r#"{"name": "t"}"#);
        d
    }

    fn names(d: &Document) -> Vec<String> {
        d.project().skeleton.constraints.iter().map(|c| c.name().to_string()).collect()
    }

    #[test]
    fn constraint_create_checks_required_refs() {
        let mut d = doc();
        assert_eq!(run(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["a", "a2"], "target": "t"}"#).get("name").unwrap().as_str(), Some("ik"));
        assert_eq!(run(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["a"], "target": "t"}"#).get("name").unwrap().as_str(), Some("ik2"));
        assert!(matches!(exec(&mut d, "constraint.create", r#"{"type": "ik", "bones": [], "target": "t"}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["a"]}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["a", "t"], "target": "a2"}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "constraint.create", r#"{"type": "transform", "bones": ["a"], "source": "없음"}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "constraint.create", r#"{"type": "transform", "bones": ["a"], "source": "t"}"#);
        assert!(matches!(exec(&mut d, "constraint.create", r#"{"type": "path", "bones": ["a"], "target": "s"}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "slot.create", r#"{"bone": "t", "name": "s"}"#);
        run(&mut d, "constraint.create", r#"{"type": "path", "bones": ["a"], "target": "s"}"#);
        let r = run(&mut d, "constraint.create", r#"{"type": "physics", "bones": ["a2"]}"#);
        assert!(matches!(d.project().skeleton.constraint(r.get("name").unwrap().as_str().unwrap()), Some(Constraint::Physics(p)) if p.bone == "a2"));
        assert!(matches!(exec(&mut d, "constraint.create", r#"{"type": "slider"}"#), Err(CoreError::Rejected(_))));
        run(&mut d, "animation.create", r#"{"name": "흔들기"}"#);
        run(&mut d, "constraint.create", r#"{"type": "slider"}"#);
        assert!(matches!(d.project().skeleton.constraint("slider"), Some(Constraint::Slider(s)) if s.animation == "흔들기"));
        assert!(matches!(exec(&mut d, "animation.delete", r#"{"name": "흔들기"}"#), Err(CoreError::Rejected(_))));
        assert_eq!(names(&d), ["ik", "ik2", "transform", "path", "physics", "slider"]);
    }

    #[test]
    fn f_f07_constraint_reorder_roundtrip() {
        let mut d = doc();
        run(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["a"], "target": "t", "name": "c1"}"#);
        run(&mut d, "constraint.create", r#"{"type": "transform", "bones": ["a2"], "source": "t", "name": "c2"}"#);
        run(&mut d, "constraint.create", r#"{"type": "physics", "bone": "t", "name": "c3"}"#);
        run(&mut d, "constraint.reorder", r#"{"name": "c3", "index": 0}"#);
        assert_eq!(names(&d), ["c3", "c1", "c2"]);
        run(&mut d, "constraint.reorder", r#"{"name": "c3", "index": 99}"#);
        assert_eq!(names(&d), ["c1", "c2", "c3"]);
        run(&mut d, "constraint.reorder", r#"{"name": "c2", "index": 0}"#);
        assert_eq!(names(&d), ["c2", "c1", "c3"]);
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        let back = Project::from_json(&parse(&text).unwrap()).unwrap();
        assert_eq!(back.skeleton.constraints, d.project().skeleton.constraints);
        run(&mut d, "edit.undo", "{}");
        assert_eq!(names(&d), ["c1", "c2", "c3"]);
    }

    #[test]
    fn constraint_set_setup_and_animate_mode() {
        let mut d = doc();
        run(&mut d, "constraint.create", r#"{"type": "ik", "bones": ["a"], "target": "t"}"#);
        run(&mut d, "constraint.set", r#"{"name": "ik", "props": {"mix": 0.5, "stretch": true}}"#);
        assert!(matches!(d.project().skeleton.constraint("ik"), Some(Constraint::Ik(c)) if c.mix == 0.5 && c.stretch));
        assert!(matches!(exec(&mut d, "constraint.set", r#"{"name": "ik", "props": {"type": "path"}}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "constraint.set", r#"{"name": "ik", "props": {"target": "없음"}}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "animation.create", r#"{"name": "x"}"#);
        run(&mut d, "mode.set", r#"{"mode": "animate"}"#);
        run(&mut d, "time.set", r#"{"time": 2}"#);
        // mix·bendPositive는 키, uniform은 셋업
        run(&mut d, "constraint.set", r#"{"name": "ik", "props": {"mix": 0.25, "bendPositive": false, "uniform": true}}"#);
        let skel = &d.project().skeleton;
        assert!(matches!(skel.constraint("ik"), Some(Constraint::Ik(c)) if c.mix == 0.5 && c.bend_positive && c.uniform));
        let k = &skel.animations[0].constraints.get("ik").unwrap().get("ik").unwrap()[0];
        // [mix, softness, bendPositive, compress, stretch] = [0.25, 0(셋업), −1, 0(셋업), 1(셋업 stretch=true)]
        assert_eq!((k.time, k.v.clone()), (2.0, vec![0.25, 0.0, -1.0, 0.0, 1.0]));
    }

    #[test]
    fn constraint_delete_cleans_refs() {
        let mut d = doc();
        run(&mut d, "constraint.create", r#"{"type": "transform", "bones": ["a"], "source": "t", "name": "tc"}"#);
        run(&mut d, "skin.create", r#"{"name": "s"}"#);
        run(&mut d, "skin.addItem", r#"{"skin": "s", "kind": "constraint", "name": "tc"}"#);
        run(&mut d, "animation.create", "{}");
        run(&mut d, "key.set", r#"{"target": {"kind": "constraint", "name": "tc"}, "timeline": "transform", "v": [1, 1, 1, 1, 1, 1]}"#);
        run(&mut d, "constraint.delete", r#"{"name": "tc"}"#);
        let skel = &d.project().skeleton;
        assert!(skel.constraints.is_empty());
        assert!(skel.skin("s").unwrap().constraints.is_empty());
        assert!(skel.animations[0].constraints.is_empty());
    }
}

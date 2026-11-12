//! 이벤트 명령.

use super::refs::pick_name;
use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::{FromJson, JValue as Value, ToJson};
use crate::model::EventDef;
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("event.create", "이벤트", event_create),
    cmd("event.delete", "이벤트 삭제", event_delete),
    cmd("event.set", "이벤트 속성", event_set),
];

/// 이벤트 정의에 `props`의 키를 덮어쓴다(이름은 바꾸지 않는다).
fn apply_props(def: &EventDef, props: Option<&Value>) -> Result<EventDef> {
    let Some(Value::Obj(m)) = props else { return Ok(def.clone()) };
    let mut j = def.to_json(true);
    for (k, v) in m {
        if k == "name" {
            if v.as_str() != Some(def.name.as_str()) {
                return Err(CoreError::Rejected("이벤트 이름은 '이름 바꾸기'로 바꾸세요".into()));
            }
            continue;
        }
        j.set(k, v.clone());
    }
    EventDef::from_json(&j).map_err(|e| CoreError::InvalidArgs(e.to_string()))
}

/// 인자: {"name?": "string", "props?": "object"} → {"name"}
fn event_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let skel = &mut ctx.project.skeleton;
    let name = pick_name(skel, args.opt_str("name"), "event", |n| skel.event(n).is_some())?;
    let mut props = args.obj("props").cloned();
    if let Some(p) = &mut props {
        p.remove("name");
    }
    let def = apply_props(&EventDef::new(name.clone()), props.as_ref())?;
    skel.events.push(def);
    Ok(Value::obj().with("name", Value::Str(name)))
}

/// 인자: {"name": "string"} — 모든 애니메이션의 이 이벤트 키도 지운다.
fn event_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let skel = &mut ctx.project.skeleton;
    let i = skel.events.iter().position(|e| e.name == name).ok_or_else(|| CoreError::NotFound(format!("이벤트 '{name}'")))?;
    skel.events.remove(i);
    for a in &mut skel.animations {
        a.events.retain(|k| k.name != name);
    }
    Ok(Value::Null)
}

/// 인자: {"name": "string", "props": "object"}
fn event_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let name = args.str("name")?;
    let props = args.obj("props").ok_or_else(|| CoreError::InvalidArgs("'props' 객체가 필요합니다".into()))?;
    let skel = &mut ctx.project.skeleton;
    let def = skel.events.iter_mut().find(|e| e.name == name).ok_or_else(|| CoreError::NotFound(format!("이벤트 '{name}'")))?;
    *def = apply_props(def, Some(props))?;
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::Project;
    use crate::{CoreError, Document};

    #[test]
    fn f_h01_event_create_set_roundtrip_korean() {
        let mut d = Document::new();
        let r = run(&mut d, "event.create", r#"{"name": "발소리", "props": {"int": 3, "string": "쿵", "volume": 0.5}}"#);
        assert_eq!(r.get("name").unwrap().as_str(), Some("발소리"));
        assert_eq!(run(&mut d, "event.create", "{}").get("name").unwrap().as_str(), Some("event"));
        run(&mut d, "event.set", r#"{"name": "발소리", "props": {"float": 1.5, "audio": "소리/쿵"}}"#);
        assert!(matches!(exec(&mut d, "event.set", r#"{"name": "발소리", "props": {"name": "x"}}"#), Err(CoreError::Rejected(_))));
        let e = d.project().skeleton.event("발소리").unwrap().clone();
        assert_eq!((e.int, e.float, e.string.as_str(), e.audio.as_str(), e.volume), (3, 1.5, "쿵", "소리/쿵", 0.5));
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        let back = Project::from_json(&parse(&text).unwrap()).unwrap();
        assert_eq!(back.skeleton.events, d.project().skeleton.events);
    }

    #[test]
    fn f_h02_event_key_only_stores_non_default_and_delete_cleans() {
        let mut d = Document::new();
        run(&mut d, "event.create", r#"{"name": "e", "props": {"int": 3}}"#);
        run(&mut d, "animation.create", "{}");
        run(&mut d, "key.set", r#"{"target": {"kind": "event", "name": "e"}, "timeline": "", "time": 0.5, "int": 3, "float": 2}"#);
        let k = &d.project().skeleton.animations[0].events[0];
        // int 3 = 정의 기본값이라 저장하지 않음, float 2는 다르므로 저장
        assert_eq!((k.time, k.name.as_str(), k.int, k.float), (0.5, "e", None, Some(2.0)));
        assert!(matches!(exec(&mut d, "key.set", r#"{"target": {"kind": "event", "name": "없음"}, "timeline": ""}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "event.delete", r#"{"name": "e"}"#);
        assert!(d.project().skeleton.animations[0].events.is_empty());
        assert!(d.project().skeleton.events.is_empty());
    }
}

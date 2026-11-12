//! 키프레임 명령.
//!
//! 키 하나를 가리키는 KeyRef = `{target, timeline, time}` (시간은 |Δt| < 1e-4로 찾는다).
//! target.kind: bone | slot | constraint | deform | sequence | drawOrder | event.
//! drawOrder·event는 timeline을 무시한다. event 키의 이벤트 이름은 target.name(또는 s).

use super::keying::{self, TIME_EPS};
use super::refs::{bone_exists, slot_exists};
use super::{cmd, Args, CommandContext, FnCommand};
use crate::json::{FromJson, JValue as Value};
use crate::model::{timeline, Animation, Constraint, Curve, DrawOrderKey, DrawOrderOffset, EventKey, Key, Skeleton, Skin};
use crate::pose::{PoseEngine, PoseRequest};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("key.edited", "선택 항목 키 지정", key_edited),
    cmd("key.constrained", "제약 적용 값 키 지정", key_constrained),
    cmd("key.all", "전부 키 지정", key_all),
    cmd("key.set", "키 지정", key_set),
    cmd("key.delete", "키 삭제", key_delete),
    cmd("key.move", "키 이동", key_move),
    cmd("key.setCurve", "보간 지정", key_set_curve),
    cmd("key.paste", "키 붙여넣기", key_paste),
];

/// 키가 들어 있는 곳.
#[derive(Clone, Debug, PartialEq)]
enum Target {
    Bone(String),
    Slot(String),
    /// 이름 ""은 모든 물리 제약에 적용되는 전체 물리 키.
    Constraint(String),
    Deform { skin: String, slot: String, attachment: String },
    Sequence { skin: String, slot: String, attachment: String },
    DrawOrder,
    Event(Option<String>),
}

fn parse_target(v: Option<&Value>) -> Result<Target> {
    let v = v.filter(|v| v.as_object().is_some()).ok_or_else(|| CoreError::InvalidArgs("'target' 객체가 필요합니다".into()))?;
    let a = Args::new(v);
    let kind = a.str("kind")?;
    let att = |a: &Args| -> Result<(String, String, String)> {
        Ok((a.opt_str("skin").unwrap_or(Skin::DEFAULT).to_string(), a.str("slot")?.to_string(), a.str("attachment")?.to_string()))
    };
    Ok(match kind {
        "bone" => Target::Bone(a.str("name")?.to_string()),
        "slot" => Target::Slot(a.str("name")?.to_string()),
        "constraint" => Target::Constraint(a.opt_str("name").unwrap_or("").to_string()),
        "deform" => {
            let (skin, slot, attachment) = att(&a)?;
            Target::Deform { skin, slot, attachment }
        }
        "sequence" => {
            let (skin, slot, attachment) = att(&a)?;
            Target::Sequence { skin, slot, attachment }
        }
        "drawOrder" => Target::DrawOrder,
        "event" => Target::Event(a.opt_str("name").map(String::from)),
        other => return Err(CoreError::InvalidArgs(format!("모르는 키 대상 종류: {other}"))),
    })
}

/// 대상이 있고 타임라인 이름이 그 대상에 맞는지 확인한다.
fn check_timeline(skel: &Skeleton, t: &Target, tl: &str) -> Result<()> {
    let allowed: &[&str] = match t {
        Target::Bone(n) => {
            bone_exists(skel, n)?;
            timeline::BONE
        }
        Target::Slot(n) => {
            slot_exists(skel, n)?;
            timeline::SLOT
        }
        Target::Constraint(n) if n.is_empty() => timeline::PHYSICS,
        Target::Constraint(n) => match skel.constraint(n).ok_or_else(|| CoreError::NotFound(format!("제약 '{n}'")))? {
            Constraint::Ik(_) => timeline::IK,
            Constraint::Transform(_) => timeline::TRANSFORM,
            Constraint::Path(_) => timeline::PATH,
            Constraint::Physics(_) => timeline::PHYSICS,
            Constraint::Slider(_) => timeline::SLIDER,
        },
        Target::Deform { skin, slot, attachment } | Target::Sequence { skin, slot, attachment } => {
            if skel.skin(skin).and_then(|s| s.get(slot, attachment)).is_none() {
                return Err(CoreError::NotFound(format!("스킨 '{skin}' 슬롯 '{slot}'의 첨부물 '{attachment}'")));
            }
            return Ok(());
        }
        Target::DrawOrder | Target::Event(_) => return Ok(()),
    };
    if !allowed.contains(&tl) {
        return Err(CoreError::InvalidArgs(format!("이 대상에 없는 타임라인: '{tl}'")));
    }
    Ok(())
}

/// 일반 키 목록. `create`면 없을 때 만든다. drawOrder·event는 None.
fn keys_mut<'a>(anim: &'a mut Animation, t: &Target, tl: &str, create: bool) -> Option<&'a mut Vec<Key>> {
    match t {
        Target::Bone(n) if create => Some(anim.bones.entry(n).entry(tl)),
        Target::Bone(n) => anim.bones.get_mut(n)?.get_mut(tl),
        Target::Slot(n) if create => Some(anim.slots.entry(n).entry(tl)),
        Target::Slot(n) => anim.slots.get_mut(n)?.get_mut(tl),
        Target::Constraint(n) if create => Some(anim.constraints.entry(n).entry(tl)),
        Target::Constraint(n) => anim.constraints.get_mut(n)?.get_mut(tl),
        Target::Deform { skin, slot, attachment } if create => Some(anim.deform.entry(skin).entry(slot).entry(attachment)),
        Target::Deform { skin, slot, attachment } => anim.deform.get_mut(skin)?.get_mut(slot)?.get_mut(attachment),
        Target::Sequence { skin, slot, attachment } if create => Some(anim.sequence.entry(skin).entry(slot).entry(attachment)),
        Target::Sequence { skin, slot, attachment } => anim.sequence.get_mut(skin)?.get_mut(slot)?.get_mut(attachment),
        Target::DrawOrder | Target::Event(_) => None,
    }
}

fn parse_curve(v: Option<&Value>) -> Result<Curve> {
    let Some(v) = v else { return Ok(Curve::Linear) };
    let c = Curve::from_json(v).map_err(|e| CoreError::InvalidArgs(e.to_string()))?;
    if let Curve::Bezier(b) = &c {
        if b.is_empty() || b.len() % 4 != 0 {
            return Err(CoreError::InvalidArgs("베지어 곡선은 값 채널마다 숫자 4개가 필요합니다".into()));
        }
    }
    Ok(c)
}

/// 대상 애니메이션 이름: 인자 → 현재 애니메이션. 없으면 오류.
fn anim_name(ctx: &CommandContext, args: &Args) -> Result<String> {
    let name = match args.opt_str("animation") {
        Some(a) => a.to_string(),
        None => ctx.project.editor.active_animation.clone().ok_or_else(|| CoreError::InvalidArgs("애니메이션을 먼저 고르세요".into()))?,
    };
    if ctx.project.skeleton.animation(&name).is_none() {
        return Err(CoreError::NotFound(format!("애니메이션 '{name}'")));
    }
    Ok(name)
}

fn insert_draw_order(keys: &mut Vec<DrawOrderKey>, key: DrawOrderKey) {
    match keys.iter_mut().find(|k| (k.time - key.time).abs() < TIME_EPS) {
        Some(k) => *k = key,
        None => {
            let i = keys.iter().position(|k| k.time > key.time).unwrap_or(keys.len());
            keys.insert(i, key);
        }
    }
}

fn insert_event(keys: &mut Vec<EventKey>, key: EventKey) {
    match keys.iter_mut().find(|k| (k.time - key.time).abs() < TIME_EPS && k.name == key.name) {
        Some(k) => *k = key,
        None => {
            let i = keys.iter().position(|k| k.time > key.time).unwrap_or(keys.len());
            keys.insert(i, key);
        }
    }
}

/// 키 하나를 `time`에 넣는다(`spec`은 key.set 인자 또는 KeyCopy 모양).
fn put_key(skel: &mut Skeleton, anim: &str, spec: &Args, time: f32) -> Result<()> {
    if !(time >= 0.0) {
        return Err(CoreError::InvalidArgs("키 시간은 0 이상이어야 합니다".into()));
    }
    let target = parse_target(spec.get("target"))?;
    let tl = spec.opt_str("timeline").unwrap_or("");
    match &target {
        Target::DrawOrder => {
            let offsets = match spec.get("offsets") {
                Some(v) => Vec::<DrawOrderOffset>::from_json(v).map_err(|e| CoreError::InvalidArgs(e.to_string()))?,
                None => vec![],
            };
            for o in &offsets {
                slot_exists(skel, &o.slot)?;
            }
            insert_draw_order(&mut keying::animation_mut(skel, anim)?.draw_order, DrawOrderKey { time, offsets });
        }
        Target::Event(name) => {
            let name = name.as_deref().or(spec.opt_str("s")).ok_or_else(|| CoreError::InvalidArgs("이벤트 이름이 필요합니다".into()))?;
            let def = skel.event(name).ok_or_else(|| CoreError::NotFound(format!("이벤트 '{name}'")))?.clone();
            // 정의의 기본값과 다른 값만 저장한다.
            let int = spec.opt_f32("int").map(|v| v.round() as i32).filter(|v| *v != def.int);
            let float = spec.opt_f32("float").filter(|v| *v != def.float);
            let string = spec.opt_str("string").map(String::from).filter(|v| *v != def.string);
            let volume = spec.opt_f32("volume").filter(|v| *v != def.volume);
            let balance = spec.opt_f32("balance").filter(|v| *v != def.balance);
            let key = EventKey { time, name: name.to_string(), int, float, string, volume, balance };
            insert_event(&mut keying::animation_mut(skel, anim)?.events, key);
        }
        t => {
            check_timeline(skel, t, tl)?;
            let mut key = Key {
                time,
                v: spec.f32_list("v").unwrap_or_default(),
                s: spec.opt_str("s").map(String::from),
                offset: spec.opt_usize("offset").unwrap_or(0) as u32,
                curve: parse_curve(spec.get("curve"))?,
            };
            let keys = keys_mut(keying::animation_mut(skel, anim)?, t, tl, true).expect("키 목록");
            if spec.get("curve").is_none() {
                if let Some(i) = keying::find_key(keys, time) {
                    key.curve = keys[i].curve.clone();
                }
            }
            keying::insert_key(keys, key);
        }
    }
    Ok(())
}

/// KeyRef 하나.
struct KeyRef {
    target: Target,
    timeline: String,
    time: f32,
}

fn parse_refs(args: &Args) -> Result<Vec<KeyRef>> {
    let list = args.get("keys").and_then(Value::as_array).ok_or_else(|| CoreError::InvalidArgs("'keys' 배열이 필요합니다".into()))?;
    list.iter()
        .map(|v| {
            let a = Args::new(v);
            Ok(KeyRef { target: parse_target(a.get("target"))?, timeline: a.opt_str("timeline").unwrap_or("").to_string(), time: a.f32("time")? })
        })
        .collect()
}

/// 꺼낸 키.
enum Taken {
    Key(Target, String, Key),
    Draw(DrawOrderKey),
    Event(EventKey),
}

fn not_found(r: &KeyRef) -> CoreError {
    CoreError::NotFound(format!("{} 시점의 키", r.time))
}

/// 가리키는 키를 애니메이션에서 꺼낸다.
fn take(anim: &mut Animation, r: &KeyRef) -> Result<Taken> {
    let near = |t: f32| (t - r.time).abs() < TIME_EPS;
    match &r.target {
        Target::DrawOrder => {
            let i = anim.draw_order.iter().position(|k| near(k.time)).ok_or_else(|| not_found(r))?;
            Ok(Taken::Draw(anim.draw_order.remove(i)))
        }
        Target::Event(name) => {
            let i = anim
                .events
                .iter()
                .position(|k| near(k.time) && name.as_deref().is_none_or(|n| n == k.name))
                .ok_or_else(|| not_found(r))?;
            Ok(Taken::Event(anim.events.remove(i)))
        }
        t => {
            let keys = keys_mut(anim, t, &r.timeline, false).ok_or_else(|| not_found(r))?;
            let i = keying::find_key(keys, r.time).ok_or_else(|| not_found(r))?;
            Ok(Taken::Key(t.clone(), r.timeline.clone(), keys.remove(i)))
        }
    }
}

fn put_back(anim: &mut Animation, taken: Taken) {
    match taken {
        Taken::Key(t, tl, k) => keying::insert_key(keys_mut(anim, &t, &tl, true).expect("키 목록"), k),
        Taken::Draw(k) => insert_draw_order(&mut anim.draw_order, k),
        Taken::Event(k) => insert_event(&mut anim.events, k),
    }
}

/// 인자: {"bones?": "string[]", "slots?": "string[]"}
/// 현재 시점에 본은 rotate·translate·scale·shear를, 슬롯은 rgba·attachment를 현재 값으로 키 지정한다.
fn key_edited(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, &Args::new(&Value::Null))?;
    let time = ctx.project.editor.time;
    key_current(&mut ctx.project.skeleton, &anim, time, &args.str_list("bones"), &args.str_list("slots"))?;
    Ok(Value::Null)
}

fn key_current(skel: &mut Skeleton, anim: &str, time: f32, bones: &[String], slots: &[String]) -> Result<()> {
    for b in bones {
        bone_exists(skel, b)?;
    }
    for s in slots {
        slot_exists(skel, s)?;
    }
    let slot_data: Vec<_> = slots.iter().map(|s| skel.slot(s).expect("슬롯").clone()).collect();
    let a = keying::animation_mut(skel, anim)?;
    for b in bones {
        let v = keying::current_bone_values(a, b, time);
        keying::key_bone_values(a, b, time, v);
    }
    for s in &slot_data {
        let c = keying::current_slot_color(a, s, time);
        keying::set_key_value(a.slots.entry(&s.name).entry("rgba"), time, c, None);
        let att = keying::current_slot_attachment(a, s, time);
        keying::key_slot_attachment(a, &s.name, time, att);
    }
    Ok(())
}

/// 인자: {"bones": "string[]"}
/// 제약까지 적용한 로컬 값(PoseOutput.locals)을 셋업 기준 상대값으로 바꿔 4개 타임라인에 키 지정한다.
/// 포즈 계산 결과가 비어 있으면 현재 애니메이션 값(키가 없으면 셋업 값)을 쓴다.
fn key_constrained(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, &Args::new(&Value::Null))?;
    let time = ctx.project.editor.time;
    let bones = args.str_list("bones");
    let skel = &mut ctx.project.skeleton;
    for b in &bones {
        bone_exists(skel, b)?;
    }
    let req = PoseRequest { animation: Some(anim.clone()), time, skin: ctx.project.editor.active_skin.clone(), ..Default::default() };
    let out = PoseEngine::new().compute(skel, &req);
    let values: Vec<_> = bones
        .iter()
        .map(|b| {
            let i = skel.bone_index(b).expect("본");
            match out.locals.get(i) {
                Some(l) if out.locals.len() == skel.bones.len() => keying::relative_values(&skel.bones[i], l),
                _ => keying::current_bone_values(skel.animation(&anim).expect("애니메이션"), b, time),
            }
        })
        .collect();
    let a = keying::animation_mut(skel, &anim)?;
    for (b, v) in bones.iter().zip(values) {
        keying::key_bone_values(a, b, time, v);
    }
    Ok(Value::Null)
}

/// 인자: {} — 모든 본에 key.edited.
fn key_all(ctx: &mut CommandContext, _args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, &Args::new(&Value::Null))?;
    let time = ctx.project.editor.time;
    let bones: Vec<String> = ctx.project.skeleton.bones.iter().map(|b| b.name.clone()).collect();
    key_current(&mut ctx.project.skeleton, &anim, time, &bones, &[])?;
    Ok(Value::Null)
}

/// 인자: {"animation?": "string", "target": "{kind, name?, skin?, slot?, attachment?}", "timeline": "string", "time?": "number", "v?": "number[]", "s?": "string|null", "offset?": "number", "curve?": "\"linear\"|\"stepped\"|number[]"}
/// drawOrder는 `offsets`([{slot, offset}]), event는 `int`·`float`·`string`·`volume`·`balance`를 더 받는다.
fn key_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, args)?;
    let time = args.opt_f32("time").unwrap_or(ctx.project.editor.time);
    put_key(&mut ctx.project.skeleton, &anim, args, time)?;
    Ok(Value::Null)
}

/// 인자: {"animation?": "string", "keys": "KeyRef[]"}
fn key_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, args)?;
    let refs = parse_refs(args)?;
    let a = keying::animation_mut(&mut ctx.project.skeleton, &anim)?;
    for r in &refs {
        take(a, r)?;
    }
    keying::prune(a);
    Ok(Value::Null)
}

/// 인자: {"animation?": "string", "keys": "KeyRef[]", "delta": "number(초)"}
/// 고른 키를 모두 꺼낸 뒤 새 시간에 다시 넣는다(같은 시점의 다른 키는 덮어쓴다). 0보다 앞으로는 못 간다.
fn key_move(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, args)?;
    let refs = parse_refs(args)?;
    let delta = args.f32("delta")?;
    if refs.iter().any(|r| r.time + delta < -TIME_EPS) {
        return Err(CoreError::Rejected("키를 0초보다 앞으로 옮길 수 없습니다".into()));
    }
    let a = keying::animation_mut(&mut ctx.project.skeleton, &anim)?;
    let mut taken = vec![];
    for r in &refs {
        taken.push(take(a, r)?);
    }
    for t in taken {
        let moved = |time: f32| (time + delta).max(0.0);
        let t = match t {
            Taken::Key(tg, tl, mut k) => {
                k.time = moved(k.time);
                Taken::Key(tg, tl, k)
            }
            Taken::Draw(mut k) => {
                k.time = moved(k.time);
                Taken::Draw(k)
            }
            Taken::Event(mut k) => {
                k.time = moved(k.time);
                Taken::Event(k)
            }
        };
        put_back(a, t);
    }
    keying::prune(a);
    Ok(Value::Null)
}

/// 인자: {"animation?": "string", "keys": "KeyRef[]", "curve": "\"linear\"|\"stepped\"|number[]"}
/// drawOrder·event 키는 곡선이 없으므로 건너뛴다.
fn key_set_curve(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, args)?;
    let refs = parse_refs(args)?;
    let curve = parse_curve(args.get("curve"))?;
    let a = keying::animation_mut(&mut ctx.project.skeleton, &anim)?;
    for r in &refs {
        if matches!(r.target, Target::DrawOrder | Target::Event(_)) {
            continue;
        }
        let keys = keys_mut(a, &r.target, &r.timeline, false).ok_or_else(|| not_found(r))?;
        let i = keying::find_key(keys, r.time).ok_or_else(|| not_found(r))?;
        keys[i].curve = curve.clone();
    }
    Ok(Value::Null)
}

/// 인자: {"animation?": "string", "keys": "KeyCopy[]", "time": "number"}
/// KeyCopy의 time은 복사한 키 가운데 가장 이른 키 기준 상대 시간이다.
fn key_paste(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let anim = anim_name(ctx, args)?;
    let base = args.f32("time")?;
    let list = args.get("keys").and_then(Value::as_array).ok_or_else(|| CoreError::InvalidArgs("'keys' 배열이 필요합니다".into()))?;
    for v in list {
        let spec = Args::new(v);
        let rel = spec.opt_f32("time").unwrap_or(0.0);
        put_key(&mut ctx.project.skeleton, &anim, &spec, base + rel)?;
    }
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::json::{parse, write, FromJson, ToJson, WriteStyle};
    use crate::model::{Curve, Project};
    use crate::{CoreError, Document};

    fn doc() -> Document {
        let mut d = Document::new();
        run(&mut d, "bone.create", r#"{"name": "a", "x": 10, "rotation": 20}"#);
        run(&mut d, "slot.create", r#"{"bone": "a", "name": "s"}"#);
        run(&mut d, "animation.create", r#"{"name": "anim"}"#);
        d
    }

    fn rotate_times(d: &Document) -> Vec<f32> {
        d.project().skeleton.animation("anim").unwrap().bones.get("a").map_or(vec![], |t| t.get("rotate").map_or(vec![], |k| k.iter().map(|k| k.time).collect()))
    }

    #[test]
    fn key_set_validates_and_keeps_order() {
        let mut d = doc();
        for t in [1.0, 0.0, 0.5] {
            run(&mut d, "key.set", &format!(r#"{{"target": {{"kind": "bone", "name": "a"}}, "timeline": "rotate", "time": {t}, "v": [{t}]}}"#));
        }
        assert_eq!(rotate_times(&d), vec![0.0, 0.5, 1.0]);
        assert!(matches!(exec(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "rgba", "v": [1]}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "없음"}, "timeline": "rotate"}"#), Err(CoreError::NotFound(_))));
        assert!(matches!(exec(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": -1}"#), Err(CoreError::InvalidArgs(_))));
        run(&mut d, "animation.setActive", r#"{"name": null}"#);
        assert!(matches!(exec(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate"}"#), Err(CoreError::InvalidArgs(_))));
        // animation 인자로 직접 고를 수 있다
        run(&mut d, "key.set", r#"{"animation": "anim", "target": {"kind": "drawOrder"}, "timeline": "", "time": 0.2, "offsets": [{"slot": "s", "offset": 0}]}"#);
        assert_eq!(d.project().skeleton.animations[0].draw_order.len(), 1);
    }

    #[test]
    fn f_g09_key_move_delete_undo() {
        let mut d = doc();
        for t in [0.0, 0.5, 1.0] {
            run(&mut d, "key.set", &format!(r#"{{"target": {{"kind": "bone", "name": "a"}}, "timeline": "rotate", "time": {t}, "v": [{t}]}}"#));
        }
        run(&mut d, "event.create", r#"{"name": "e"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "event", "name": "e"}, "timeline": "", "time": 0.5}"#);
        let before = d.project().skeleton.clone();
        // 0초 키를 0.75로: 시간순 재정렬
        run(&mut d, "key.move", r#"{"keys": [{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0}], "delta": 0.75}"#);
        assert_eq!(rotate_times(&d), vec![0.5, 0.75, 1.0]);
        let k = &d.project().skeleton.animations[0].bones.get("a").unwrap().get("rotate").unwrap()[1];
        assert_eq!(k.v, vec![0.0]);
        // 두 키를 함께 −0.5: 0.5 → 0, 0.75 → 0.25
        run(&mut d, "key.move", r#"{"keys": [{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0.5}, {"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0.75}, {"target": {"kind": "event"}, "time": 0.5}], "delta": -0.5}"#);
        assert_eq!(rotate_times(&d), vec![0.0, 0.25, 1.0]);
        assert_eq!(d.project().skeleton.animations[0].events[0].time, 0.0);
        // 0 미만 거부, 없는 키는 NotFound
        assert!(matches!(exec(&mut d, "key.move", r#"{"keys": [{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0.25}], "delta": -1}"#), Err(CoreError::Rejected(_))));
        assert!(matches!(exec(&mut d, "key.delete", r#"{"keys": [{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0.3}]}"#), Err(CoreError::NotFound(_))));
        run(&mut d, "key.delete", r#"{"keys": [{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0.25}, {"target": {"kind": "event", "name": "e"}, "time": 0}]}"#);
        assert_eq!(rotate_times(&d), vec![0.0, 1.0]);
        assert!(d.project().skeleton.animations[0].events.is_empty());
        // 마지막 키까지 지우면 빈 타임라인이 정리된다
        run(&mut d, "key.delete", r#"{"keys": [{"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0}, {"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 1}]}"#);
        assert!(d.project().skeleton.animations[0].bones.is_empty());
        for _ in 0..4 {
            run(&mut d, "edit.undo", "{}");
        }
        assert_eq!(d.project().skeleton, before);
    }

    #[test]
    fn f_g10_set_curve_roundtrip() {
        let mut d = doc();
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "translate", "time": 0, "v": [0, 0]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "translate", "time": 1, "v": [10, 5]}"#);
        let r = r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "translate", "time": 0}"#;
        run(&mut d, "key.setCurve", &format!(r#"{{"keys": [{r}], "curve": [0.25, 0, 0.75, 10, 0.25, 0, 0.75, 5]}}"#));
        let key = |d: &Document| d.project().skeleton.animations[0].bones.get("a").unwrap().get("translate").unwrap()[0].clone();
        assert_eq!(key(&d).curve, Curve::Bezier(vec![0.25, 0.0, 0.75, 10.0, 0.25, 0.0, 0.75, 5.0]));
        let text = write(&d.project().to_json(false), &WriteStyle::compact());
        assert_eq!(Project::from_json(&parse(&text).unwrap()).unwrap().skeleton.animations, d.project().skeleton.animations);
        // key.set으로 값만 바꾸면 곡선은 유지
        run(&mut d, "key.set", r#"{"target": {"kind": "bone", "name": "a"}, "timeline": "translate", "time": 0, "v": [1, 1]}"#);
        assert!(matches!(key(&d).curve, Curve::Bezier(_)));
        run(&mut d, "key.setCurve", &format!(r#"{{"keys": [{r}], "curve": "stepped"}}"#));
        assert_eq!(key(&d).curve, Curve::Stepped);
        run(&mut d, "key.setCurve", &format!(r#"{{"keys": [{r}], "curve": "linear"}}"#));
        assert_eq!(key(&d).curve, Curve::Linear);
        assert!(matches!(exec(&mut d, "key.setCurve", &format!(r#"{{"keys": [{r}], "curve": [1, 2, 3]}}"#)), Err(CoreError::InvalidArgs(_))));
        run(&mut d, "edit.undo", "{}");
        assert_eq!(key(&d).curve, Curve::Stepped);
    }

    #[test]
    fn f_g13_key_edited_all_and_constrained() {
        let mut d = doc();
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "img", "type": "region"}"#);
        run(&mut d, "time.set", r#"{"time": 0.5}"#);
        run(&mut d, "key.edited", r#"{"bones": ["a"], "slots": ["s"]}"#);
        let skel = &d.project().skeleton;
        let tl = skel.animations[0].bones.get("a").unwrap();
        // 키가 없으므로 셋업 = 상대값 0 / 1
        assert_eq!(tl.get("rotate").unwrap()[0].v, vec![0.0]);
        assert_eq!(tl.get("translate").unwrap()[0].v, vec![0.0, 0.0]);
        assert_eq!(tl.get("scale").unwrap()[0].v, vec![1.0, 1.0]);
        assert_eq!(tl.get("shear").unwrap()[0].v, vec![0.0, 0.0]);
        assert_eq!(tl.get("rotate").unwrap()[0].time, 0.5);
        let st = skel.animations[0].slots.get("s").unwrap();
        assert_eq!(st.get("rgba").unwrap()[0].v, vec![1.0, 1.0, 1.0, 1.0]);
        assert_eq!(st.get("attachment").unwrap()[0].s.as_deref(), Some("img"));
        // 애니메이트 모드에서 회전 후 K: 현재 키 값 유지
        run(&mut d, "mode.set", r#"{"mode": "animate"}"#);
        run(&mut d, "bone.setTransform", r#"{"name": "a", "rotation": 50}"#);
        run(&mut d, "key.edited", r#"{"bones": ["a"]}"#);
        assert_eq!(d.project().skeleton.animations[0].bones.get("a").unwrap().get("rotate").unwrap()[0].v, vec![30.0]);
        // 전부 키: root에도 4개 타임라인
        run(&mut d, "key.all", "{}");
        assert_eq!(d.project().skeleton.animations[0].bones.get("root").unwrap().len(), 4);
        // 제약 없는 뼈대의 제약 적용 값 = 현재 값: rotate 30 그대로, 키 4개
        run(&mut d, "time.set", r#"{"time": 1}"#);
        run(&mut d, "key.constrained", r#"{"bones": ["a"]}"#);
        let tl = d.project().skeleton.animations[0].bones.get("a").unwrap();
        let k = tl.get("rotate").unwrap().iter().find(|k| k.time == 1.0).unwrap();
        assert_eq!(k.v, vec![30.0]);
        assert!(tl.values().all(|keys| keys.iter().any(|k| k.time == 1.0)));
        run(&mut d, "animation.setActive", r#"{"name": null}"#);
        assert!(matches!(exec(&mut d, "key.edited", r#"{"bones": ["a"]}"#), Err(CoreError::InvalidArgs(_))));
    }

    #[test]
    fn key_paste_relative_times() {
        let mut d = doc();
        run(&mut d, "key.paste", r#"{"time": 2, "keys": [
            {"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0, "v": [1], "curve": "stepped"},
            {"target": {"kind": "bone", "name": "a"}, "timeline": "rotate", "time": 0.5, "v": [2]},
            {"target": {"kind": "slot", "name": "s"}, "timeline": "alpha", "time": 0.25, "v": [0.5]}
        ]}"#);
        assert_eq!(rotate_times(&d), vec![2.0, 2.5]);
        let a = &d.project().skeleton.animations[0];
        assert_eq!(a.bones.get("a").unwrap().get("rotate").unwrap()[0].curve, Curve::Stepped);
        assert_eq!(a.slots.get("s").unwrap().get("alpha").unwrap()[0].time, 2.25);
    }

    #[test]
    fn key_set_constraint_and_sequence_targets() {
        let mut d = doc();
        run(&mut d, "bone.create", r#"{"name": "t"}"#);
        run(&mut d, "constraint.create", r#"{"type": "physics", "bone": "a"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "constraint", "name": "physics"}, "timeline": "wind", "v": [3]}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "constraint", "name": ""}, "timeline": "gravity", "v": [1]}"#);
        assert!(matches!(exec(&mut d, "key.set", r#"{"target": {"kind": "constraint", "name": "physics"}, "timeline": "ik"}"#), Err(CoreError::InvalidArgs(_))));
        run(&mut d, "attachment.create", r#"{"slot": "s", "name": "r", "type": "region"}"#);
        run(&mut d, "key.set", r#"{"target": {"kind": "sequence", "slot": "s", "attachment": "r"}, "timeline": "", "s": "loop", "v": [0, 0.1]}"#);
        let a = &d.project().skeleton.animations[0];
        assert_eq!(a.constraints.get("").unwrap().get("gravity").unwrap()[0].v, vec![1.0]);
        assert_eq!(a.sequence.get("default").unwrap().get("s").unwrap().get("r").unwrap()[0].s.as_deref(), Some("loop"));
    }
}

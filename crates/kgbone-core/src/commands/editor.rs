//! 모드·시점·작업 상태·뼈대 속성 명령.

use super::{cmd, state_cmd, Args, CommandContext, FnCommand};
use crate::json::{FromJson, JValue as Value, ToJson};
use crate::model::{EditorMode, EditorState};
use crate::{CoreError, Result};

pub(super) const COMMANDS: &[FnCommand] = &[
    state_cmd("mode.toggle", "셋업/애니메이트 전환", mode_toggle),
    state_cmd("mode.set", "모드 지정", mode_set),
    state_cmd("time.set", "시점 이동", time_set),
    cmd("skeleton.set", "뼈대 속성", skeleton_set),
    cmd("project.setImagesPath", "이미지 폴더 경로", project_set_images_path),
    state_cmd("editor.set", "작업 상태", editor_set),
];

/// 인자: {} → {"mode"}
fn mode_toggle(ctx: &mut CommandContext, _args: &Args) -> Result<Value> {
    let e = &mut ctx.project.editor;
    e.mode = match e.mode {
        EditorMode::Setup => EditorMode::Animate,
        EditorMode::Animate => EditorMode::Setup,
    };
    Ok(Value::obj().with("mode", e.mode.to_json(true)))
}

/// 인자: {"mode": "setup|animate"}
fn mode_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let s = args.str("mode")?;
    ctx.project.editor.mode = EditorMode::parse(s).ok_or_else(|| CoreError::InvalidArgs(format!("모르는 모드: {s}")))?;
    Ok(Value::Null)
}

fn check_time(t: f32) -> Result<f32> {
    if t >= 0.0 && t.is_finite() {
        Ok(t)
    } else {
        Err(CoreError::InvalidArgs("시점은 0 이상이어야 합니다".into()))
    }
}

/// 인자: {"time": "number(초)"}
fn time_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    ctx.project.editor.time = check_time(args.f32("time")?)?;
    Ok(Value::Null)
}

/// 인자: {"fps?": "number", "referenceScale?": "number"}
fn skeleton_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let positive = |k: &str| -> Result<Option<f32>> {
        match args.opt_f32(k) {
            Some(v) if !(v > 0.0) || !v.is_finite() => Err(CoreError::InvalidArgs(format!("'{k}'는 0보다 커야 합니다"))),
            v => Ok(v),
        }
    };
    let fps = positive("fps")?;
    let rs = positive("referenceScale")?;
    let skel = &mut ctx.project.skeleton;
    if let Some(v) = fps {
        skel.fps = v;
    }
    if let Some(v) = rs {
        skel.reference_scale = v;
    }
    Ok(Value::Null)
}

/// 인자: {"path": "string"}
fn project_set_images_path(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    ctx.project.images_path = args.str("path")?.to_string();
    Ok(Value::Null)
}

/// 인자: {"patch": "EditorState 일부(view, treeExpanded, onion, playback, activeAnimation, activeSkin, time, mode)"}
/// 객체 값(view, onion, playback)은 주어진 키만 바꾼다. 되돌리기 대상 아님.
fn editor_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let Some(Value::Obj(patch)) = args.obj("patch") else {
        return Err(CoreError::InvalidArgs("'patch' 객체가 필요합니다".into()));
    };
    let mut j = ctx.project.editor.to_json(true);
    for (k, v) in patch {
        let nested = matches!(j.get(k), Some(Value::Obj(_)));
        match (nested, v) {
            (true, Value::Obj(sub)) => {
                let cur = j.get_mut(k).expect("객체");
                for (sk, sv) in sub {
                    cur.set(sk, sv.clone());
                }
            }
            _ => j.set(k, v.clone()),
        }
    }
    let e = EditorState::from_json(&j).map_err(|e| CoreError::InvalidArgs(e.to_string()))?;
    check_time(e.time)?;
    let skel = &ctx.project.skeleton;
    if let Some(a) = &e.active_animation {
        if skel.animation(a).is_none() {
            return Err(CoreError::NotFound(format!("애니메이션 '{a}'")));
        }
    }
    if let Some(s) = &e.active_skin {
        if skel.skin(s).is_none() {
            return Err(CoreError::NotFound(format!("스킨 '{s}'")));
        }
    }
    ctx.project.editor = e;
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::{exec, run};
    use crate::model::EditorMode;
    use crate::{CoreError, Document};

    #[test]
    fn mode_time_editor_state_not_undoable() {
        let mut d = Document::new();
        assert_eq!(run(&mut d, "mode.toggle", "{}").get("mode").unwrap().as_str(), Some("animate"));
        run(&mut d, "time.set", r#"{"time": 1.5}"#);
        assert!(matches!(exec(&mut d, "time.set", r#"{"time": -1}"#), Err(CoreError::InvalidArgs(_))));
        assert!(matches!(exec(&mut d, "mode.set", r#"{"mode": "x"}"#), Err(CoreError::InvalidArgs(_))));
        run(&mut d, "editor.set", r#"{"patch": {"view": {"zoom": 2}, "treeExpanded": ["bone:root"], "onion": {"enabled": true}, "playback": {"loop": false}}}"#);
        let e = &d.project().editor;
        assert_eq!((e.mode, e.time), (EditorMode::Animate, 1.5));
        assert_eq!((e.view.x, e.view.zoom), (0.0, 2.0));
        assert_eq!(e.tree_expanded, vec!["bone:root".to_string()]);
        assert!(e.onion.enabled && e.onion.before == 2);
        assert!(!e.playback.loop_ && e.playback.speed == 1.0);
        assert!(matches!(exec(&mut d, "editor.set", r#"{"patch": {"activeAnimation": "없음"}}"#), Err(CoreError::NotFound(_))));
        assert!(!d.can_undo());
        assert!(!d.is_dirty());
    }

    #[test]
    fn skeleton_set_and_images_path() {
        let mut d = Document::new();
        run(&mut d, "skeleton.set", r#"{"fps": 24, "referenceScale": 50}"#);
        assert_eq!((d.project().skeleton.fps, d.project().skeleton.reference_scale), (24.0, 50.0));
        assert!(matches!(exec(&mut d, "skeleton.set", r#"{"fps": 0}"#), Err(CoreError::InvalidArgs(_))));
        run(&mut d, "project.setImagesPath", r#"{"path": "./그림"}"#);
        assert_eq!(d.project().images_path, "./그림");
        run(&mut d, "edit.undo", "{}");
        assert_eq!(d.project().images_path, "./images");
    }
}

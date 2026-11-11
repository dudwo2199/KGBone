//! 모드·시점·작업 상태·뼈대 속성 명령.

use super::{cmd, state_cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    state_cmd("mode.toggle", "셋업/애니메이트 전환", mode_toggle),
    state_cmd("mode.set", "모드 지정", mode_set),
    state_cmd("time.set", "시점 이동", time_set),
    cmd("skeleton.set", "뼈대 속성", skeleton_set),
    cmd("project.setImagesPath", "이미지 폴더 경로", project_set_images_path),
    state_cmd("editor.set", "작업 상태", editor_set),
];

/// 인자: {}
fn mode_toggle(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mode.toggle"))
}

/// 인자: {"mode": "setup|animate"}
fn mode_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mode.set"))
}

/// 인자: {"time": "number(초)"}
fn time_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("time.set"))
}

/// 인자: {"fps?": "number", "referenceScale?": "number"}
fn skeleton_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("skeleton.set"))
}

/// 인자: {"path": "string"}
fn project_set_images_path(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("project.setImagesPath"))
}

/// 인자: {"patch": "EditorState 일부(view, treeExpanded, onion, playback)"}
fn editor_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("editor.set"))
}

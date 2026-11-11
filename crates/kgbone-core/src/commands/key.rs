//! 키프레임 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

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

/// 인자: {"bones?": "string[]", "slots?": "string[]"}
fn key_edited(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.edited"))
}

/// 인자: {"bones": "string[]"}
fn key_constrained(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.constrained"))
}

/// 인자: {}
fn key_all(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.all"))
}

/// 인자: {"animation?": "string", "target": "{kind: bone|slot|constraint|deform|sequence|drawOrder|event, name?, skin?, slot?, attachment?}", "timeline": "string", "time?": "number", "v?": "number[]", "s?": "string|null", "offset?": "number", "curve?": "\"linear\"|\"stepped\"|number[]"}
fn key_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.set"))
}

/// 인자: {"animation?": "string", "keys": "KeyRef[]"}
fn key_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.delete"))
}

/// 인자: {"animation?": "string", "keys": "KeyRef[]", "delta": "number(초)"}
fn key_move(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.move"))
}

/// 인자: {"animation?": "string", "keys": "KeyRef[]", "curve": "\"linear\"|\"stepped\"|number[]"}
fn key_set_curve(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.setCurve"))
}

/// 인자: {"animation?": "string", "keys": "KeyCopy[]", "time": "number"}
fn key_paste(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("key.paste"))
}

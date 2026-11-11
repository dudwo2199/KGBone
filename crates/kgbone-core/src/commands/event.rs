//! 이벤트 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("event.create", "이벤트", event_create),
    cmd("event.delete", "이벤트 삭제", event_delete),
    cmd("event.set", "이벤트 속성", event_set),
];

/// 인자: {"name?": "string", "props?": "object"}
fn event_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("event.create"))
}

/// 인자: {"name": "string"}
fn event_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("event.delete"))
}

/// 인자: {"name": "string", "props": "object"}
fn event_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("event.set"))
}

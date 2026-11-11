//! 애니메이션 명령.

use super::{cmd, state_cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("animation.create", "애니메이션", animation_create),
    cmd("animation.delete", "애니메이션 삭제", animation_delete),
    cmd("animation.duplicate", "애니메이션 복제", animation_duplicate),
    state_cmd("animation.setActive", "애니메이션 고르기", animation_set_active),
];

/// 인자: {"name?": "string"}
fn animation_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("animation.create"))
}

/// 인자: {"name": "string"}
fn animation_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("animation.delete"))
}

/// 인자: {"name": "string"}
fn animation_duplicate(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("animation.duplicate"))
}

/// 인자: {"name": "string|null"}
fn animation_set_active(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("animation.setActive"))
}

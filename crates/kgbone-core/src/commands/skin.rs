//! 스킨 명령.

use super::{cmd, state_cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("skin.create", "스킨", skin_create),
    cmd("skin.delete", "스킨 삭제", skin_delete),
    state_cmd("skin.setActive", "스킨 전환", skin_set_active),
    cmd("skin.addItem", "스킨에 본/제약 추가", skin_add_item),
    cmd("skin.removeItem", "스킨에서 본/제약 빼기", skin_remove_item),
];

/// 인자: {"name?": "string"}
fn skin_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("skin.create"))
}

/// 인자: {"name": "string"}
fn skin_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("skin.delete"))
}

/// 인자: {"name": "string|null"}
fn skin_set_active(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("skin.setActive"))
}

/// 인자: {"skin": "string", "kind": "bone|constraint", "name": "string"}
fn skin_add_item(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("skin.addItem"))
}

/// 인자: {"skin": "string", "kind": "bone|constraint", "name": "string"}
fn skin_remove_item(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("skin.removeItem"))
}

//! 제약 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("constraint.create", "제약", constraint_create),
    cmd("constraint.delete", "제약 삭제", constraint_delete),
    cmd("constraint.set", "제약 속성", constraint_set),
    cmd("constraint.reorder", "제약 순서 변경", constraint_reorder),
];

/// 인자: {"type": "ik|transform|path|physics|slider", "name?": "string", "bones?": "string[]", "target?": "string", "source?": "string", "bone?": "string", "animation?": "string"}
fn constraint_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("constraint.create"))
}

/// 인자: {"name": "string"}
fn constraint_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("constraint.delete"))
}

/// 인자: {"name": "string", "props": "object(바꿀 필드만)"}
fn constraint_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("constraint.set"))
}

/// 인자: {"name": "string", "index": "number"}
fn constraint_reorder(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("constraint.reorder"))
}

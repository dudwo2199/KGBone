//! 첨부물 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("attachment.create", "첨부물", attachment_create),
    cmd("attachment.delete", "첨부물 삭제", attachment_delete),
    cmd("attachment.set", "첨부물 속성", attachment_set),
];

/// 인자: {"slot": "string", "name": "string", "type": "region|mesh|linkedmesh|boundingbox|point|clipping|path", "skin?": "string", "props?": "object", "show?": "bool(기본 true)"}
fn attachment_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("attachment.create"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string"}
fn attachment_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("attachment.delete"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "props": "object(바꿀 필드만)"}
fn attachment_set(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("attachment.set"))
}

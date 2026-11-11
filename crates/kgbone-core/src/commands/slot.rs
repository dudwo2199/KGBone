//! 슬롯 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("slot.moveUp", "그리기 순서 위로", slot_move_up),
    cmd("slot.moveDown", "그리기 순서 아래로", slot_move_down),
    cmd("slot.create", "슬롯", slot_create),
    cmd("slot.delete", "슬롯 삭제", slot_delete),
    cmd("slot.setColor", "슬롯 색상", slot_set_color),
    cmd("slot.setDarkColor", "어두운 색", slot_set_dark_color),
    cmd("slot.setBlend", "블렌드 모드", slot_set_blend),
    cmd("slot.reorder", "그리기 순서 변경", slot_reorder),
    cmd("slot.setAttachment", "보이는 첨부물", slot_set_attachment),
    cmd("slot.setProps", "슬롯 속성", slot_set_props),
];

/// 인자: {"name": "string"}
fn slot_move_up(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.moveUp"))
}

/// 인자: {"name": "string"}
fn slot_move_down(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.moveDown"))
}

/// 인자: {"bone": "string", "name?": "string"}
fn slot_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.create"))
}

/// 인자: {"name": "string"}
fn slot_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.delete"))
}

/// 인자: {"name": "string", "color": "RRGGBBAA"}
fn slot_set_color(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.setColor"))
}

/// 인자: {"name": "string", "dark": "RRGGBB|null"}
fn slot_set_dark_color(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.setDarkColor"))
}

/// 인자: {"name": "string", "blend": "normal|additive|multiply|screen"}
fn slot_set_blend(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.setBlend"))
}

/// 인자: {"name": "string", "index": "number"}
fn slot_reorder(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.reorder"))
}

/// 인자: {"name": "string", "attachment": "string|null"}
fn slot_set_attachment(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.setAttachment"))
}

/// 인자: {"name": "string", "visible?": "bool", "bone?": "string"}
fn slot_set_props(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("slot.setProps"))
}

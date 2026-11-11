//! 이름 바꾸기 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("item.rename", "이름 바꾸기", item_rename),
];

/// 인자: {"kind": "bone|slot|constraint|skin|animation|event|attachment", "name": "string", "newName": "string", "skin?": "string", "slot?": "string"}
fn item_rename(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("item.rename"))
}

//! 본 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("bone.create", "본", bone_create),
    cmd("bone.delete", "본 삭제", bone_delete),
    cmd("bone.setParent", "부모 변경", bone_set_parent),
    cmd("bone.setTransform", "본 변환", bone_set_transform),
    cmd("bone.setInherit", "상속 설정", bone_set_inherit),
    cmd("bone.setColor", "본 색상", bone_set_color),
    cmd("bone.setLength", "본 길이", bone_set_length),
    cmd("bone.setProps", "본 속성", bone_set_props),
];

/// 인자: {"parent?": "string", "name?": "string", "x?": "number", "y?": "number", "rotation?": "number", "length?": "number"}
fn bone_create(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.create"))
}

/// 인자: {"name": "string"}
fn bone_delete(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.delete"))
}

/// 인자: {"name": "string", "parent": "string", "keepWorld?": "bool(기본 true)"}
fn bone_set_parent(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.setParent"))
}

/// 인자: {"name": "string", "x?": "number", "y?": "number", "rotation?": "number", "scaleX?": "number", "scaleY?": "number", "shearX?": "number", "shearY?": "number"}
fn bone_set_transform(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.setTransform"))
}

/// 인자: {"name": "string", "inherit": "normal|onlyTranslation|noRotationOrReflection|noScale|noScaleOrReflection"}
fn bone_set_inherit(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.setInherit"))
}

/// 인자: {"name": "string", "color": "RRGGBBAA"}
fn bone_set_color(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.setColor"))
}

/// 인자: {"name": "string", "length": "number"}
fn bone_set_length(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.setLength"))
}

/// 인자: {"name": "string", "skinRequired?": "bool", "icon?": "string|null", "visible?": "bool"}
fn bone_set_props(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("bone.setProps"))
}

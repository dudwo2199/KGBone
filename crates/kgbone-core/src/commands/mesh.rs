//! 메시 정점·삼각형·가중치 명령.

use super::{cmd, Args, CommandContext, FnCommand};
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub(super) const COMMANDS: &[FnCommand] = &[
    cmd("mesh.setVertices", "메시 정점 편집", mesh_set_vertices),
    cmd("mesh.setTriangles", "메시 삼각형 편집", mesh_set_triangles),
    cmd("mesh.generate", "메시 자동 생성", mesh_generate),
    cmd("mesh.bind", "가중치 바인딩", mesh_bind),
    cmd("mesh.unbind", "가중치 해제", mesh_unbind),
    cmd("weights.auto", "자동 가중치", weights_auto),
    cmd("weights.paint", "가중치 페인팅", weights_paint),
];

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "vertices": "number[] (가중치 없는 로컬 x,y)", "uvs?": "number[]", "hull?": "number"}
fn mesh_set_vertices(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mesh.setVertices"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "triangles?": "number[] (없으면 외곽선으로 자동 삼각분할)"}
fn mesh_set_triangles(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mesh.setTriangles"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "detail?": "number(0~1)"}
fn mesh_generate(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mesh.generate"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "bones": "string[]"}
fn mesh_bind(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mesh.bind"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string"}
fn mesh_unbind(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("mesh.unbind"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "bones?": "string[]"}
fn weights_auto(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("weights.auto"))
}

/// 인자: {"slot": "string", "name": "string", "skin?": "string", "bone": "string", "x": "number(월드)", "y": "number(월드)", "radius": "number", "strength": "number(0~1)", "mode?": "add|remove|replace"}
fn weights_paint(ctx: &mut CommandContext, args: &Args) -> Result<Value> {
    let _ = (ctx, args);
    Err(CoreError::NotImplemented("weights.paint"))
}

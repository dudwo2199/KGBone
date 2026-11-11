//! 명령: `interface/commands.json`의 scope=core 명령 하나가 [`Command`] 하나다.
//!
//! 명령은 [`CommandContext`]로 모델을 바꾸고, 되돌리기 기록은 [`crate::Document`]가 맡는다.
//! 기능별 파일(bone.rs, slot.rs …)이 각자 `COMMANDS` 목록을 내놓고, 여기서 한 목록으로 합친다.
mod animation;
mod args;
mod attachment;
mod bone;
mod constraint;
mod editor;
mod event;
mod images;
mod item;
mod key;
mod keying;
mod mesh;
mod skin;
mod slot;

pub use args::Args;
pub use images::{ImageInfo, ImageStore};

use crate::model::Project;
use crate::Result;
use crate::json::JValue as Value;

/// 명령이 쓸 수 있는 것.
pub struct CommandContext<'a> {
    pub project: &'a mut Project,
    pub images: &'a ImageStore,
}

/// 명령 하나.
pub trait Command: Sync {
    fn id(&self) -> &'static str;
    /// 되돌리기 목록에 보일 이름.
    fn label(&self) -> &'static str;
    /// 되돌리기 기록 대상인가(작업 상태만 바꾸는 명령은 false).
    fn undoable(&self) -> bool {
        true
    }
    fn run(&self, ctx: &mut CommandContext, args: &Value) -> Result<Value>;
}

/// 함수 하나로 된 명령.
pub struct FnCommand {
    pub id: &'static str,
    pub label: &'static str,
    pub undoable: bool,
    pub run: fn(&mut CommandContext, &Args) -> Result<Value>,
}

impl Command for FnCommand {
    fn id(&self) -> &'static str {
        self.id
    }
    fn label(&self) -> &'static str {
        self.label
    }
    fn undoable(&self) -> bool {
        self.undoable
    }
    fn run(&self, ctx: &mut CommandContext, args: &Value) -> Result<Value> {
        (self.run)(ctx, &Args::new(args))
    }
}

/// 짧게 명령 목록을 쓰기 위한 도우미.
pub(crate) const fn cmd(id: &'static str, label: &'static str, run: fn(&mut CommandContext, &Args) -> Result<Value>) -> FnCommand {
    FnCommand { id, label, undoable: true, run }
}

/// 되돌리기 기록 대상이 아닌 명령.
pub(crate) const fn state_cmd(id: &'static str, label: &'static str, run: fn(&mut CommandContext, &Args) -> Result<Value>) -> FnCommand {
    FnCommand { id, label, undoable: false, run }
}

/// 등록된 모든 명령.
pub fn all() -> impl Iterator<Item = &'static dyn Command> {
    [
        bone::COMMANDS,
        slot::COMMANDS,
        attachment::COMMANDS,
        mesh::COMMANDS,
        skin::COMMANDS,
        constraint::COMMANDS,
        event::COMMANDS,
        animation::COMMANDS,
        key::COMMANDS,
        item::COMMANDS,
        editor::COMMANDS,
    ]
    .into_iter()
    .flat_map(|list| list.iter().map(|c| c as &'static dyn Command))
}

pub fn find(id: &str) -> Option<&'static dyn Command> {
    all().find(|c| c.id() == id)
}

#[cfg(test)]
mod tests {
    /// interface/commands.json의 scope=core 명령은 전부 등록돼 있고, 등록된 명령은 전부 거기 있다.
    #[test]
    fn f_l02_registry_matches_interface() {
        let text = include_str!("../../../../interface/commands.json");
        let v = crate::json::parse(text).unwrap();
        let handled_by_document = ["edit.undo", "edit.redo"];
        let mut core_ids = vec![];
        for c in v.get("commands").unwrap().as_array().unwrap() {
            if c.get("scope").and_then(|s| s.as_str()) == Some("core") {
                core_ids.push(c.get("id").unwrap().as_str().unwrap().to_string());
            }
        }
        for id in &core_ids {
            if handled_by_document.contains(&id.as_str()) {
                continue;
            }
            assert!(super::find(id).is_some(), "등록 안 된 명령: {id}");
        }
        for c in super::all() {
            assert!(core_ids.iter().any(|i| i == c.id()), "commands.json에 없는 명령: {}", c.id());
        }
    }
}

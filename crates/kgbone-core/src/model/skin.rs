//! 스킨과 이벤트 정의.
use super::{Attachment, OMap};
use crate::json_object;

/// 슬롯 이름 → (첨부물 이름 → 첨부물).
pub type SkinAttachments = OMap<OMap<Attachment>>;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Skin {
    pub name: String,
    /// 이 스킨이 켜졌을 때만 계산되는 본.
    pub bones: Vec<String>,
    /// 이 스킨이 켜졌을 때만 계산되는 제약.
    pub constraints: Vec<String>,
    pub attachments: SkinAttachments,
}
json_object!(Skin {
    name: "name" = String::new(),
    bones: "bones" = Vec::new(),
    constraints: "constraints" = Vec::new(),
    attachments: "attachments" = OMap::new(),
});

impl Skin {
    pub const DEFAULT: &'static str = "default";

    pub fn new(name: impl Into<String>) -> Self {
        Skin { name: name.into(), ..Default::default() }
    }

    pub fn get(&self, slot: &str, name: &str) -> Option<&Attachment> {
        self.attachments.get(slot)?.get(name)
    }

    pub fn get_mut(&mut self, slot: &str, name: &str) -> Option<&mut Attachment> {
        self.attachments.get_mut(slot)?.get_mut(name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventDef {
    pub name: String,
    pub int: i32,
    pub float: f32,
    pub string: String,
    pub audio: String,
    pub volume: f32,
    pub balance: f32,
}
json_object!(EventDef {
    name: "name" = String::new(),
    int: "int" = 0,
    float: "float" = 0.0,
    string: "string" = String::new(),
    audio: "audio" = String::new(),
    volume: "volume" = 1.0,
    balance: "balance" = 0.0,
});

impl EventDef {
    pub fn new(name: impl Into<String>) -> Self {
        EventDef { name: name.into(), int: 0, float: 0.0, string: String::new(), audio: String::new(), volume: 1.0, balance: 0.0 }
    }
}

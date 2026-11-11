//! 슬롯 데이터.
use super::{Color, DarkColor};
use crate::{json_object, json_string_enum};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
    Multiply,
    Screen,
}

json_string_enum!(BlendMode { Normal = "normal", Additive = "additive", Multiply = "multiply", Screen = "screen" });

#[derive(Clone, Debug, PartialEq)]
pub struct Slot {
    pub name: String,
    pub bone: String,
    pub color: Color,
    pub dark: Option<DarkColor>,
    pub attachment: Option<String>,
    pub blend: BlendMode,
    pub visible: bool,
}

json_object!(Slot {
    name: "name" = String::new(),
    bone: "bone" = String::new(),
    color: "color" = Color::WHITE,
    dark: "dark" = None,
    attachment: "attachment" = None,
    blend: "blend" = BlendMode::Normal,
    visible: "visible" = true,
});

impl Slot {
    pub fn new(name: impl Into<String>, bone: impl Into<String>) -> Self {
        Slot {
            name: name.into(),
            bone: bone.into(),
            color: Color::WHITE,
            dark: None,
            attachment: None,
            blend: BlendMode::Normal,
            visible: true,
        }
    }
}

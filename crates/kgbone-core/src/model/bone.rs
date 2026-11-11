//! 본 데이터.
use super::Color;
use crate::{json_object, json_string_enum};

/// 부모 변환을 얼마나 물려받는지.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Inherit {
    #[default]
    Normal,
    OnlyTranslation,
    NoRotationOrReflection,
    NoScale,
    NoScaleOrReflection,
}

json_string_enum!(Inherit {
    Normal = "normal",
    OnlyTranslation = "onlyTranslation",
    NoRotationOrReflection = "noRotationOrReflection",
    NoScale = "noScale",
    NoScaleOrReflection = "noScaleOrReflection",
});

impl Inherit {
    pub const ALL: [Inherit; 5] = [
        Inherit::Normal,
        Inherit::OnlyTranslation,
        Inherit::NoRotationOrReflection,
        Inherit::NoScale,
        Inherit::NoScaleOrReflection,
    ];
}

/// 본의 로컬 변환 값 7개.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub shear_x: f32,
    pub shear_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Transform { x: 0.0, y: 0.0, rotation: 0.0, scale_x: 1.0, scale_y: 1.0, shear_x: 0.0, shear_y: 0.0 }
    }
}

json_object!(Transform {
    x: "x" = 0.0,
    y: "y" = 0.0,
    rotation: "rotation" = 0.0,
    scale_x: "scaleX" = 1.0,
    scale_y: "scaleY" = 1.0,
    shear_x: "shearX" = 0.0,
    shear_y: "shearY" = 0.0,
});

#[derive(Clone, Debug, PartialEq)]
pub struct Bone {
    pub name: String,
    pub parent: Option<String>,
    pub length: f32,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub shear_x: f32,
    pub shear_y: f32,
    pub inherit: Inherit,
    pub skin_required: bool,
    pub color: Color,
    pub icon: Option<String>,
    pub visible: bool,
}

json_object!(Bone {
    name: "name" = String::new(),
    parent: "parent" = None,
    length: "length" = 0.0,
    x: "x" = 0.0,
    y: "y" = 0.0,
    rotation: "rotation" = 0.0,
    scale_x: "scaleX" = 1.0,
    scale_y: "scaleY" = 1.0,
    shear_x: "shearX" = 0.0,
    shear_y: "shearY" = 0.0,
    inherit: "inherit" = Inherit::Normal,
    skin_required: "skinRequired" = false,
    color: "color" = Color::BONE,
    icon: "icon" = None,
    visible: "visible" = true,
});

impl Bone {
    pub fn new(name: impl Into<String>, parent: Option<String>) -> Self {
        Bone {
            name: name.into(),
            parent,
            length: 0.0,
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            shear_x: 0.0,
            shear_y: 0.0,
            inherit: Inherit::Normal,
            skin_required: false,
            color: Color::BONE,
            icon: None,
            visible: true,
        }
    }

    /// 로컬 변환 값을 한 묶음으로 꺼낸다.
    pub fn local(&self) -> Transform {
        Transform {
            x: self.x,
            y: self.y,
            rotation: self.rotation,
            scale_x: self.scale_x,
            scale_y: self.scale_y,
            shear_x: self.shear_x,
            shear_y: self.shear_y,
        }
    }

    pub fn set_local(&mut self, t: Transform) {
        self.x = t.x;
        self.y = t.y;
        self.rotation = t.rotation;
        self.scale_x = t.scale_x;
        self.scale_y = t.scale_y;
        self.shear_x = t.shear_x;
        self.shear_y = t.shear_y;
    }
}

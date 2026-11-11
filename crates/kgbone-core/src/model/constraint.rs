//! 제약 데이터. 모든 제약은 `Skeleton::constraints` 한 배열에 적용 순서대로 들어간다.
use super::attachment::tagged;
use crate::json::{FromJson, JValue, ToJson};
use crate::{json_object, json_string_enum, CoreError, Result};

/// 트랜스폼·슬라이더가 읽고 쓰는 본 속성.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BoneProperty {
    Rotate,
    #[default]
    X,
    Y,
    ScaleX,
    ScaleY,
    ShearY,
}
json_string_enum!(BoneProperty { Rotate = "rotate", X = "x", Y = "y", ScaleX = "scaleX", ScaleY = "scaleY", ShearY = "shearY" });

impl BoneProperty {
    pub const ALL: [BoneProperty; 6] =
        [BoneProperty::Rotate, BoneProperty::X, BoneProperty::Y, BoneProperty::ScaleX, BoneProperty::ScaleY, BoneProperty::ShearY];
}

#[derive(Clone, Debug, PartialEq)]
pub struct IkConstraint {
    pub name: String,
    pub skin_required: bool,
    pub bones: Vec<String>,
    pub target: String,
    pub mix: f32,
    pub softness: f32,
    pub bend_positive: bool,
    pub compress: bool,
    pub stretch: bool,
    pub uniform: bool,
    /// 4.3 부피 보존.
    pub volume: bool,
}
json_object!(IkConstraint {
    name: "name" = String::new(),
    skin_required: "skinRequired" = false,
    bones: "bones" = Vec::new(),
    target: "target" = String::new(),
    mix: "mix" = 1.0,
    softness: "softness" = 0.0,
    bend_positive: "bendPositive" = true,
    compress: "compress" = false,
    stretch: "stretch" = false,
    uniform: "uniform" = false,
    volume: "volume" = false,
});

/// 트랜스폼 매핑의 받는 쪽.
#[derive(Clone, Debug, PartialEq)]
pub struct ToProperty {
    pub property: BoneProperty,
    pub offset: f32,
    pub max: f32,
    pub scale: f32,
}
json_object!(ToProperty { property: "property" = BoneProperty::X, offset: "offset" = 0.0, max: "max" = 0.0, scale: "scale" = 1.0 });

/// 트랜스폼 매핑의 주는 쪽.
#[derive(Clone, Debug, PartialEq)]
pub struct FromProperty {
    pub from: BoneProperty,
    pub offset: f32,
    pub to: Vec<ToProperty>,
}
json_object!(FromProperty { from: "from" = BoneProperty::X, offset: "offset" = 0.0, to: "to" = Vec::new() });

#[derive(Clone, Debug, PartialEq)]
pub struct TransformConstraint {
    pub name: String,
    pub skin_required: bool,
    pub bones: Vec<String>,
    pub source: String,
    pub local_source: bool,
    pub local_target: bool,
    pub additive: bool,
    pub clamp: bool,
    pub mix_rotate: f32,
    pub mix_x: f32,
    pub mix_y: f32,
    pub mix_scale_x: f32,
    pub mix_scale_y: f32,
    pub mix_shear_y: f32,
    /// 비어 있으면 6개 속성의 같은-속성 기본 매핑.
    pub properties: Vec<FromProperty>,
}
json_object!(TransformConstraint {
    name: "name" = String::new(),
    skin_required: "skinRequired" = false,
    bones: "bones" = Vec::new(),
    source: "source" = String::new(),
    local_source: "localSource" = false,
    local_target: "localTarget" = false,
    additive: "additive" = false,
    clamp: "clamp" = false,
    mix_rotate: "mixRotate" = 1.0,
    mix_x: "mixX" = 1.0,
    mix_y: "mixY" = 1.0,
    mix_scale_x: "mixScaleX" = 1.0,
    mix_scale_y: "mixScaleY" = 1.0,
    mix_shear_y: "mixShearY" = 1.0,
    properties: "properties" = Vec::new(),
});

impl TransformConstraint {
    /// 실제로 쓰는 매핑 목록(비어 있으면 기본 매핑).
    pub fn effective_properties(&self) -> Vec<FromProperty> {
        if !self.properties.is_empty() {
            return self.properties.clone();
        }
        BoneProperty::ALL
            .into_iter()
            .map(|p| FromProperty { from: p, offset: 0.0, to: vec![ToProperty { property: p, offset: 0.0, max: 0.0, scale: 1.0 }] })
            .collect()
    }

    /// 받는 속성의 혼합 값.
    pub fn mix_of(&self, p: BoneProperty) -> f32 {
        match p {
            BoneProperty::Rotate => self.mix_rotate,
            BoneProperty::X => self.mix_x,
            BoneProperty::Y => self.mix_y,
            BoneProperty::ScaleX => self.mix_scale_x,
            BoneProperty::ScaleY => self.mix_scale_y,
            BoneProperty::ShearY => self.mix_shear_y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PositionMode {
    Fixed,
    #[default]
    Percent,
}
json_string_enum!(PositionMode { Fixed = "fixed", Percent = "percent" });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpacingMode {
    #[default]
    Length,
    Fixed,
    Percent,
    Proportional,
}
json_string_enum!(SpacingMode { Length = "length", Fixed = "fixed", Percent = "percent", Proportional = "proportional" });

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RotateMode {
    #[default]
    Tangent,
    Chain,
    ChainScale,
}
json_string_enum!(RotateMode { Tangent = "tangent", Chain = "chain", ChainScale = "chainScale" });

#[derive(Clone, Debug, PartialEq)]
pub struct PathConstraint {
    pub name: String,
    pub skin_required: bool,
    pub bones: Vec<String>,
    /// 패스 첨부물이 있는 슬롯.
    pub target: String,
    pub position_mode: PositionMode,
    pub spacing_mode: SpacingMode,
    pub rotate_mode: RotateMode,
    pub offset_rotation: f32,
    pub position: f32,
    pub spacing: f32,
    pub mix_rotate: f32,
    pub mix_x: f32,
    pub mix_y: f32,
}
json_object!(PathConstraint {
    name: "name" = String::new(),
    skin_required: "skinRequired" = false,
    bones: "bones" = Vec::new(),
    target: "target" = String::new(),
    position_mode: "positionMode" = PositionMode::Percent,
    spacing_mode: "spacingMode" = SpacingMode::Length,
    rotate_mode: "rotateMode" = RotateMode::Tangent,
    offset_rotation: "offsetRotation" = 0.0,
    position: "position" = 0.0,
    spacing: "spacing" = 0.0,
    mix_rotate: "mixRotate" = 1.0,
    mix_x: "mixX" = 1.0,
    mix_y: "mixY" = 1.0,
});

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsConstraint {
    pub name: String,
    pub skin_required: bool,
    pub bone: String,
    pub x: f32,
    pub y: f32,
    pub rotate: f32,
    pub scale_x: f32,
    pub shear_x: f32,
    pub limit: f32,
    pub fps: f32,
    pub inertia: f32,
    pub strength: f32,
    pub damping: f32,
    pub mass: f32,
    pub wind: f32,
    pub gravity: f32,
    pub mix: f32,
    /// 4.3 부피 보존.
    pub volume: bool,
}
json_object!(PhysicsConstraint {
    name: "name" = String::new(),
    skin_required: "skinRequired" = false,
    bone: "bone" = String::new(),
    x: "x" = 0.0,
    y: "y" = 0.0,
    rotate: "rotate" = 0.0,
    scale_x: "scaleX" = 0.0,
    shear_x: "shearX" = 0.0,
    limit: "limit" = 5000.0,
    fps: "fps" = 60.0,
    inertia: "inertia" = 1.0,
    strength: "strength" = 100.0,
    damping: "damping" = 1.0,
    mass: "mass" = 1.0,
    wind: "wind" = 0.0,
    gravity: "gravity" = 0.0,
    mix: "mix" = 1.0,
    volume: "volume" = false,
});

#[derive(Clone, Debug, PartialEq)]
pub struct SliderConstraint {
    pub name: String,
    pub skin_required: bool,
    pub animation: String,
    pub time: f32,
    pub loop_: bool,
    pub additive: bool,
    pub mix: f32,
    pub bone: Option<String>,
    pub property: BoneProperty,
    pub property_from: f32,
    pub property_to: f32,
    pub local: bool,
}
json_object!(SliderConstraint {
    name: "name" = String::new(),
    skin_required: "skinRequired" = false,
    animation: "animation" = String::new(),
    time: "time" = 0.0,
    loop_: "loop" = false,
    additive: "additive" = false,
    mix: "mix" = 1.0,
    bone: "bone" = None,
    property: "property" = BoneProperty::X,
    property_from: "propertyFrom" = 0.0,
    property_to: "propertyTo" = 1.0,
    local: "local" = false,
});

/// 제약 종류. `type` 값으로 구분.
#[derive(Clone, Debug, PartialEq)]
pub enum Constraint {
    Ik(IkConstraint),
    Transform(TransformConstraint),
    Path(PathConstraint),
    Physics(PhysicsConstraint),
    Slider(SliderConstraint),
}

impl ToJson for Constraint {
    fn to_json(&self, full: bool) -> JValue {
        let body = match self {
            Constraint::Ik(c) => c.to_json(full),
            Constraint::Transform(c) => c.to_json(full),
            Constraint::Path(c) => c.to_json(full),
            Constraint::Physics(c) => c.to_json(full),
            Constraint::Slider(c) => c.to_json(full),
        };
        tagged(self.type_name(), body)
    }
}

impl FromJson for Constraint {
    fn from_json(v: &JValue) -> Result<Self> {
        Ok(match v.get("type").and_then(JValue::as_str).unwrap_or("") {
            "ik" => Constraint::Ik(FromJson::from_json(v)?),
            "transform" => Constraint::Transform(FromJson::from_json(v)?),
            "path" => Constraint::Path(FromJson::from_json(v)?),
            "physics" => Constraint::Physics(FromJson::from_json(v)?),
            "slider" => Constraint::Slider(FromJson::from_json(v)?),
            other => return Err(CoreError::Parse(format!("모르는 제약 종류: '{other}'"))),
        })
    }
}

impl Constraint {
    pub const TYPES: [&'static str; 5] = ["ik", "transform", "path", "physics", "slider"];

    pub fn name(&self) -> &str {
        match self {
            Constraint::Ik(c) => &c.name,
            Constraint::Transform(c) => &c.name,
            Constraint::Path(c) => &c.name,
            Constraint::Physics(c) => &c.name,
            Constraint::Slider(c) => &c.name,
        }
    }
    pub fn name_mut(&mut self) -> &mut String {
        match self {
            Constraint::Ik(c) => &mut c.name,
            Constraint::Transform(c) => &mut c.name,
            Constraint::Path(c) => &mut c.name,
            Constraint::Physics(c) => &mut c.name,
            Constraint::Slider(c) => &mut c.name,
        }
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Constraint::Ik(_) => "ik",
            Constraint::Transform(_) => "transform",
            Constraint::Path(_) => "path",
            Constraint::Physics(_) => "physics",
            Constraint::Slider(_) => "slider",
        }
    }
    pub fn skin_required(&self) -> bool {
        match self {
            Constraint::Ik(c) => c.skin_required,
            Constraint::Transform(c) => c.skin_required,
            Constraint::Path(c) => c.skin_required,
            Constraint::Physics(c) => c.skin_required,
            Constraint::Slider(c) => c.skin_required,
        }
    }
    /// 이 제약이 값을 바꾸는 본 이름들.
    pub fn constrained_bones(&self) -> Vec<&str> {
        match self {
            Constraint::Ik(c) => c.bones.iter().map(String::as_str).collect(),
            Constraint::Transform(c) => c.bones.iter().map(String::as_str).collect(),
            Constraint::Path(c) => c.bones.iter().map(String::as_str).collect(),
            Constraint::Physics(c) => vec![c.bone.as_str()],
            Constraint::Slider(_) => vec![],
        }
    }
    /// 이 제약이 참조하는 모든 본 이름(제약 받는 본 + 목표·원본 본).
    pub fn referenced_bones(&self) -> Vec<&str> {
        let mut v = self.constrained_bones();
        match self {
            Constraint::Ik(c) => v.push(&c.target),
            Constraint::Transform(c) => v.push(&c.source),
            Constraint::Slider(c) => {
                if let Some(b) = &c.bone {
                    v.push(b)
                }
            }
            _ => {}
        }
        v
    }
}

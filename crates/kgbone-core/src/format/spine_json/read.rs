//! Spine JSON → Project (뼈대 머리, 본, 슬롯, 제약, 이벤트 정의). 스킨·애니메이션은 따로 나눈 파일에서 읽는다.
use super::read_anim::read_animation;
use super::read_skin::read_skins;
use super::value::{at, b, color, entries, f, i, items, s, strs};
use crate::format::ReadResult;
use crate::json::{self, JValue};
use crate::model::{
    BlendMode, Bone, BoneProperty, Color, Constraint, DarkColor, EventDef, FromProperty, IkConstraint, Inherit, PathConstraint,
    PhysicsConstraint, PositionMode, Project, RotateMode, Skeleton, Slot, SpacingMode, ToProperty, TransformConstraint,
};
use crate::{CoreError, Result, Warning};

/// 최상위에서 아는 키.
pub(super) const TOP_KEYS: [&str; 10] = ["skeleton", "bones", "slots", "ik", "transform", "path", "physics", "skins", "events", "animations"];
/// 제약 종류 배열 키(같은 order일 때의 순서).
pub(super) const CONSTRAINT_KINDS: [&str; 4] = ["ik", "transform", "path", "physics"];
/// 4.2 트랜스폼 오프셋 키 ↔ 속성.
pub(super) const TRANSFORM_OFFSETS: [(&str, BoneProperty); 6] = [
    ("rotation", BoneProperty::Rotate),
    ("x", BoneProperty::X),
    ("y", BoneProperty::Y),
    ("scaleX", BoneProperty::ScaleX),
    ("scaleY", BoneProperty::ScaleY),
    ("shearY", BoneProperty::ShearY),
];

pub(super) fn read(bytes: &[u8]) -> Result<ReadResult> {
    let text = std::str::from_utf8(bytes).map_err(|_| CoreError::Parse("UTF-8 텍스트가 아닙니다".into()))?;
    let raw = json::parse(text)?;
    if !matches!(raw, JValue::Obj(_)) {
        return Err(CoreError::Parse("Spine JSON 최상위가 객체가 아닙니다".into()));
    }
    let mut warnings = vec![];
    let head = raw.get("skeleton").cloned().unwrap_or_else(JValue::obj);
    check_version(&head, &mut warnings);
    for (k, _) in entries(Some(&raw)) {
        if !TOP_KEYS.contains(&k.as_str()) {
            warnings.push(Warning::new("spine.unknownKey", format!("모르는 항목 '{k}'은 읽지 않고 내보낼 때 그대로 보존합니다."), k.clone()));
        }
    }
    let mut sk = Skeleton {
        x: f(&head, "x", 0.0)?,
        y: f(&head, "y", 0.0)?,
        width: f(&head, "width", 0.0)?,
        height: f(&head, "height", 0.0)?,
        fps: f(&head, "fps", 30.0)?,
        reference_scale: f(&head, "referenceScale", 100.0)?,
        ..Skeleton::default()
    };
    for v in items(raw.get("bones")) {
        sk.bones.push(read_bone(v, &mut warnings).map_err(at(format!("본 {}", name_of(v))))?);
    }
    for v in items(raw.get("slots")) {
        sk.slots.push(read_slot(v).map_err(at(format!("슬롯 {}", name_of(v))))?);
    }
    sk.constraints = read_constraints(&raw)?;
    sk.skins = read_skins(raw.get("skins"), &mut warnings)?;
    for (name, v) in entries(raw.get("events")) {
        sk.events.push(read_event(name, v).map_err(at(format!("이벤트 {name}")))?);
    }
    for (name, v) in entries(raw.get("animations")) {
        sk.animations.push(read_animation(name, v, &mut warnings).map_err(at(format!("애니메이션 {name}")))?);
    }
    let images_path = match head.get("images").and_then(JValue::as_str) {
        Some(p) if !p.trim_end_matches('/').is_empty() => p.trim_end_matches('/').to_string(),
        _ => "./images".to_string(),
    };
    let project = Project { images_path, skeleton: sk, ..Project::new() };
    Ok(ReadResult { project, raw, warnings })
}

fn name_of(v: &JValue) -> &str {
    v.get("name").and_then(JValue::as_str).unwrap_or("?")
}

/// 버전 문자열을 보고 지원 범위 밖이면 경고한다.
fn check_version(head: &JValue, warnings: &mut Vec<Warning>) {
    let Some(ver) = head.get("spine").and_then(JValue::as_str) else { return };
    let mut it = ver.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    let (major, minor) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
    if major < 4 {
        warnings.push(Warning::new(
            "spine.oldVersion",
            format!("Spine {ver} 형식입니다. 4.x 구조로 읽으므로 일부 값이 다르게 읽힐 수 있습니다."),
            "skeleton/spine",
        ));
    } else if major > 4 || minor > 3 {
        warnings.push(Warning::new(
            "spine.newerVersion",
            format!("Spine {ver} 형식입니다. 4.3까지의 구조로 읽고 모르는 항목은 보존만 합니다."),
            "skeleton/spine",
        ));
    }
}

fn read_bone(v: &JValue, warnings: &mut Vec<Warning>) -> Result<Bone> {
    let name = s(v, "name")?.ok_or_else(|| CoreError::Parse("본 이름이 없습니다".into()))?;
    // 4.2는 `inherit`, 그 이전은 `transform`.
    let inherit_s = s(v, "inherit")?.or(s(v, "transform")?);
    let inherit = match inherit_s.as_deref() {
        None => Inherit::Normal,
        Some(t) => Inherit::parse(t).unwrap_or_else(|| {
            warnings.push(Warning::new("spine.unknownValue", format!("모르는 상속 종류 '{t}' → normal"), format!("bones/{name}")));
            Inherit::Normal
        }),
    };
    Ok(Bone {
        parent: s(v, "parent")?,
        length: f(v, "length", 0.0)?,
        x: f(v, "x", 0.0)?,
        y: f(v, "y", 0.0)?,
        rotation: f(v, "rotation", 0.0)?,
        scale_x: f(v, "scaleX", 1.0)?,
        scale_y: f(v, "scaleY", 1.0)?,
        shear_x: f(v, "shearX", 0.0)?,
        shear_y: f(v, "shearY", 0.0)?,
        inherit,
        skin_required: b(v, "skin", false)?,
        color: color(v, "color", Color::BONE)?,
        icon: s(v, "icon")?,
        visible: b(v, "visible", true)?,
        name,
    })
}

fn read_slot(v: &JValue) -> Result<Slot> {
    let blend = match s(v, "blend")? {
        None => BlendMode::Normal,
        Some(t) => BlendMode::parse(&t).ok_or_else(|| CoreError::Parse(format!("모르는 블렌드 '{t}'")))?,
    };
    Ok(Slot {
        name: s(v, "name")?.ok_or_else(|| CoreError::Parse("슬롯 이름이 없습니다".into()))?,
        bone: s(v, "bone")?.ok_or_else(|| CoreError::Parse("슬롯 본이 없습니다".into()))?,
        color: color(v, "color", Color::WHITE)?,
        dark: <Option<DarkColor> as crate::json::FromJson>::from_json(v.get("dark").unwrap_or(&JValue::Null)).map_err(at("'dark'"))?,
        attachment: s(v, "attachment")?,
        blend,
        visible: true,
    })
}

/// ik·transform·path·physics 배열을 `order`(같으면 종류 순서, 그 안에서 원래 순서)로 합친다.
fn read_constraints(raw: &JValue) -> Result<Vec<Constraint>> {
    let mut all: Vec<(f64, usize, usize, Constraint)> = vec![];
    for (kind_i, kind) in CONSTRAINT_KINDS.iter().enumerate() {
        for (idx, v) in items(raw.get(kind)).iter().enumerate() {
            let c = read_constraint(kind, v).map_err(at(format!("{kind} 제약 {}", name_of(v))))?;
            let order = v.get("order").and_then(JValue::as_f64).unwrap_or(0.0);
            all.push((order, kind_i, idx, c));
        }
    }
    all.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    Ok(all.into_iter().map(|x| x.3).collect())
}

/// 값 `k`가 없으면 구버전 키 `old`를, 그것도 없으면 `d`.
fn f_or(v: &JValue, k: &str, old: &str, d: f32) -> Result<f32> {
    match v.get(k) {
        Some(_) => f(v, k, d),
        None => f(v, old, d),
    }
}

fn read_constraint(kind: &str, v: &JValue) -> Result<Constraint> {
    let name = s(v, "name")?.unwrap_or_default();
    let skin_required = b(v, "skin", false)?;
    Ok(match kind {
        "ik" => Constraint::Ik(IkConstraint {
            name,
            skin_required,
            bones: strs(v, "bones")?,
            target: s(v, "target")?.unwrap_or_default(),
            mix: f(v, "mix", 1.0)?,
            softness: f(v, "softness", 0.0)?,
            bend_positive: b(v, "bendPositive", true)?,
            compress: b(v, "compress", false)?,
            stretch: b(v, "stretch", false)?,
            uniform: b(v, "uniform", false)?,
            volume: false,
        }),
        "transform" => {
            let mut offsets = [0.0f32; 6];
            for (n, (k, _)) in TRANSFORM_OFFSETS.iter().enumerate() {
                offsets[n] = f(v, k, 0.0)?;
            }
            let properties = if offsets.iter().all(|o| *o == 0.0) {
                vec![]
            } else {
                TRANSFORM_OFFSETS
                    .iter()
                    .zip(offsets)
                    .map(|((_, p), o)| FromProperty { from: *p, offset: 0.0, to: vec![ToProperty { property: *p, offset: o, max: 0.0, scale: 1.0 }] })
                    .collect()
            };
            let local = b(v, "local", false)?;
            let mix_x = f_or(v, "mixX", "translateMix", 1.0)?;
            let mix_scale_x = f_or(v, "mixScaleX", "scaleMix", 1.0)?;
            Constraint::Transform(TransformConstraint {
                name,
                skin_required,
                bones: strs(v, "bones")?,
                source: s(v, "target")?.unwrap_or_default(),
                local_source: local,
                local_target: local,
                additive: b(v, "relative", false)?,
                clamp: false,
                mix_rotate: f_or(v, "mixRotate", "rotateMix", 1.0)?,
                mix_x,
                mix_y: f_or(v, "mixY", "translateMix", mix_x)?,
                mix_scale_x,
                mix_scale_y: f_or(v, "mixScaleY", "scaleMix", mix_scale_x)?,
                mix_shear_y: f_or(v, "mixShearY", "shearMix", 1.0)?,
                properties,
            })
        }
        "path" => {
            let mix_x = f_or(v, "mixX", "translateMix", 1.0)?;
            Constraint::Path(PathConstraint {
                name,
                skin_required,
                bones: strs(v, "bones")?,
                target: s(v, "target")?.unwrap_or_default(),
                position_mode: enum_or(v, "positionMode", PositionMode::parse, PositionMode::Percent)?,
                spacing_mode: enum_or(v, "spacingMode", SpacingMode::parse, SpacingMode::Length)?,
                rotate_mode: enum_or(v, "rotateMode", RotateMode::parse, RotateMode::Tangent)?,
                offset_rotation: f(v, "rotation", 0.0)?,
                position: f(v, "position", 0.0)?,
                spacing: f(v, "spacing", 0.0)?,
                mix_rotate: f_or(v, "mixRotate", "rotateMix", 1.0)?,
                mix_x,
                mix_y: f_or(v, "mixY", "translateMix", mix_x)?,
            })
        }
        _ => Constraint::Physics(PhysicsConstraint {
            name,
            skin_required,
            bone: s(v, "bone")?.unwrap_or_default(),
            x: f(v, "x", 0.0)?,
            y: f(v, "y", 0.0)?,
            rotate: f(v, "rotate", 0.0)?,
            scale_x: f(v, "scaleX", 0.0)?,
            shear_x: f(v, "shearX", 0.0)?,
            limit: f(v, "limit", 5000.0)?,
            fps: f(v, "fps", 60.0)?,
            inertia: f(v, "inertia", 1.0)?,
            strength: f(v, "strength", 100.0)?,
            damping: f(v, "damping", 1.0)?,
            mass: f(v, "mass", 1.0)?,
            wind: f(v, "wind", 0.0)?,
            gravity: f(v, "gravity", 0.0)?,
            mix: f(v, "mix", 1.0)?,
            volume: false,
        }),
    })
}

fn enum_or<T>(v: &JValue, k: &str, parse: fn(&str) -> Option<T>, d: T) -> Result<T> {
    match s(v, k)? {
        None => Ok(d),
        Some(t) => parse(&t).ok_or_else(|| CoreError::Parse(format!("'{k}': 모르는 값 '{t}'"))),
    }
}

fn read_event(name: &str, v: &JValue) -> Result<EventDef> {
    Ok(EventDef {
        name: name.to_string(),
        int: i(v, "int")?.unwrap_or(0),
        float: f(v, "float", 0.0)?,
        string: s(v, "string")?.unwrap_or_default(),
        audio: s(v, "audio")?.unwrap_or_default(),
        volume: f(v, "volume", 1.0)?,
        balance: f(v, "balance", 0.0)?,
    })
}

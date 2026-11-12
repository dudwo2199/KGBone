//! Project → Spine JSON.
//!
//! 모델을 Spine 모양 트리로 "전체(기본값 포함)"와 "희소(기본값 생략)" 두 번 만든다. 가져온 원본이 있으면
//! 원본에 합쳐(모르는 필드·키 순서·숫자 표기 보존) 원본 텍스트 모양으로 쓰고, 없으면 희소 트리를 쓴다.
use super::read::{CONSTRAINT_KINDS, TRANSFORM_OFFSETS};
use super::schema::SpineSchema;
use super::value::{arr_s, O};
use super::write_anim::animations;
use super::write_skin::skins;
use super::{SpineExportOptions, SpineVersion};
use crate::format::preserve;
use crate::format::{Original, WriteResult};
use crate::json::{self, JValue, WriteStyle};
use crate::model::{BoneProperty, Color, Constraint, Project, TransformConstraint};
use crate::{Result, Warning};

/// 만들 때 함께 들고 다니는 정보.
pub(super) struct Ctx<'a> {
    pub(super) project: &'a Project,
    /// 가져온 원본 트리(있으면).
    pub(super) raw: Option<&'a JValue>,
    pub(super) opts: &'a SpineExportOptions,
}

pub(super) fn write(project: &Project, original: Option<&Original>, opts: &SpineExportOptions) -> Result<WriteResult> {
    let ctx = Ctx { project, raw: original.map(|o| &o.raw), opts };
    let mut warnings = vec![];
    let full = build(&ctx, true, &mut warnings);
    let sparse = build(&ctx, false, &mut vec![]);
    let text = match original {
        Some(o) => {
            let merged = preserve::merge(&o.raw, &full, &sparse, &SpineSchema);
            match std::str::from_utf8(&o.bytes) {
                Ok(t) if !t.trim().is_empty() => preserve::splice(t, &merged),
                _ => json::write(&merged, &style(opts)),
            }
        }
        None => json::write(&sparse, &style(opts)),
    };
    Ok(WriteResult { bytes: text.into_bytes(), warnings })
}

fn style(opts: &SpineExportOptions) -> WriteStyle {
    if opts.pretty {
        WriteStyle::default()
    } else {
        WriteStyle::compact()
    }
}

fn build(ctx: &Ctx, full: bool, warnings: &mut Vec<Warning>) -> JValue {
    let sk = &ctx.project.skeleton;
    let mut root = O::new(full);
    root.put("skeleton", head(ctx, full));
    root.put("bones", JValue::Arr(sk.bones.iter().map(|b| bone(b, full)).collect()));
    root.list("slots", JValue::Arr(sk.slots.iter().map(|s| slot(s, full)).collect()));
    let orders = constraint_orders(ctx);
    let mut kinds: [Vec<JValue>; 4] = Default::default();
    for (n, c) in sk.constraints.iter().enumerate() {
        let order = orders[n].clone();
        let path = format!("constraints/{}", c.name());
        let (kind, v) = match c {
            Constraint::Slider(_) => {
                warnings.push(Warning::new(
                    "spine43.slider",
                    format!("슬라이더 제약 '{}'은 Spine JSON 표기가 확인되지 않아 내보내지 않았습니다(GAPS G-11).", c.name()),
                    path,
                ));
                continue;
            }
            Constraint::Ik(k) => {
                if k.volume {
                    warnings.push(volume_warning(c.name(), &path));
                }
                let mut o = O::new(full);
                o.put("name", JValue::str(&k.name)).put("order", order).b("skin", k.skin_required, false);
                o.put("bones", arr_s(&k.bones)).put("target", JValue::str(&k.target));
                o.f("mix", k.mix, 1.0).f("softness", k.softness, 0.0).b("bendPositive", k.bend_positive, true);
                o.b("compress", k.compress, false).b("stretch", k.stretch, false).b("uniform", k.uniform, false);
                (0, o.done())
            }
            Constraint::Transform(t) => (1, transform(t, order, full, warnings, &path)),
            Constraint::Path(p) => {
                let mut o = O::new(full);
                o.put("name", JValue::str(&p.name)).put("order", order).b("skin", p.skin_required, false);
                o.put("bones", arr_s(&p.bones)).put("target", JValue::str(&p.target));
                o.s("positionMode", p.position_mode.as_str(), "percent");
                o.s("spacingMode", p.spacing_mode.as_str(), "length");
                o.s("rotateMode", p.rotate_mode.as_str(), "tangent");
                o.f("rotation", p.offset_rotation, 0.0).f("position", p.position, 0.0).f("spacing", p.spacing, 0.0);
                o.f("mixRotate", p.mix_rotate, 1.0).f("mixX", p.mix_x, 1.0).f("mixY", p.mix_y, p.mix_x);
                (2, o.done())
            }
            Constraint::Physics(p) => {
                if p.volume {
                    warnings.push(volume_warning(c.name(), &path));
                }
                let mut o = O::new(full);
                o.put("name", JValue::str(&p.name)).put("order", order).b("skin", p.skin_required, false);
                o.put("bone", JValue::str(&p.bone));
                o.f("x", p.x, 0.0).f("y", p.y, 0.0).f("rotate", p.rotate, 0.0).f("scaleX", p.scale_x, 0.0).f("shearX", p.shear_x, 0.0);
                o.f("limit", p.limit, 5000.0).f("fps", p.fps, 60.0).f("inertia", p.inertia, 1.0).f("strength", p.strength, 100.0);
                o.f("damping", p.damping, 1.0).f("mass", p.mass, 1.0).f("wind", p.wind, 0.0).f("gravity", p.gravity, 0.0);
                o.f("mix", p.mix, 1.0);
                (3, o.done())
            }
        };
        kinds[kind].push(v);
    }
    for (kind, list) in CONSTRAINT_KINDS.iter().zip(kinds) {
        root.list(kind, JValue::Arr(list));
    }
    root.list("skins", skins(ctx, full));
    let mut events = O::new(full);
    for e in &sk.events {
        let mut o = O::new(full);
        o.f("int", e.int as f32, 0.0).f("float", e.float, 0.0).s("string", &e.string, "").s("audio", &e.audio, "");
        o.f("volume", e.volume, 1.0).f("balance", e.balance, 0.0);
        events.put(&e.name, o.done());
    }
    root.list("events", events.done());
    root.list("animations", animations(ctx, full, warnings));
    root.done()
}

fn volume_warning(name: &str, path: &str) -> Warning {
    Warning::new(
        "spine43.volume",
        format!("'{name}'의 부피 보존(volume)은 Spine JSON 표기가 확인되지 않아 빼고 내보냈습니다(GAPS G-12)."),
        path,
    )
}

/// 버전 문자열(가져온 원본이 같은 버전 계열이면 원본 값을 그대로 둔다).
fn head(ctx: &Ctx, full: bool) -> JValue {
    let sk = &ctx.project.skeleton;
    let raw_head = ctx.raw.and_then(|r| r.get("skeleton"));
    let mut o = O::new(full);
    let (prefix, ver) = match ctx.opts.version {
        SpineVersion::V43 => ("4.3", "4.3.00"),
        SpineVersion::V42 => ("4.2", "4.2.00"),
    };
    let raw_ver = raw_head.and_then(|h| h.get("spine")).and_then(JValue::as_str);
    if !raw_ver.is_some_and(|v| v == prefix || v.starts_with(&format!("{prefix}."))) {
        o.put("spine", JValue::str(ver));
    }
    o.put("x", JValue::from(sk.x)).put("y", JValue::from(sk.y));
    o.put("width", JValue::from(sk.width)).put("height", JValue::from(sk.height));
    o.f("fps", sk.fps, 30.0).f("referenceScale", sk.reference_scale, 100.0);
    // 이미지 경로: 가져온 원본에 있으면 원본 값을 둔다.
    if raw_head.and_then(|h| h.get("images")).is_none() {
        o.put("images", JValue::str(ctx.opts.images_path.as_str()));
    }
    o.done()
}

fn bone(b: &crate::model::Bone, full: bool) -> JValue {
    let mut o = O::new(full);
    o.put("name", JValue::str(&b.name)).opt("parent", b.parent.as_ref());
    o.f("length", b.length, 0.0).f("rotation", b.rotation, 0.0).f("x", b.x, 0.0).f("y", b.y, 0.0);
    o.f("scaleX", b.scale_x, 1.0).f("scaleY", b.scale_y, 1.0).f("shearX", b.shear_x, 0.0).f("shearY", b.shear_y, 0.0);
    o.s("inherit", b.inherit.as_str(), "normal").b("skin", b.skin_required, false);
    o.color("color", b.color, Color::BONE).opt("icon", b.icon.as_ref()).b("visible", b.visible, true);
    o.done()
}

fn slot(s: &crate::model::Slot, full: bool) -> JValue {
    let mut o = O::new(full);
    o.put("name", JValue::str(&s.name)).put("bone", JValue::str(&s.bone));
    o.color("color", s.color, Color::WHITE).opt("dark", s.dark.as_ref()).opt("attachment", s.attachment.as_ref());
    o.s("blend", s.blend.as_str(), "normal");
    o.done()
}

/// 4.2 표기 트랜스폼. 같은-속성·배율 1 매핑의 오프셋만 표현할 수 있다(GAPS G-10).
fn transform(t: &TransformConstraint, order: JValue, full: bool, warnings: &mut Vec<Warning>, path: &str) -> JValue {
    let mut offsets = [0.0f32; 6];
    let mut exact = !t.clamp && t.local_source == t.local_target;
    if !t.properties.is_empty() {
        for p in BoneProperty::ALL {
            let maps: Vec<_> = t.properties.iter().filter(|fp| fp.from == p).collect();
            match maps.as_slice() {
                [fp] if fp.to.len() == 1 && fp.to[0].property == p && fp.to[0].scale == 1.0 => {
                    offsets[idx(p)] = fp.to[0].offset - fp.offset;
                }
                // 매핑이 없는 속성은 4.2에서는 같은-속성으로 따라가므로, 혼합이 0일 때만 같다.
                [] if t.mix_of(p) == 0.0 => {}
                _ => exact = false,
            }
        }
        if t.properties.iter().any(|fp| fp.to.iter().any(|to| to.property != fp.from || to.scale != 1.0)) {
            exact = false;
        }
    }
    if !exact {
        warnings.push(Warning::new(
            "spine43.transformMapping",
            format!("트랜스폼 제약 '{}'의 속성 매핑·범위 제한·로컬/월드 분리는 4.2 표기로 표현할 수 없어 같은-속성 오프셋만 내보냈습니다(GAPS G-10).", t.name),
            path,
        ));
    }
    let mut o = O::new(full);
    o.put("name", JValue::str(&t.name)).put("order", order).b("skin", t.skin_required, false);
    o.put("bones", arr_s(&t.bones)).put("target", JValue::str(&t.source));
    for ((k, _), off) in TRANSFORM_OFFSETS.iter().zip(offsets) {
        o.f(k, off, 0.0);
    }
    o.f("mixRotate", t.mix_rotate, 1.0).f("mixX", t.mix_x, 1.0).f("mixY", t.mix_y, t.mix_x);
    o.f("mixScaleX", t.mix_scale_x, 1.0).f("mixScaleY", t.mix_scale_y, t.mix_scale_x).f("mixShearY", t.mix_shear_y, 1.0);
    o.b("local", t.local_source, false).b("relative", t.additive, false);
    o.done()
}

fn idx(p: BoneProperty) -> usize {
    BoneProperty::ALL.iter().position(|x| *x == p).unwrap_or(0)
}

fn kind_index(c: &Constraint) -> usize {
    CONSTRAINT_KINDS.iter().position(|k| *k == c.type_name()).unwrap_or(4)
}

/// 제약마다 `order` 값. 원본의 order 값으로 다시 읽어도 지금 순서가 되면 원본 값을, 아니면 배열 위치를 쓴다.
fn constraint_orders(ctx: &Ctx) -> Vec<JValue> {
    let cs = &ctx.project.skeleton.constraints;
    let by_index = || cs.iter().enumerate().map(|(n, _)| JValue::from(n)).collect::<Vec<_>>();
    let Some(raw) = ctx.raw else { return by_index() };
    let mut from_raw = vec![];
    for c in cs {
        if matches!(c, Constraint::Slider(_)) {
            from_raw.push(JValue::from(0usize));
            continue;
        }
        let found = raw
            .get(c.type_name())
            .and_then(JValue::as_array)
            .and_then(|a| a.iter().find(|x| x.get("name").and_then(JValue::as_str) == Some(c.name())));
        match found {
            Some(x) => from_raw.push(x.get("order").cloned().unwrap_or(JValue::from(0usize))),
            None => return by_index(),
        }
    }
    // 다시 읽을 때의 정렬(order, 종류 순서, 종류 안 위치)을 흉내 낸다.
    let mut within = [0usize; 5];
    let mut keyed: Vec<(f64, usize, usize, usize)> = vec![];
    for (n, c) in cs.iter().enumerate() {
        if matches!(c, Constraint::Slider(_)) {
            continue;
        }
        let k = kind_index(c);
        keyed.push((from_raw[n].as_f64().unwrap_or(0.0), k, within[k], n));
        within[k] += 1;
    }
    let mut sorted = keyed.clone();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    if sorted.iter().map(|x| x.3).eq(keyed.iter().map(|x| x.3)) {
        from_raw
    } else {
        by_index()
    }
}

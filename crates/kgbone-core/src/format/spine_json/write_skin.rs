//! 스킨·첨부물 → Spine JSON(4.x 배열 형식).
use super::read::CONSTRAINT_KINDS;
use super::read_skin::raw_attachment;
use super::value::{arr_f, arr_s, arr_u, O};
use super::write::Ctx;
use crate::json::{JValue, ToJson};
use crate::model::{Attachment, Color, Constraint, Sequence, Skin};

pub(super) fn skins(ctx: &Ctx, full: bool) -> JValue {
    JValue::Arr(ctx.project.skeleton.skins.iter().map(|s| skin(ctx, s, full)).collect())
}

fn skin(ctx: &Ctx, s: &Skin, full: bool) -> JValue {
    let sk = &ctx.project.skeleton;
    let mut o = O::new(full);
    o.put("name", JValue::str(&s.name));
    o.list("bones", arr_s(&s.bones));
    for kind in CONSTRAINT_KINDS {
        let names: Vec<String> = s
            .constraints
            .iter()
            .filter(|n| sk.constraint(n).is_some_and(|c| c.type_name() == kind && !matches!(c, Constraint::Slider(_))))
            .cloned()
            .collect();
        o.list(kind, arr_s(&names));
    }
    let mut slots = O::new(full);
    for (slot, atts) in s.attachments.iter() {
        let mut m = O::new(full);
        for (key, a) in atts.iter() {
            let raw = ctx.raw.and_then(|r| raw_attachment(r, &s.name, slot, key));
            m.put(key, attachment(a, raw, full));
        }
        slots.put(slot, m.done());
    }
    o.list("attachments", slots.done());
    o.done()
}

/// 이미지 경로 키: 원본이 `path` 없이 `name`만 썼으면 `name`, 아니면 `path`.
fn path_key(raw: Option<&JValue>) -> &'static str {
    match raw {
        Some(r) if r.get("path").is_none() && r.get("name").is_some() => "name",
        _ => "path",
    }
}

fn seq(o: &mut O, s: Option<&Sequence>) {
    o.opt("sequence", s);
}

fn attachment(a: &Attachment, raw: Option<&JValue>, full: bool) -> JValue {
    let mut o = O::new(full);
    o.s("type", a.type_name(), "region");
    match a {
        Attachment::Region(r) => {
            o.opt(path_key(raw), r.path.as_ref());
            o.f("x", r.x, 0.0).f("y", r.y, 0.0).f("scaleX", r.scale_x, 1.0).f("scaleY", r.scale_y, 1.0);
            o.f("rotation", r.rotation, 0.0).f("width", r.width, 32.0).f("height", r.height, 32.0);
            o.color("color", r.color, Color::WHITE);
            seq(&mut o, r.sequence.as_ref());
        }
        Attachment::Mesh(m) => {
            o.opt(path_key(raw), m.path.as_ref());
            o.put("uvs", arr_f(&m.uvs)).put("triangles", arr_u(&m.triangles)).put("vertices", arr_f(&m.vertices));
            o.f("hull", m.hull as f32, 0.0).list("edges", arr_u(&m.edges));
            o.f("width", m.width, 32.0).f("height", m.height, 32.0).color("color", m.color, Color::WHITE);
            seq(&mut o, m.sequence.as_ref());
        }
        Attachment::LinkedMesh(m) => {
            o.opt(path_key(raw), m.path.as_ref());
            o.put("parent", JValue::str(&m.parent)).opt("skin", m.skin.as_ref()).b("timelines", m.timelines, true);
            o.f("width", m.width, 32.0).f("height", m.height, 32.0).color("color", m.color, Color::WHITE);
            seq(&mut o, m.sequence.as_ref());
        }
        Attachment::BoundingBox(b) => {
            o.put("vertexCount", b.vertex_count.to_json(full)).put("vertices", arr_f(&b.vertices));
            o.color("color", b.color, Color::BOUNDING_BOX);
        }
        Attachment::Clipping(c) => {
            o.opt("end", c.end.as_ref()).put("vertexCount", c.vertex_count.to_json(full)).put("vertices", arr_f(&c.vertices));
            // 반전·볼록(4.3)은 표기 미확인: 켜졌을 때만 쓴다.
            o.b("inverse", c.inverse, false).b("convex", c.convex, false);
            o.color("color", c.color, Color::CLIPPING);
        }
        Attachment::Path(p) => {
            o.b("closed", p.closed, false).b("constantSpeed", p.constant_speed, true);
            o.put("lengths", arr_f(&p.lengths)).put("vertexCount", p.vertex_count.to_json(full)).put("vertices", arr_f(&p.vertices));
            o.color("color", p.color, Color::PATH);
        }
        Attachment::Point(p) => {
            o.f("x", p.x, 0.0).f("y", p.y, 0.0).f("rotation", p.rotation, 0.0).color("color", p.color, Color::POINT);
        }
    }
    o.done()
}

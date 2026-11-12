//! Spine JSON 스킨·첨부물 → 모델. 4.x 배열 형식과 3.x 객체 형식(`{"default": {...}}`)을 모두 읽는다.
use super::read::CONSTRAINT_KINDS;
use super::value::{at, b, color, entries, f, floats, ints, s, strs, u};
use crate::json::{FromJson, JValue};
use crate::model::{
    Attachment, BoundingBoxAttachment, ClippingAttachment, Color, LinkedMeshAttachment, MeshAttachment, OMap, PathAttachment,
    PointAttachment, RegionAttachment, Sequence, Skin,
};
use crate::{Result, Warning};

pub(super) fn read_skins(v: Option<&JValue>, warnings: &mut Vec<Warning>) -> Result<Vec<Skin>> {
    let mut out = vec![];
    match v {
        Some(JValue::Arr(list)) => {
            for sv in list {
                let name = s(sv, "name")?.unwrap_or_default();
                let mut skin = Skin::new(name.clone());
                skin.bones = strs(sv, "bones")?;
                for kind in CONSTRAINT_KINDS {
                    skin.constraints.extend(strs(sv, kind)?);
                }
                skin.attachments = read_slots(&name, sv.get("attachments"), warnings).map_err(at(format!("스킨 {name}")))?;
                out.push(skin);
            }
        }
        Some(JValue::Obj(m)) => {
            for (name, sv) in m {
                let mut skin = Skin::new(name.clone());
                skin.attachments = read_slots(name, Some(sv), warnings).map_err(at(format!("스킨 {name}")))?;
                out.push(skin);
            }
        }
        _ => {}
    }
    Ok(out)
}

fn read_slots(skin: &str, v: Option<&JValue>, warnings: &mut Vec<Warning>) -> Result<OMap<OMap<Attachment>>> {
    let mut slots = OMap::new();
    for (slot, atts) in entries(v) {
        let mut m = OMap::new();
        for (key, av) in entries(Some(atts)) {
            match read_attachment(av).map_err(at(format!("첨부물 {slot}/{key}")))? {
                Some(a) => {
                    m.insert(key.clone(), a);
                }
                None => warnings.push(Warning::new(
                    "spine.unknownAttachment",
                    format!("모르는 첨부물 종류라 읽지 않았습니다: {slot}/{key}"),
                    format!("skins/{skin}/{slot}/{key}"),
                )),
            }
        }
        slots.insert(slot.clone(), m);
    }
    Ok(slots)
}

/// 이미지 경로: `path`, 없으면 `name`(이미지 이름이 키와 다를 때).
fn image_path(v: &JValue) -> Result<Option<String>> {
    Ok(s(v, "path")?.or(s(v, "name")?))
}

fn sequence(v: &JValue) -> Result<Option<Sequence>> {
    match v.get("sequence") {
        None | Some(JValue::Null) => Ok(None),
        Some(x) => Sequence::from_json(x).map(Some).map_err(at("'sequence'")),
    }
}

/// 정점 목록과 가중치 여부(길이가 정점 수 × 2보다 길면 가중치).
fn verts(v: &JValue, count: usize) -> Result<(bool, Vec<f32>)> {
    let vs = floats(v, "vertices")?;
    Ok((vs.len() > count * 2, vs))
}

fn read_attachment(v: &JValue) -> Result<Option<Attachment>> {
    let t = s(v, "type")?.unwrap_or_else(|| "region".into());
    Ok(Some(match t.as_str() {
        "region" => Attachment::Region(RegionAttachment {
            path: image_path(v)?,
            x: f(v, "x", 0.0)?,
            y: f(v, "y", 0.0)?,
            rotation: f(v, "rotation", 0.0)?,
            scale_x: f(v, "scaleX", 1.0)?,
            scale_y: f(v, "scaleY", 1.0)?,
            width: f(v, "width", 32.0)?,
            height: f(v, "height", 32.0)?,
            color: color(v, "color", Color::WHITE)?,
            sequence: sequence(v)?,
        }),
        "mesh" => {
            let uvs = floats(v, "uvs")?;
            let (weighted, vertices) = verts(v, uvs.len() / 2)?;
            Attachment::Mesh(MeshAttachment {
                path: image_path(v)?,
                width: f(v, "width", 32.0)?,
                height: f(v, "height", 32.0)?,
                color: color(v, "color", Color::WHITE)?,
                triangles: ints(v, "triangles")?,
                weighted,
                vertices,
                uvs,
                hull: u(v, "hull", 0)?,
                edges: ints(v, "edges")?,
                sequence: sequence(v)?,
            })
        }
        "linkedmesh" => Attachment::LinkedMesh(LinkedMeshAttachment {
            path: image_path(v)?,
            width: f(v, "width", 32.0)?,
            height: f(v, "height", 32.0)?,
            color: color(v, "color", Color::WHITE)?,
            parent: s(v, "parent")?.unwrap_or_default(),
            skin: s(v, "skin")?,
            // 구버전 키는 `deform`.
            timelines: if v.get("timelines").is_some() { b(v, "timelines", true)? } else { b(v, "deform", true)? },
            sequence: sequence(v)?,
        }),
        "boundingbox" => {
            let vertex_count = u(v, "vertexCount", 0)?;
            let (weighted, vertices) = verts(v, vertex_count as usize)?;
            Attachment::BoundingBox(BoundingBoxAttachment { vertex_count, weighted, vertices, color: color(v, "color", Color::BOUNDING_BOX)? })
        }
        "clipping" => {
            let vertex_count = u(v, "vertexCount", 0)?;
            let (weighted, vertices) = verts(v, vertex_count as usize)?;
            Attachment::Clipping(ClippingAttachment {
                end: s(v, "end")?,
                vertex_count,
                weighted,
                vertices,
                inverse: b(v, "inverse", false)?,
                convex: b(v, "convex", false)?,
                color: color(v, "color", Color::CLIPPING)?,
            })
        }
        "path" => {
            let vertex_count = u(v, "vertexCount", 0)?;
            let (weighted, vertices) = verts(v, vertex_count as usize)?;
            Attachment::Path(PathAttachment {
                closed: b(v, "closed", false)?,
                constant_speed: b(v, "constantSpeed", true)?,
                vertex_count,
                weighted,
                vertices,
                lengths: floats(v, "lengths")?,
                color: color(v, "color", Color::PATH)?,
            })
        }
        "point" => Attachment::Point(PointAttachment {
            x: f(v, "x", 0.0)?,
            y: f(v, "y", 0.0)?,
            rotation: f(v, "rotation", 0.0)?,
            color: color(v, "color", Color::POINT)?,
        }),
        _ => return Ok(None),
    }))
}

/// 내보내기에서 원본 첨부물 객체를 찾는다(4.x 배열·3.x 객체 모두).
pub(super) fn raw_attachment<'a>(raw: &'a JValue, skin: &str, slot: &str, key: &str) -> Option<&'a JValue> {
    let skins = raw.get("skins")?;
    let atts = match skins {
        JValue::Arr(list) => list.iter().find(|s| s.get("name").and_then(JValue::as_str) == Some(skin))?.get("attachments")?,
        JValue::Obj(_) => skins.get(skin)?,
        _ => return None,
    };
    atts.get(slot)?.get(key)
}

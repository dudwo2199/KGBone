//! Spine JSON 애니메이션 → 모델. 본 타임라인 값은 두 형식 모두 셋업 기준 상대값이라 그대로 옮긴다(FORMAT.md 3.7).
use super::value::{at, b, entries, f, floats, i, items, key, of, s};
use crate::json::JValue;
use crate::model::{timeline, Animation, Color, DrawOrderKey, DrawOrderOffset, EventKey, Key};
use crate::{CoreError, Result, Warning};

pub(super) fn read_animation(name: &str, v: &JValue, warnings: &mut Vec<Warning>) -> Result<Animation> {
    let mut a = Animation::new(name);
    for (bone, tls) in entries(v.get("bones")) {
        let t = a.bones.entry(bone);
        for (tl, keys) in entries(Some(tls)) {
            if !timeline::BONE.contains(&tl.as_str()) {
                warnings.push(unknown_timeline(name, &format!("bones/{bone}/{tl}")));
                continue;
            }
            let ks = read_keys(keys, |k| bone_key(tl, k)).map_err(at(format!("본 {bone} {tl}")))?;
            t.insert(tl.clone(), ks);
        }
    }
    for (slot, tls) in entries(v.get("slots")) {
        let t = a.slots.entry(slot);
        for (tl, keys) in entries(Some(tls)) {
            if !timeline::SLOT.contains(&tl.as_str()) {
                warnings.push(unknown_timeline(name, &format!("slots/{slot}/{tl}")));
                continue;
            }
            let ks = read_keys(keys, |k| slot_key(tl, k)).map_err(at(format!("슬롯 {slot} {tl}")))?;
            t.insert(tl.clone(), ks);
        }
    }
    for (c, keys) in entries(v.get("ik")) {
        let ks = read_keys(keys, ik_key).map_err(at(format!("IK {c}")))?;
        a.constraints.entry(c).insert("ik", ks);
    }
    for (c, keys) in entries(v.get("transform")) {
        let ks = read_keys(keys, transform_key).map_err(at(format!("트랜스폼 {c}")))?;
        a.constraints.entry(c).insert("transform", ks);
    }
    for (c, tls) in entries(v.get("path")) {
        let t = a.constraints.entry(c);
        for (tl, keys) in entries(Some(tls)) {
            let ks = match tl.as_str() {
                "position" | "spacing" => read_keys(keys, |k| key(k, vec![f(k, "value", 0.0)?])),
                "mix" => read_keys(keys, |k| {
                    let mx = f(k, "mixX", 1.0)?;
                    key(k, vec![f(k, "mixRotate", 1.0)?, mx, f(k, "mixY", mx)?])
                }),
                _ => {
                    warnings.push(unknown_timeline(name, &format!("path/{c}/{tl}")));
                    continue;
                }
            }
            .map_err(at(format!("패스 {c} {tl}")))?;
            t.insert(tl.clone(), ks);
        }
    }
    for (c, tls) in entries(v.get("physics")) {
        let t = a.constraints.entry(c);
        for (tl, keys) in entries(Some(tls)) {
            if !timeline::PHYSICS.contains(&tl.as_str()) {
                warnings.push(unknown_timeline(name, &format!("physics/{c}/{tl}")));
                continue;
            }
            let ks = read_keys(keys, |k| match tl.as_str() {
                "reset" => key(k, vec![]),
                "mix" => key(k, vec![f(k, "value", 1.0)?]),
                _ => key(k, vec![f(k, "value", 0.0)?]),
            })
            .map_err(at(format!("물리 {c} {tl}")))?;
            t.insert(tl.clone(), ks);
        }
    }
    // 4.x: attachments → 스킨 → 슬롯 → 첨부물 → {deform, sequence}
    for (skin, slots) in entries(v.get("attachments")) {
        for (slot, atts) in entries(Some(slots)) {
            for (att, tls) in entries(Some(atts)) {
                for (tl, keys) in entries(Some(tls)) {
                    let place = format!("첨부물 {skin}/{slot}/{att} {tl}");
                    match tl.as_str() {
                        "deform" => {
                            let ks = read_keys(keys, deform_key).map_err(at(place))?;
                            a.deform.entry(skin).entry(slot).insert(att.clone(), ks);
                        }
                        "sequence" => {
                            let ks = read_keys(keys, sequence_key).map_err(at(place))?;
                            a.sequence.entry(skin).entry(slot).insert(att.clone(), ks);
                        }
                        _ => warnings.push(unknown_timeline(name, &format!("attachments/{skin}/{slot}/{att}/{tl}"))),
                    }
                }
            }
        }
    }
    // 구버전: deform → 스킨 → 슬롯 → 첨부물 → 키 목록
    for (skin, slots) in entries(v.get("deform")) {
        for (slot, atts) in entries(Some(slots)) {
            for (att, keys) in entries(Some(atts)) {
                let ks = read_keys(keys, deform_key).map_err(at(format!("변형 {skin}/{slot}/{att}")))?;
                a.deform.entry(skin).entry(slot).insert(att.clone(), ks);
            }
        }
    }
    let draw = v.get("drawOrder").or(v.get("draworder"));
    for k in items(draw) {
        let mut offsets = vec![];
        for o in items(k.get("offsets")) {
            offsets.push(DrawOrderOffset { slot: s(o, "slot")?.unwrap_or_default(), offset: i(o, "offset")?.unwrap_or(0) });
        }
        a.draw_order.push(DrawOrderKey { time: f(k, "time", 0.0)?, offsets });
    }
    for k in items(v.get("events")) {
        a.events.push(EventKey {
            time: f(k, "time", 0.0)?,
            name: s(k, "name")?.unwrap_or_default(),
            int: i(k, "int")?,
            float: of(k, "float")?,
            string: s(k, "string")?,
            volume: of(k, "volume")?,
            balance: of(k, "balance")?,
        });
    }
    Ok(a)
}

fn unknown_timeline(anim: &str, path: &str) -> Warning {
    Warning::new("spine.unknownTimeline", format!("모르는 타임라인이라 읽지 않았습니다(내보낼 때 보존): {path}"), format!("animations/{anim}/{path}"))
}

fn read_keys(v: &JValue, mut one: impl FnMut(&JValue) -> Result<Key>) -> Result<Vec<Key>> {
    let list = v.as_array().ok_or_else(|| CoreError::Parse("키 배열이 필요합니다".into()))?;
    list.iter().enumerate().map(|(n, k)| one(k).map_err(at(format!("{}번째 키", n + 1)))).collect()
}

fn bone_key(tl: &str, k: &JValue) -> Result<Key> {
    match tl {
        "rotate" | "translatex" | "translatey" | "shearx" | "sheary" => key(k, vec![f(k, "value", 0.0)?]),
        "scalex" | "scaley" => key(k, vec![f(k, "value", 1.0)?]),
        "translate" | "shear" => key(k, vec![f(k, "x", 0.0)?, f(k, "y", 0.0)?]),
        "scale" => key(k, vec![f(k, "x", 1.0)?, f(k, "y", 1.0)?]),
        _ => {
            // inherit
            let mut key = key(k, vec![])?;
            key.s = Some(s(k, "inherit")?.unwrap_or_else(|| "normal".into()));
            Ok(key)
        }
    }
}

fn hex(k: &JValue, name: &str, d: &str) -> Result<Color> {
    let t = s(k, name)?.unwrap_or_else(|| d.to_string());
    Color::parse_hex(&t).ok_or_else(|| CoreError::Parse(format!("'{name}': 색 형식이 잘못됨: {t}")))
}

fn slot_key(tl: &str, k: &JValue) -> Result<Key> {
    let c = |c: Color| c.to_f32();
    match tl {
        "attachment" => {
            let mut key = key(k, vec![])?;
            key.s = s(k, "name")?;
            Ok(key)
        }
        "rgba" => key(k, c(hex(k, "color", "FFFFFFFF")?).to_vec()),
        "rgb" => key(k, c(hex(k, "color", "FFFFFF")?)[..3].to_vec()),
        "alpha" => key(k, vec![f(k, "value", 1.0)?]),
        "rgba2" => {
            let mut v = c(hex(k, "light", "FFFFFFFF")?).to_vec();
            v.extend_from_slice(&c(hex(k, "dark", "000000")?)[..3]);
            key(k, v)
        }
        _ => {
            // rgb2
            let mut v = c(hex(k, "light", "FFFFFF")?)[..3].to_vec();
            v.extend_from_slice(&c(hex(k, "dark", "000000")?)[..3]);
            key(k, v)
        }
    }
}

fn ik_key(k: &JValue) -> Result<Key> {
    let sign = if b(k, "bendPositive", true)? { 1.0 } else { -1.0 };
    let flag = |x: bool| if x { 1.0 } else { 0.0 };
    key(k, vec![f(k, "mix", 1.0)?, f(k, "softness", 0.0)?, sign, flag(b(k, "compress", false)?), flag(b(k, "stretch", false)?)])
}

fn transform_key(k: &JValue) -> Result<Key> {
    let mx = f(k, "mixX", 1.0)?;
    let msx = f(k, "mixScaleX", 1.0)?;
    key(k, vec![f(k, "mixRotate", 1.0)?, mx, f(k, "mixY", mx)?, msx, f(k, "mixScaleY", msx)?, f(k, "mixShearY", 1.0)?])
}

fn deform_key(k: &JValue) -> Result<Key> {
    let mut key = key(k, floats(k, "vertices")?)?;
    key.offset = super::value::u(k, "offset", 0)?;
    Ok(key)
}

fn sequence_key(k: &JValue) -> Result<Key> {
    let mut key = key(k, vec![f(k, "index", 0.0)?, f(k, "delay", 0.0)?])?;
    key.s = Some(s(k, "mode")?.unwrap_or_else(|| "hold".into()));
    Ok(key)
}


//! 애니메이션 → Spine JSON. 본 타임라인 값은 그대로(두 형식 모두 셋업 기준 상대값).
use super::value::{arr_f, O};
use super::write::Ctx;
use crate::json::JValue;
use crate::model::{timeline, Animation, Color, Constraint, Key, Timelines};
use crate::Warning;

pub(super) fn animations(ctx: &Ctx, full: bool, warnings: &mut Vec<Warning>) -> JValue {
    let mut o = O::new(full);
    for a in &ctx.project.skeleton.animations {
        o.put(&a.name, animation(ctx, a, full, warnings));
    }
    o.done()
}

fn at(k: &Key, i: usize, d: f32) -> f32 {
    k.v.get(i).copied().unwrap_or(d)
}

/// time + 값 + curve.
fn keys(list: &[Key], full: bool, mut body: impl FnMut(&mut O, &Key)) -> JValue {
    JValue::Arr(
        list.iter()
            .map(|k| {
                let mut o = O::new(full);
                o.f("time", k.time, 0.0);
                body(&mut o, k);
                o.curve(&k.curve);
                o.done()
            })
            .collect(),
    )
}

fn hex4(k: &Key, i: usize) -> String {
    Color::from_f32([at(k, i, 1.0), at(k, i + 1, 1.0), at(k, i + 2, 1.0), at(k, i + 3, 1.0)]).to_hex_rgba()
}
fn hex3(k: &Key, i: usize, d: f32) -> String {
    Color::from_f32([at(k, i, d), at(k, i + 1, d), at(k, i + 2, d), 1.0]).to_hex_rgb()
}

fn bone_timelines(t: &Timelines, full: bool) -> JValue {
    let mut o = O::new(full);
    for (name, list) in t.iter() {
        if !timeline::BONE.contains(&name.as_str()) {
            continue;
        }
        let v = match name.as_str() {
            "rotate" | "translatex" | "translatey" | "shearx" | "sheary" => keys(list, full, |o, k| {
                o.f("value", at(k, 0, 0.0), 0.0);
            }),
            "scalex" | "scaley" => keys(list, full, |o, k| {
                o.f("value", at(k, 0, 1.0), 1.0);
            }),
            "translate" | "shear" => keys(list, full, |o, k| {
                o.f("x", at(k, 0, 0.0), 0.0).f("y", at(k, 1, 0.0), 0.0);
            }),
            "scale" => keys(list, full, |o, k| {
                o.f("x", at(k, 0, 1.0), 1.0).f("y", at(k, 1, 1.0), 1.0);
            }),
            _ => keys(list, full, |o, k| {
                o.put("inherit", JValue::str(k.s.as_deref().unwrap_or("normal")));
            }),
        };
        o.put(name, v);
    }
    o.done()
}

fn slot_timelines(t: &Timelines, full: bool) -> JValue {
    let mut o = O::new(full);
    for (name, list) in t.iter() {
        let v = match name.as_str() {
            "attachment" => keys(list, full, |o, k| {
                o.put("name", k.s.as_deref().map_or(JValue::Null, JValue::str));
            }),
            "rgba" => keys(list, full, |o, k| {
                o.put("color", JValue::str(hex4(k, 0)));
            }),
            "rgb" => keys(list, full, |o, k| {
                o.put("color", JValue::str(hex3(k, 0, 1.0)));
            }),
            "alpha" => keys(list, full, |o, k| {
                o.f("value", at(k, 0, 1.0), 1.0);
            }),
            "rgba2" => keys(list, full, |o, k| {
                o.put("light", JValue::str(hex4(k, 0))).put("dark", JValue::str(hex3(k, 4, 0.0)));
            }),
            "rgb2" => keys(list, full, |o, k| {
                o.put("light", JValue::str(hex3(k, 0, 1.0))).put("dark", JValue::str(hex3(k, 3, 0.0)));
            }),
            _ => continue,
        };
        o.put(name, v);
    }
    o.done()
}

/// 제약 이름으로 종류를 정한다. 모르는 이름은 타임라인 이름으로 짐작한다.
fn kind_of<'a>(ctx: &'a Ctx, name: &str, t: &Timelines) -> &'static str {
    if name.is_empty() {
        return "physics";
    }
    if let Some(c) = ctx.project.skeleton.constraint(name) {
        return c.type_name();
    }
    let has = |n: &str| t.contains_key(n);
    if has("ik") {
        "ik"
    } else if has("transform") {
        "transform"
    } else if has("position") || has("spacing") || t.get("mix").is_some_and(|l| l.iter().any(|k| k.v.len() == 3)) {
        "path"
    } else {
        "physics"
    }
}

fn animation(ctx: &Ctx, a: &Animation, full: bool, warnings: &mut Vec<Warning>) -> JValue {
    let mut bones = O::new(full);
    for (b, t) in a.bones.iter() {
        bones.put(b, bone_timelines(t, full));
    }
    let mut slots = O::new(full);
    for (s, t) in a.slots.iter() {
        slots.put(s, slot_timelines(t, full));
    }
    let (mut ik, mut tr, mut path, mut phys) = (O::new(full), O::new(full), O::new(full), O::new(full));
    for (c, t) in a.constraints.iter() {
        if matches!(ctx.project.skeleton.constraint(c), Some(Constraint::Slider(_))) {
            warnings.push(Warning::new(
                "spine43.slider",
                format!("슬라이더 제약 '{c}'의 타임라인은 내보내지 않았습니다(GAPS G-11)."),
                format!("animations/{}/constraints/{c}", a.name),
            ));
            continue;
        }
        match kind_of(ctx, c, t) {
            "ik" => {
                let list = t.get("ik").map(Vec::as_slice).unwrap_or(&[]);
                ik.put(
                    c,
                    keys(list, full, |o, k| {
                        o.f("mix", at(k, 0, 1.0), 1.0).f("softness", at(k, 1, 0.0), 0.0);
                        o.b("bendPositive", at(k, 2, 1.0) >= 0.0, true);
                        o.b("compress", at(k, 3, 0.0) != 0.0, false).b("stretch", at(k, 4, 0.0) != 0.0, false);
                    }),
                );
            }
            "transform" => {
                let list = t.get("transform").map(Vec::as_slice).unwrap_or(&[]);
                tr.put(
                    c,
                    keys(list, full, |o, k| {
                        let (mx, msx) = (at(k, 1, 1.0), at(k, 3, 1.0));
                        o.f("mixRotate", at(k, 0, 1.0), 1.0).f("mixX", mx, 1.0).f("mixY", at(k, 2, mx), mx);
                        o.f("mixScaleX", msx, 1.0).f("mixScaleY", at(k, 4, msx), msx).f("mixShearY", at(k, 5, 1.0), 1.0);
                    }),
                );
            }
            "path" => {
                let mut o = O::new(full);
                for (n, list) in t.iter() {
                    let v = match n.as_str() {
                        "position" | "spacing" => keys(list, full, |o, k| {
                            o.f("value", at(k, 0, 0.0), 0.0);
                        }),
                        "mix" => keys(list, full, |o, k| {
                            let mx = at(k, 1, 1.0);
                            o.f("mixRotate", at(k, 0, 1.0), 1.0).f("mixX", mx, 1.0).f("mixY", at(k, 2, mx), mx);
                        }),
                        _ => continue,
                    };
                    o.put(n, v);
                }
                path.put(c, o.done());
            }
            _ => {
                let mut o = O::new(full);
                for (n, list) in t.iter() {
                    let v = match n.as_str() {
                        "reset" => keys(list, full, |_, _| {}),
                        "mix" => keys(list, full, |o, k| {
                            o.f("value", at(k, 0, 1.0), 1.0);
                        }),
                        n if timeline::PHYSICS.contains(&n) => keys(list, full, |o, k| {
                            o.f("value", at(k, 0, 0.0), 0.0);
                        }),
                        _ => continue,
                    };
                    o.put(n, v);
                }
                phys.put(c, o.done());
            }
        }
    }
    // 첨부물 타임라인: 스킨 → 슬롯 → 첨부물 → {deform, sequence}
    let mut atts: AttTree = vec![];
    for (skin, slots_) in a.deform.iter() {
        for (slot, m) in slots_.iter() {
            for (att, list) in m.iter() {
                let v = keys(list, full, |o, k| {
                    o.f("offset", k.offset as f32, 0.0).list("vertices", arr_f(&k.v));
                });
                slot_of(&mut atts, full, skin, slot, att).put("deform", v);
            }
        }
    }
    for (skin, slots_) in a.sequence.iter() {
        for (slot, m) in slots_.iter() {
            for (att, list) in m.iter() {
                let v = JValue::Arr(
                    list.iter()
                        .map(|k| {
                            let mut o = O::new(full);
                            o.f("time", k.time, 0.0).s("mode", k.s.as_deref().unwrap_or("hold"), "hold");
                            o.f("index", at(k, 0, 0.0), 0.0).f("delay", at(k, 1, 0.0), 0.0);
                            o.done()
                        })
                        .collect(),
                );
                slot_of(&mut atts, full, skin, slot, att).put("sequence", v);
            }
        }
    }
    let mut att_o = O::new(full);
    for (skin, sl) in atts {
        let mut so = O::new(full);
        for (slot, al) in sl {
            let mut ao = O::new(full);
            for (att, mut o) in al {
                ao.put(&att, o.done());
            }
            so.put(&slot, ao.done());
        }
        att_o.put(&skin, so.done());
    }
    let draw = JValue::Arr(
        a.draw_order
            .iter()
            .map(|k| {
                let mut o = O::new(full);
                o.f("time", k.time, 0.0);
                let offs = k
                    .offsets
                    .iter()
                    .map(|x| {
                        let mut oo = O::new(full);
                        oo.put("slot", JValue::str(&x.slot)).put("offset", JValue::from(x.offset as i64));
                        oo.done()
                    })
                    .collect();
                o.list("offsets", JValue::Arr(offs));
                o.done()
            })
            .collect(),
    );
    let events = JValue::Arr(
        a.events
            .iter()
            .map(|e| {
                let mut o = O::new(full);
                o.f("time", e.time, 0.0).put("name", JValue::str(&e.name));
                o.opt("int", e.int.as_ref()).opt("float", e.float.as_ref()).opt("string", e.string.as_ref());
                o.opt("volume", e.volume.as_ref()).opt("balance", e.balance.as_ref());
                o.done()
            })
            .collect(),
    );
    let mut o = O::new(full);
    o.list("bones", bones.done()).list("slots", slots.done());
    o.list("ik", ik.done()).list("transform", tr.done()).list("path", path.done()).list("physics", phys.done());
    o.list("attachments", att_o.done()).list("drawOrder", draw).list("events", events);
    o.done()
}

/// 스킨 → 슬롯 → 첨부물 → 타임라인 객체(순서 유지).
type AttTree = Vec<(String, Vec<(String, Vec<(String, O)>)>)>;

fn slot_of<'a>(atts: &'a mut AttTree, full: bool, skin: &str, slot: &str, att: &str) -> &'a mut O {
    let si = match atts.iter().position(|x| x.0 == skin) {
        Some(i) => i,
        None => {
            atts.push((skin.to_string(), vec![]));
            atts.len() - 1
        }
    };
    let sl = &mut atts[si].1;
    let li = match sl.iter().position(|x| x.0 == slot) {
        Some(i) => i,
        None => {
            sl.push((slot.to_string(), vec![]));
            sl.len() - 1
        }
    };
    let al = &mut sl[li].1;
    let ai = match al.iter().position(|x| x.0 == att) {
        Some(i) => i,
        None => {
            al.push((att.to_string(), O::new(full)));
            al.len() - 1
        }
    };
    &mut al[ai].1
}

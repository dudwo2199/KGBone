//! Spine JSON 가져오기·내보내기 테스트 (J04, J05, K03, C02~C04, D08, D13, G03).
use super::sample;
use crate::format::kgbone::KgboneFormat;
use crate::format::spine_json::{SpineExportOptions, SpineJsonFormat, SpineVersion};
use crate::format::{Original, SkeletonFormat};
use crate::model::{Attachment, BlendMode, Color, Constraint, Curve, Inherit};

const SAMPLES: [(&str, &[u8]); 10] = [
    ("bones-only", sample!("spine43/bones-only.json")),
    ("clipping", sample!("spine43/clipping.json")),
    ("events", sample!("spine43/events.json")),
    ("ik", sample!("spine43/ik.json")),
    ("mesh-weights", sample!("spine43/mesh-weights.json")),
    ("path", sample!("spine43/path.json")),
    ("physics", sample!("spine43/physics.json")),
    ("skins", sample!("spine43/skins.json")),
    ("transform", sample!("spine43/transform.json")),
    ("character-4.2", sample!("spine42/character-4.2.json")),
];

fn fmt(version: SpineVersion) -> SpineJsonFormat {
    SpineJsonFormat { options: SpineExportOptions { version, ..Default::default() } }
}

fn import(bytes: &[u8]) -> crate::format::ReadResult {
    SpineJsonFormat::default().read(bytes).unwrap()
}

fn version_of(name: &str) -> SpineVersion {
    if name.ends_with("4.2") {
        SpineVersion::V42
    } else {
        SpineVersion::V43
    }
}

#[test]
fn f_j04_reads_all_samples() {
    for (name, bytes) in SAMPLES {
        let r = SpineJsonFormat::default().read(bytes);
        assert!(r.is_ok(), "{name}: {:?}", r.err());
    }
    let r = import(sample!("spine42/character-4.2.json"));
    let sk = &r.project.skeleton;
    assert_eq!(sk.bones.len(), 8);
    assert_eq!(sk.slots.len(), 4);
    // order: foot(1) → arm-follow(2)
    assert_eq!(sk.constraints.iter().map(|c| c.name()).collect::<Vec<_>>(), ["foot", "arm-follow"]);
    let a = sk.animation("idle").unwrap();
    let rot = a.bones.get("torso").unwrap().get("rotate").unwrap();
    assert_eq!(rot[0].curve, Curve::Stepped);
    assert_eq!(rot[1].v, vec![5.0]);
    // scale 두 번째 키는 y가 없으므로 1.
    assert_eq!(a.bones.get("torso").unwrap().get("scale").unwrap()[1].v, vec![1.1, 1.0]);
    assert_eq!(a.bones.get("head").unwrap().get("inherit").unwrap()[0].s.as_deref(), Some("noRotationOrReflection"));
    let head = a.slots.get("head").unwrap();
    assert_eq!(head.get("rgba").unwrap()[1].v, Color::rgba(255, 0x80, 0x80, 255).to_f32().to_vec());
    assert_eq!(head.get("attachment").unwrap()[0].s, None);
    assert_eq!(a.draw_order[0].offsets[0].offset, -2);
    assert_eq!(r.project.images_path, "../images");
}

#[test]
fn f_j04_mapping_details() {
    let sk = import(sample!("spine43/bones-only.json")).project.skeleton;
    assert_eq!(sk.bone("neck").unwrap().inherit, Inherit::NoScale);
    assert_eq!(sk.bone("hip").unwrap().color, Color::rgba(255, 0, 0, 255));

    let sk = import(sample!("spine43/transform.json")).project.skeleton;
    let Constraint::Transform(t) = &sk.constraints[0] else { panic!() };
    // 오프셋 rotation 10, x 5 → 같은-속성 매핑 6개, mixY는 mixX(0.5)를 따른다.
    assert_eq!(t.source, "src");
    assert_eq!(t.properties.len(), 6);
    assert_eq!(t.properties[0].to[0].offset, 10.0);
    assert_eq!(t.properties[1].to[0].offset, 5.0);
    assert_eq!((t.mix_x, t.mix_y, t.mix_scale_x, t.mix_scale_y), (0.5, 0.5, 0.0, 0.0));
    let Constraint::Transform(t) = &sk.constraints[1] else { panic!() };
    assert!(t.local_source && t.local_target && t.additive && t.properties.is_empty());
    let tl = &sk.animations[0].constraints.get("tc-world").unwrap().get("transform").unwrap()[1];
    assert_eq!(tl.v, vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);

    let sk = import(sample!("spine43/ik.json")).project.skeleton;
    let k = &sk.animations[0].constraints.get("leg-ik").unwrap().get("ik").unwrap()[1];
    assert_eq!(k.v, vec![0.0, 5.0, -1.0, 0.0, 0.0]);

    let sk = import(sample!("spine43/physics.json")).project.skeleton;
    let Constraint::Physics(p) = &sk.constraints[0] else { panic!() };
    assert_eq!((p.rotate, p.x, p.strength, p.gravity), (1.0, 0.5, 120.0, 50.0));
    assert!(sk.animations[0].constraints.get("").unwrap().contains_key("reset"));

    let sk = import(sample!("spine43/mesh-weights.json")).project.skeleton;
    let Some(Attachment::Mesh(m)) = sk.skin("default").unwrap().get("tail", "tail") else { panic!() };
    assert!(m.weighted);
    assert_eq!(m.hull, 6);
    let d = &sk.animations[0].deform.get("default").unwrap().get("tail").unwrap().get("tail").unwrap();
    assert_eq!((d[1].offset, d[1].v.clone()), (2, vec![0.0, 4.0, 0.0, 4.0]));
    assert!(matches!(sk.animations[0].bones.get("tail2").unwrap().get("rotate").unwrap()[0].curve, Curve::Bezier(ref c) if c.len() == 4));

    let sk = import(sample!("spine43/skins.json")).project.skeleton;
    assert!(sk.bone("hat-bone").unwrap().skin_required);
    let hat = sk.slot("hat").unwrap();
    assert_eq!(hat.dark.unwrap().0, Color::rgba(0x20, 0x20, 0x20, 255));
    assert_eq!(sk.slot("glow").unwrap().blend, BlendMode::Additive);
    let Some(Attachment::Region(r)) = sk.skin("red").unwrap().get("hat", "hat") else { panic!() };
    assert_eq!(r.path.as_deref(), Some("hat-red"));

    let sk = import(sample!("spine43/clipping.json")).project.skeleton;
    let Some(Attachment::Clipping(c)) = sk.skin("default").unwrap().get("clip", "clip") else { panic!() };
    assert_eq!(c.end.as_deref(), Some("body"));

    let r = import(sample!("spine43/events.json"));
    assert!(r.warnings.iter().any(|w| w.code == "spine.unknownKey" && w.path == "x-custom-root"));
    let e = r.project.skeleton.event("footstep").unwrap();
    assert_eq!((e.int, e.string.as_str(), e.audio.as_str()), (1, "왼발", "step.wav"));
    let keys = &r.project.skeleton.animations[0].events;
    assert_eq!((keys[1].string.as_deref(), keys[1].int), (Some("오른발"), Some(2)));
}

#[test]
fn f_j05_export_right_after_import_preserves_everything() {
    for (name, bytes) in SAMPLES {
        let r = import(bytes);
        let orig = Original { bytes: bytes.to_vec(), raw: r.raw.clone(), project: r.project.clone() };
        let w = fmt(version_of(name)).write(&r.project, Some(&orig)).unwrap();
        let text = String::from_utf8(w.bytes).unwrap();
        // 편집 없이 내보내면 원본과 글자 단위로 같다(키 순서·숫자 표기·모르는 필드 모두 보존).
        assert_eq!(text, std::str::from_utf8(bytes).unwrap(), "{name}");
        assert_eq!(import(text.as_bytes()).project, r.project, "{name}");
    }
}

#[test]
fn f_j05_edit_then_export_keeps_unknown_and_notation() {
    let bytes = sample!("spine43/events.json");
    let r = import(bytes);
    let orig = Original { bytes: bytes.to_vec(), raw: r.raw.clone(), project: r.project.clone() };
    let mut p = r.project.clone();
    p.skeleton.bones[0].x = 12.0;
    p.skeleton.events[0].string = "왼쪽 발".into();
    let text = String::from_utf8(SpineJsonFormat::default().write(&p, Some(&orig)).unwrap().bytes).unwrap();
    for keep in ["\"x-custom\"", "\"x-custom-root\"", "2.50", "1.250", "\"spine\": \"4.3.00\"", "\"hash\": \"kgbone-sample\""] {
        assert!(text.contains(keep), "{keep}\n{text}");
    }
    assert!(text.contains("\"왼쪽 발\""));
    assert!(text.find("\"skeleton\"").unwrap() < text.find("\"bones\"").unwrap());
    assert!(text.find("\"events\"").unwrap() < text.find("\"animations\"").unwrap());
    assert_eq!(import(text.as_bytes()).project, p);
}

#[test]
fn f_j05_export_without_original_roundtrips() {
    for (name, bytes) in SAMPLES {
        let p = import(bytes).project;
        for pretty in [true, false] {
            let f = SpineJsonFormat { options: SpineExportOptions { pretty, ..Default::default() } };
            let text = f.write(&p, None).unwrap().bytes;
            let back = import(&text).project;
            assert_eq!(back.skeleton, p.skeleton, "{name} pretty={pretty}");
        }
    }
}

#[test]
fn version_string_follows_target() {
    let bytes = sample!("spine42/character-4.2.json");
    let r = import(bytes);
    let orig = Original { bytes: bytes.to_vec(), raw: r.raw.clone(), project: r.project.clone() };
    let t = String::from_utf8(fmt(SpineVersion::V43).write(&r.project, Some(&orig)).unwrap().bytes).unwrap();
    assert!(t.contains("\"spine\": \"4.3.00\""));
    let t = String::from_utf8(fmt(SpineVersion::V42).write(&r.project, None).unwrap().bytes).unwrap();
    assert!(t.contains("\"spine\": \"4.2.00\""));
}

#[test]
fn f_k03_unconfirmed_43_features_warn() {
    let p = KgboneFormat.read(sample!("kgbone/full-4.3.kgbone")).unwrap().project;
    let w = SpineJsonFormat::default().write(&p, None).unwrap();
    let codes: Vec<&str> = w.warnings.iter().map(|w| w.code.as_str()).collect();
    for c in ["spine43.slider", "spine43.volume", "spine43.transformMapping"] {
        assert!(codes.contains(&c), "{c}: {codes:?}");
    }
    let text = String::from_utf8(w.bytes).unwrap();
    assert!(!text.contains("\"name\": \"흔들기\"") && !text.contains("\"volume\": true"), "{text}");
    // 내보낸 것은 다시 읽힌다.
    let back = import(text.as_bytes()).project;
    assert_eq!(back.skeleton.constraints.len(), 3);
}

#[test]
fn f_c02_c04_slot_color_dark_blend_roundtrip() {
    let bytes = sample!("spine43/skins.json");
    let r = import(bytes);
    let orig = Original { bytes: bytes.to_vec(), raw: r.raw.clone(), project: r.project.clone() };
    let mut p = r.project.clone();
    let s = p.skeleton.slot_mut("hat").unwrap();
    s.color = Color::rgba(1, 2, 3, 4);
    s.dark = None;
    s.blend = BlendMode::Screen;
    let text = String::from_utf8(SpineJsonFormat::default().write(&p, Some(&orig)).unwrap().bytes).unwrap();
    assert!(!text.contains("\"dark\""), "끄면 dark 필드가 없어진다");
    assert!(text.contains("\"color\": \"01020304\""));
    assert_eq!(import(text.as_bytes()).project, p);
}

#[test]
fn f_d08_d13_linkedmesh_sequence_roundtrip() {
    use crate::model::{LinkedMeshAttachment, Sequence};
    let mut p = import(sample!("spine43/mesh-weights.json")).project;
    let skin = p.skeleton.default_skin_mut();
    skin.attachments.entry("tail").insert(
        "tail-copy",
        Attachment::LinkedMesh(LinkedMeshAttachment {
            path: Some("tail".into()),
            width: 128.0,
            height: 32.0,
            color: Color::WHITE,
            parent: "tail".into(),
            skin: None,
            timelines: false,
            sequence: Some(Sequence { count: 3, start: 0, digits: 2, setup: 1 }),
        }),
    );
    let text = SpineJsonFormat::default().write(&p, None).unwrap().bytes;
    assert_eq!(import(&text).project.skeleton, p.skeleton);
}

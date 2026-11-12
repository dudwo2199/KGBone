//! .kgbone 읽기·쓰기 테스트 (J01, J02, J03, J07, B06, B07, H01, F06, F07).
use super::sample;
use crate::document::Document;
use crate::format::kgbone::KgboneFormat;
use crate::format::{Original, SkeletonFormat};
use crate::model::{Color, Constraint, EventDef, Project};

const SAMPLES: [(&str, &[u8]); 4] = [
    ("bones-only", sample!("kgbone/bones-only.kgbone")),
    ("full-4.3", sample!("kgbone/full-4.3.kgbone")),
    ("legacy-v0", sample!("kgbone/legacy-v0.kgbone")),
    ("notation", sample!("kgbone/notation.kgbone")),
];

fn read(bytes: &[u8]) -> crate::format::ReadResult {
    KgboneFormat.read(bytes).unwrap()
}

fn original(bytes: &[u8]) -> Original {
    let r = read(bytes);
    Original { bytes: bytes.to_vec(), raw: r.raw, project: r.project }
}

#[test]
fn f_j01_open_save_reopen_same_model() {
    for (name, bytes) in SAMPLES {
        let (mut doc, _) = Document::open_kgbone(bytes).unwrap();
        let saved = doc.save_kgbone().unwrap();
        let again = read(&saved);
        assert_eq!(&again.project, doc.project(), "{name}");
        // 원본 없이(새 파일로) 써도 같은 모델로 다시 읽힌다.
        let fresh = KgboneFormat.write(doc.project(), None).unwrap();
        assert_eq!(&read(&fresh.bytes).project, doc.project(), "{name} (새 파일)");
    }
}

#[test]
fn f_j02_unedited_save_is_byte_identical() {
    for (name, bytes) in SAMPLES {
        if name == "legacy-v0" {
            continue; // 구버전은 변환되므로 바이트가 달라진다(J07).
        }
        let (mut doc, _) = Document::open_kgbone(bytes).unwrap();
        assert_eq!(doc.save_kgbone().unwrap(), bytes, "{name}");
        // 합치기 경로(원본 바이트를 그대로 돌려주는 지름길을 거치지 않음)로도 같아야 한다.
        let orig = original(bytes);
        let p = orig.project.clone();
        // 원본 모델을 다른 것으로 바꿔 "편집됨"으로 보이게 한다.
        let forced = Original { project: Project::new(), ..orig };
        let w = KgboneFormat.write(&p, Some(&forced)).unwrap();
        assert_eq!(String::from_utf8(w.bytes).unwrap(), std::str::from_utf8(bytes).unwrap(), "{name} (합치기 경로)");
    }
}

#[test]
fn f_j03_rename_changes_only_that_line() {
    let bytes = sample!("kgbone/notation.kgbone");
    let orig = original(bytes);
    let mut p = orig.project.clone();
    p.skeleton.bones[1].name = "child2".into();
    let out = String::from_utf8(KgboneFormat.write(&p, Some(&orig)).unwrap().bytes).unwrap();
    let src = std::str::from_utf8(bytes).unwrap();
    let a: Vec<&str> = src.lines().collect();
    let b: Vec<&str> = out.lines().collect();
    assert_eq!(a.len(), b.len());
    let changed: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
    assert_eq!(changed.len(), 1, "{out}");
    assert_eq!(b[changed[0]], "\t\t\t\t\"name\": \"child2\",");
    for keep in ["\"x\": 1.50", "\"rotation\": 10.0", "\"length\": 1e2", "\"x-future\": { \"keep\": true }"] {
        assert!(out.contains(keep), "{keep}");
    }
    // 키 순서(x가 name 앞) 그대로.
    assert!(out.find("\"x\": 1.50").unwrap() < out.find("\"name\": \"root\"").unwrap());
    assert_eq!(read(out.as_bytes()).project, p);
}

#[test]
fn f_j07_legacy_v0_migrates_to_v1() {
    let r = read(sample!("kgbone/legacy-v0.kgbone"));
    assert_eq!(r.project.format_version, 1);
    assert!(r.warnings.iter().any(|w| w.code == "kgbone.migrated"));
    // order: tc(0) → ik2(1)
    let names: Vec<&str> = r.project.skeleton.constraints.iter().map(|c| c.name()).collect();
    assert_eq!(names, ["tc", "ik2"]);
    assert!(matches!(r.project.skeleton.constraints[0], Constraint::Transform(ref t) if t.mix_rotate == 0.0 && t.source == "root"));
    assert!(r.raw.get("skeleton").unwrap().get("ik").is_none());
    // 열어서 저장하면 1 형식으로 쓰이고, 다시 열면 변환 경고가 없다.
    let (mut doc, _) = Document::open_kgbone(sample!("kgbone/legacy-v0.kgbone")).unwrap();
    let saved = doc.save_kgbone().unwrap();
    let text = std::str::from_utf8(&saved).unwrap();
    assert!(text.contains("\"formatVersion\": 1") && text.contains("\"constraints\"") && !text.contains("\"order\""), "{text}");
    let again = read(&saved);
    assert!(again.warnings.is_empty());
    assert_eq!(&again.project, doc.project());
}

#[test]
fn f_b06_bone_color_length_roundtrip() {
    let orig = original(sample!("kgbone/bones-only.kgbone"));
    let mut p = orig.project.clone();
    let b = p.skeleton.bone_mut("a").unwrap();
    b.color = Color::rgba(0x12, 0x34, 0x56, 0x78);
    b.length = 77.5;
    let out = KgboneFormat.write(&p, Some(&orig)).unwrap().bytes;
    let back = read(&out).project;
    assert_eq!(back.skeleton.bone("a").unwrap().color, Color::rgba(0x12, 0x34, 0x56, 0x78));
    assert_eq!(back.skeleton.bone("a").unwrap().length, 77.5);
    assert_eq!(back, p);
}

#[test]
fn f_b07_korean_names_roundtrip() {
    let orig = original(sample!("kgbone/full-4.3.kgbone"));
    let mut p = orig.project.clone();
    p.skeleton.bones[2].name = "꼬리_첫째".into();
    let out = KgboneFormat.write(&p, Some(&orig)).unwrap().bytes;
    assert_eq!(read(&out).project, p);
}

#[test]
fn f_h01_event_roundtrip_with_korean() {
    let orig = original(sample!("kgbone/full-4.3.kgbone"));
    let mut p = orig.project.clone();
    p.skeleton.events.push(EventDef { name: "외침".into(), int: 3, float: 1.5, string: "야호".into(), audio: "소리.wav".into(), volume: 0.5, balance: -1.0 });
    let out = KgboneFormat.write(&p, Some(&orig)).unwrap().bytes;
    assert_eq!(read(&out).project, p);
    assert_eq!(read(&KgboneFormat.write(&p, None).unwrap().bytes).project, p);
}

#[test]
fn f_f06_f07_slider_and_order_roundtrip() {
    let orig = original(sample!("kgbone/full-4.3.kgbone"));
    let mut p = orig.project.clone();
    assert!(p.skeleton.constraints.iter().any(|c| matches!(c, Constraint::Slider(_))));
    p.skeleton.constraints.reverse();
    let out = KgboneFormat.write(&p, Some(&orig)).unwrap().bytes;
    assert_eq!(read(&out).project, p);
}

#[test]
fn deleted_timeline_is_removed_not_preserved() {
    // 사전(애니메이션 타임라인)에서 지운 항목은 모르는 키로 남지 않아야 한다.
    let orig = original(sample!("kgbone/full-4.3.kgbone"));
    let mut p = orig.project.clone();
    p.skeleton.animations[0].deform = Default::default();
    let out = KgboneFormat.write(&p, Some(&orig)).unwrap().bytes;
    assert_eq!(read(&out).project, p);
}

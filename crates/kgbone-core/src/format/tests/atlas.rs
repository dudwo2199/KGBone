//! .atlas 읽기·쓰기 테스트 (J06).
use super::sample;
use crate::format::atlas::{parse, write};

#[test]
fn f_j06_sample_atlas_roundtrip() {
    let text = std::str::from_utf8(sample!("spine42/character-4.2.atlas")).unwrap();
    let a = parse(text).unwrap();
    assert_eq!(a.pages.len(), 1);
    let p = &a.pages[0];
    assert_eq!((p.name.as_str(), p.width, p.height, p.pma), ("character-4.2.png", 256, 256, true));
    assert_eq!(p.filter, Some(("Linear".into(), "Linear".into())));
    assert_eq!(p.regions.len(), 4);
    // leg: bounds 2,24,24,72 + rotate:90 → 원래 방향 72×24
    let leg = p.regions.iter().find(|r| r.name == "leg").unwrap();
    assert_eq!((leg.x, leg.y, leg.width, leg.height, leg.degrees), (2, 24, 72, 24, 90));
    assert_eq!((leg.original_width, leg.original_height), (72, 24));
    let head = p.regions.iter().find(|r| r.name == "head").unwrap();
    assert_eq!((head.original_width, head.original_height, head.index), (64, 64, -1));
    let out = write(&a);
    assert!(out.contains("leg\nbounds:2,24,24,72\nrotate:90\n"), "{out}");
    assert_eq!(parse(&out).unwrap(), a);
}

#[test]
fn f_j06_old_notation_and_extras() {
    let text = "\npage1.png\nsize: 128,64\nformat: RGBA8888\nfilter: Nearest,Linear\nrepeat: none\nmystery: 7\na\n  rotate: true\n  xy: 1, 2\n  size: 10, 20\n  orig: 12, 22\n  offset: 1, 1\n  index: 3\n  custom: q\n\npage2.png\nsize:8,8\nb\nbounds:0,0,8,8\nsplit:1,2,3,4\npad:0,0,1,1\n";
    let a = parse(text).unwrap();
    assert_eq!(a.pages.len(), 2);
    let p = &a.pages[0];
    assert_eq!(p.format.as_deref(), Some("RGBA8888"));
    assert_eq!(p.extra, vec![("mystery".to_string(), "7".to_string())]);
    let r = &p.regions[0];
    assert_eq!((r.x, r.y, r.width, r.height, r.degrees, r.index), (1, 2, 20, 10, 90, 3));
    assert_eq!((r.offset_x, r.offset_y, r.original_width, r.original_height), (1, 1, 12, 22));
    assert_eq!(r.extra, vec![("custom".to_string(), "q".to_string())]);
    assert_eq!(a.pages[1].regions[0].split, Some([1, 2, 3, 4]));
    assert_eq!(parse(&write(&a)).unwrap(), a);
}
